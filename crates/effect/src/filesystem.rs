use crate::{
    AdapterReceipt, ApplyReceipt, ArtifactId, ArtifactRole, ArtifactState, CleanupReceipt,
    EffectMutation, EffectOperation, EffectOperationId, EffectOperationIntent, EffectResource,
    EffectResourceId, ExactPlannedState, ExactPreviousState, FilesystemOperationPlan, Fingerprint,
    LocalTimestamp, ManagedIdentity, NativeResourceIdentity, OperationArtifact, PlannedMutation,
    PreparedOperation, ReconcileAttemptId, ResourceAdapter, ResourceAdapterError,
    ResourceObservation, ResourceOperationPreparer, VerificationReceipt, VersionedAdapterPlan,
    VersionedMaterializationInput, VersionedResourceDescriptor, VersionedResourceLocator,
};
use ora_skill_package::{Limits, parse_manifest};
use ora_utils::directory::{
    DirectoryFingerprint, DirectoryTreeError, copy_directory, fingerprint_directory,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use thiserror::Error;

pub const MARKER_FILE_NAME: &str = ".ora-managed.json";
const OPERATIONS_DIR_NAME: &str = ".ora-effect-operations";
const MARKER_SCHEMA_VERSION: u32 = 1;

/// On-disk ownership claim that is useful only when matched with the Resource ledger.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ManagedItemMarker {
    pub schema_version: u32,
    pub resource: EffectResourceId,
    pub managed_identity: ManagedIdentity,
}

impl ManagedItemMarker {
    /// Creates the current marker schema for a newly staged Managed Item.
    pub fn current(resource: EffectResourceId, managed_identity: ManagedIdentity) -> Self {
        Self {
            schema_version: MARKER_SCHEMA_VERSION,
            resource,
            managed_identity,
        }
    }
}

/// Local filesystem implementation of the versioned directory Resource contract.
#[derive(Clone, Copy, Debug, Default)]
pub struct FilesystemResourceAdapter;

impl FilesystemResourceAdapter {
    /// Computes the exact content fingerprint used by immutable Skill definitions and observations.
    pub fn package_fingerprint(path: &Path) -> Result<Fingerprint, FilesystemEffectError> {
        fingerprint(path)
    }

    /// Converts a pure mutation proposal into immutable adapter intent and artifact authority.
    pub fn prepare_operation(
        &self,
        resource: &EffectResource,
        attempt: ReconcileAttemptId,
        generation: crate::Generation,
        sequence: u32,
        mutation: PlannedMutation,
        prepared_at: LocalTimestamp,
    ) -> Result<PreparedOperation, FilesystemEffectError> {
        let resource_root = resource_root(resource)?;
        let VersionedResourceDescriptor::FilesystemDirectoryV1(descriptor) = &resource.descriptor;
        let operation_id = EffectOperationId::random();
        let operation_root = resource_root
            .join(OPERATIONS_DIR_NAME)
            .join(operation_id.as_str());
        let staging_path = operation_root.join("staging");
        let backup_path = operation_root.join("backup");
        let source_root = mutation.input.as_ref().map(
            |VersionedMaterializationInput::SkillDirectoryV1(input)| input.package_root.clone(),
        );
        let payload = VersionedAdapterPlan::FilesystemDirectoryV1(FilesystemOperationPlan {
            workspace_root: descriptor.workspace_root.clone(),
            resource_relative_path: descriptor.relative_path.clone(),
            resource_root,
            source_root,
            staging_path: staging_path.clone(),
            backup_path: backup_path.clone(),
        });
        let mut artifacts = Vec::new();
        if let ExactPlannedState::Present { fingerprint, .. } = &mutation.planned {
            artifacts.push(OperationArtifact {
                identity: ArtifactId::random(),
                operation: operation_id.clone(),
                role: ArtifactRole::Staging,
                locator: VersionedResourceLocator::FilesystemPathV1(staging_path),
                expected_fingerprint: fingerprint.clone(),
                state: ArtifactState::Reserved,
            });
        }
        if let ExactPreviousState::Present { fingerprint, .. } = &mutation.expected {
            artifacts.push(OperationArtifact {
                identity: ArtifactId::random(),
                operation: operation_id.clone(),
                role: ArtifactRole::Backup,
                locator: VersionedResourceLocator::FilesystemPathV1(backup_path),
                expected_fingerprint: fingerprint.clone(),
                state: ArtifactState::Reserved,
            });
        }
        Ok(PreparedOperation {
            operation: EffectOperation::prepare(
                operation_id,
                EffectOperationIntent {
                    attempt,
                    resource: resource.identity.clone(),
                    generation,
                    sequence,
                    mutation: mutation.mutation,
                    expected: mutation.expected,
                    planned: mutation.planned,
                    payload,
                },
                prepared_at,
            )?,
            artifacts,
        })
    }

    /// Observes a filesystem directory without creating a missing Resource as a read side effect.
    fn observe_resource(
        self,
        resource: &EffectResource,
    ) -> Result<ResourceObservation, FilesystemEffectError> {
        let root = resolve_resource_root(resource, RootAccess::Observe)?;
        let Some(root) = root else {
            return Ok(ResourceObservation {
                resource: resource.identity.clone(),
                items: BTreeMap::new(),
                fingerprint: Fingerprint::sha256(&[]),
            });
        };
        let mut items = BTreeMap::new();
        for entry in fs::read_dir(&root).map_err(|source| FilesystemEffectError::Io {
            path: root.clone(),
            source,
        })? {
            let entry = entry.map_err(|source| FilesystemEffectError::Io {
                path: root.clone(),
                source,
            })?;
            let entry_name = entry.file_name().to_string_lossy().into_owned();
            if entry_name == OPERATIONS_DIR_NAME {
                continue;
            }
            let native_identity = NativeResourceIdentity::parse(entry_name.clone())
                .map_err(|_| FilesystemEffectError::InvalidNativeIdentity(entry_name))?;
            let path = entry.path();
            let metadata =
                fs::symlink_metadata(&path).map_err(|source| FilesystemEffectError::Io {
                    path: path.clone(),
                    source,
                })?;
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(FilesystemEffectError::UnsupportedResourceEntry { path });
            }
            let fingerprint = fingerprint(&path)?;
            let ownership_evidence = read_marker(&path)
                .filter(|marker| {
                    marker.schema_version == MARKER_SCHEMA_VERSION
                        && marker.resource == resource.identity
                })
                .map_or(crate::OwnershipEvidence::NoOwnershipEvidence, |marker| {
                    crate::OwnershipEvidence::Claims(marker.managed_identity)
                });
            items.insert(
                native_identity.clone(),
                crate::ObservedItem {
                    native_identity,
                    fingerprint,
                    ownership_evidence,
                },
            );
        }
        let summary = serde_json::to_vec(&items).map_err(FilesystemEffectError::MarkerJson)?;
        Ok(ResourceObservation {
            resource: resource.identity.clone(),
            items,
            fingerprint: Fingerprint::sha256(&summary),
        })
    }

    /// Applies a journal only when disk equals its exact expected state or already-planned state.
    fn apply_operation(
        self,
        operation: &EffectOperation,
    ) -> Result<ApplyReceipt, FilesystemEffectError> {
        let VersionedAdapterPlan::FilesystemDirectoryV1(plan) = operation.payload();
        ensure_operation_paths_are_scoped(plan)?;
        let resolved_root = resolve_declared_root(
            &plan.workspace_root,
            &plan.resource_relative_path,
            RootAccess::Mutate,
        )?
        .ok_or(FilesystemEffectError::UnsafeOperationPath)?;
        if resolved_root != plan.resource_root {
            return Err(FilesystemEffectError::UnsafeOperationPath);
        }
        if state_matches_planned(operation, plan)? {
            return Ok(apply_receipt(operation));
        }
        if !state_matches_expected(operation, plan)? {
            return Err(FilesystemEffectError::RecoveryRequired {
                operation: operation.identity().clone(),
            });
        }

        match operation.mutation() {
            EffectMutation::Create => {
                stage(operation, plan)?;
                let target = planned_path(operation, plan)?;
                if target.exists() {
                    return Err(FilesystemEffectError::TargetOccupied { path: target });
                }
                fs::rename(&plan.staging_path, &target).map_err(|source| {
                    FilesystemEffectError::Io {
                        path: target,
                        source,
                    }
                })?;
            }
            EffectMutation::Update | EffectMutation::Replace => {
                stage(operation, plan)?;
                let previous = expected_path(operation, plan)?;
                let target = planned_path(operation, plan)?;
                fs::create_dir_all(
                    plan.backup_path
                        .parent()
                        .ok_or(FilesystemEffectError::UnsafeOperationPath)?,
                )
                .map_err(|source| FilesystemEffectError::Io {
                    path: plan.backup_path.clone(),
                    source,
                })?;
                fs::rename(&previous, &plan.backup_path).map_err(|source| {
                    FilesystemEffectError::Io {
                        path: previous.clone(),
                        source,
                    }
                })?;
                if target != previous && target.exists() {
                    restore_backup(&plan.backup_path, &previous);
                    return Err(FilesystemEffectError::TargetOccupied { path: target });
                }
                if let Err(source) = fs::rename(&plan.staging_path, &target) {
                    restore_backup(&plan.backup_path, &previous);
                    return Err(FilesystemEffectError::Io {
                        path: target,
                        source,
                    });
                }
            }
            EffectMutation::Delete => {
                let previous = expected_path(operation, plan)?;
                fs::create_dir_all(
                    plan.backup_path
                        .parent()
                        .ok_or(FilesystemEffectError::UnsafeOperationPath)?,
                )
                .map_err(|source| FilesystemEffectError::Io {
                    path: plan.backup_path.clone(),
                    source,
                })?;
                fs::rename(&previous, &plan.backup_path).map_err(|source| {
                    FilesystemEffectError::Io {
                        path: previous,
                        source,
                    }
                })?;
            }
        }
        Ok(apply_receipt(operation))
    }

    /// Verifies exact planned state and never treats a merely similar directory as completion.
    fn verify_operation(
        self,
        operation: &EffectOperation,
    ) -> Result<VerificationReceipt, FilesystemEffectError> {
        let VersionedAdapterPlan::FilesystemDirectoryV1(plan) = operation.payload();
        if !state_matches_planned(operation, plan)? {
            return Err(FilesystemEffectError::VerificationFailed {
                operation: operation.identity().clone(),
            });
        }
        Ok(VerificationReceipt {
            operation: operation.identity().clone(),
            proof: AdapterReceipt {
                version: 1,
                payload: json!({ "state": "planned" }),
            },
        })
    }

    /// Cleans only the exact path/fingerprint pair granted by durable Artifact authority.
    fn cleanup_artifact(
        self,
        artifact: &OperationArtifact,
    ) -> Result<CleanupReceipt, FilesystemEffectError> {
        let VersionedResourceLocator::FilesystemPathV1(path) = &artifact.locator;
        if path.exists() {
            if fingerprint(path)? != artifact.expected_fingerprint {
                return Err(FilesystemEffectError::ArtifactFingerprintMismatch {
                    artifact: artifact.identity.clone(),
                });
            }
            fs::remove_dir_all(path).map_err(|source| FilesystemEffectError::Io {
                path: path.clone(),
                source,
            })?;
        }
        Ok(CleanupReceipt {
            artifact: artifact.identity.clone(),
            proof: AdapterReceipt {
                version: 1,
                payload: json!({ "state": "absent" }),
            },
        })
    }
}

impl ResourceAdapter for FilesystemResourceAdapter {
    fn observe(
        &self,
        resource: &EffectResource,
    ) -> Result<ResourceObservation, ResourceAdapterError> {
        (*self)
            .observe_resource(resource)
            .map_err(ResourceAdapterError::new)
    }

    fn apply(&self, operation: &EffectOperation) -> Result<ApplyReceipt, ResourceAdapterError> {
        (*self)
            .apply_operation(operation)
            .map_err(ResourceAdapterError::new)
    }

    fn verify(
        &self,
        operation: &EffectOperation,
    ) -> Result<VerificationReceipt, ResourceAdapterError> {
        (*self)
            .verify_operation(operation)
            .map_err(ResourceAdapterError::new)
    }

    fn cleanup(
        &self,
        artifact: &OperationArtifact,
    ) -> Result<CleanupReceipt, ResourceAdapterError> {
        (*self)
            .cleanup_artifact(artifact)
            .map_err(ResourceAdapterError::new)
    }
}

impl ResourceOperationPreparer for FilesystemResourceAdapter {
    fn prepare_operation(
        &self,
        resource: &EffectResource,
        attempt: ReconcileAttemptId,
        generation: crate::Generation,
        sequence: u32,
        mutation: PlannedMutation,
        prepared_at: LocalTimestamp,
    ) -> Result<PreparedOperation, ResourceAdapterError> {
        FilesystemResourceAdapter::prepare_operation(
            self,
            resource,
            attempt,
            generation,
            sequence,
            mutation,
            prepared_at,
        )
        .map_err(ResourceAdapterError::new)
    }
}

/// Selects whether resolving a Resource path may create missing safe directories.
#[derive(Clone, Copy)]
enum RootAccess {
    Observe,
    Prepare,
    Mutate,
}

/// Resolves the typed filesystem descriptor and checks the root stays inside its Workspace.
fn resolve_resource_root(
    resource: &EffectResource,
    access: RootAccess,
) -> Result<Option<PathBuf>, FilesystemEffectError> {
    let VersionedResourceDescriptor::FilesystemDirectoryV1(descriptor) = &resource.descriptor;
    resolve_declared_root(
        &descriptor.workspace_root,
        &descriptor.relative_path,
        access,
    )
}

/// Resolves a filesystem descriptor while refusing links and optionally creating safe segments.
fn resolve_declared_root(
    workspace_root: &Path,
    relative_path: &crate::ResourcePath,
    access: RootAccess,
) -> Result<Option<PathBuf>, FilesystemEffectError> {
    let root_metadata = fs::symlink_metadata(workspace_root).map_err(|source| {
        FilesystemEffectError::WorkspaceUnavailable {
            path: workspace_root.to_path_buf(),
            source,
        }
    })?;
    if !root_metadata.is_dir() || root_metadata.file_type().is_symlink() {
        return Err(FilesystemEffectError::UnsafeResourcePath {
            path: workspace_root.to_path_buf(),
        });
    }
    let canonical_workspace = workspace_root.canonicalize().map_err(|source| {
        FilesystemEffectError::WorkspaceUnavailable {
            path: workspace_root.to_path_buf(),
            source,
        }
    })?;
    let mut current = canonical_workspace.clone();
    let mut path_is_missing = false;
    for component in relative_path.to_path_buf().components() {
        let std::path::Component::Normal(segment) = component else {
            return Err(FilesystemEffectError::UnsafeResourcePath { path: current });
        };
        current = current.join(segment);
        if path_is_missing {
            continue;
        }
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                return Err(FilesystemEffectError::UnsafeResourcePath { path: current });
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => match access {
                RootAccess::Observe => return Ok(None),
                RootAccess::Prepare => path_is_missing = true,
                RootAccess::Mutate => {
                    fs::create_dir(&current).map_err(|source| FilesystemEffectError::Io {
                        path: current.clone(),
                        source,
                    })?;
                }
            },
            Err(source) => {
                return Err(FilesystemEffectError::Io {
                    path: current,
                    source,
                });
            }
        }
        if path_is_missing {
            continue;
        }
        let canonical = current
            .canonicalize()
            .map_err(|source| FilesystemEffectError::Io {
                path: current.clone(),
                source,
            })?;
        if !canonical.starts_with(&canonical_workspace) {
            return Err(FilesystemEffectError::UnsafeResourcePath { path: current });
        }
        current = canonical;
    }
    Ok(Some(current))
}

/// Resolves the Resource root for intent preparation without creating it yet.
fn resource_root(resource: &EffectResource) -> Result<PathBuf, FilesystemEffectError> {
    let VersionedResourceDescriptor::FilesystemDirectoryV1(descriptor) = &resource.descriptor;
    resolve_declared_root(
        &descriptor.workspace_root,
        &descriptor.relative_path,
        RootAccess::Prepare,
    )?
    .ok_or(FilesystemEffectError::UnsafeOperationPath)
}

/// Prevents a journal payload from redirecting artifacts outside its Resource directory.
fn ensure_operation_paths_are_scoped(
    plan: &FilesystemOperationPlan,
) -> Result<(), FilesystemEffectError> {
    let Some(staging_parent) = plan.staging_path.parent() else {
        return Err(FilesystemEffectError::UnsafeOperationPath);
    };
    if !staging_parent.starts_with(&plan.resource_root)
        || !plan.backup_path.starts_with(staging_parent)
    {
        return Err(FilesystemEffectError::UnsafeOperationPath);
    }
    Ok(())
}

/// Stages and validates immutable source content before an atomic swap.
fn stage(
    operation: &EffectOperation,
    plan: &FilesystemOperationPlan,
) -> Result<(), FilesystemEffectError> {
    let source_root = plan
        .source_root
        .as_ref()
        .ok_or(FilesystemEffectError::MissingMaterializationInput)?;
    if plan.staging_path.exists() {
        if !state_matches_path(
            operation.planned(),
            &plan.staging_path,
            operation.resource(),
        )? {
            return Err(FilesystemEffectError::StagingMismatch {
                path: plan.staging_path.clone(),
            });
        }
        return Ok(());
    }
    let operation_root = plan
        .staging_path
        .parent()
        .ok_or(FilesystemEffectError::UnsafeOperationPath)?;
    fs::create_dir_all(operation_root).map_err(|source| FilesystemEffectError::Io {
        path: operation_root.to_path_buf(),
        source,
    })?;
    copy_directory(
        source_root,
        &plan.staging_path,
        &[OsStr::new(MARKER_FILE_NAME)],
    )?;
    validate_staged_skill(operation, &plan.staging_path)?;
    let managed_identity = match operation.planned() {
        ExactPlannedState::Present {
            managed_identity, ..
        } => managed_identity.clone(),
        ExactPlannedState::Missing => return Err(FilesystemEffectError::MissingPlannedItem),
    };
    let marker = ManagedItemMarker::current(operation.resource().clone(), managed_identity);
    let marker_bytes = serde_json::to_vec(&marker).map_err(FilesystemEffectError::MarkerJson)?;
    let marker_path = plan.staging_path.join(MARKER_FILE_NAME);
    fs::write(&marker_path, marker_bytes).map_err(|source| FilesystemEffectError::Io {
        path: marker_path,
        source,
    })?;
    if !state_matches_path(
        operation.planned(),
        &plan.staging_path,
        operation.resource(),
    )? {
        return Err(FilesystemEffectError::StagingMismatch {
            path: plan.staging_path.clone(),
        });
    }
    Ok(())
}

/// Revalidates the staged Skill manifest and exact package fingerprint after copying.
fn validate_staged_skill(
    operation: &EffectOperation,
    staging: &Path,
) -> Result<(), FilesystemEffectError> {
    let VersionedAdapterPlan::FilesystemDirectoryV1(plan) = operation.payload();
    let source_root = plan
        .source_root
        .as_ref()
        .ok_or(FilesystemEffectError::MissingMaterializationInput)?;
    let source_manifest =
        fs::read(source_root.join("SKILL.md")).map_err(|source| FilesystemEffectError::Io {
            path: source_root.join("SKILL.md"),
            source,
        })?;
    let staged_manifest =
        fs::read(staging.join("SKILL.md")).map_err(|source| FilesystemEffectError::Io {
            path: staging.join("SKILL.md"),
            source,
        })?;
    let parsed = parse_manifest(&staged_manifest, Limits::default().max_manifest_bytes)
        .map_err(|_| FilesystemEffectError::InvalidSkillManifest)?;
    if source_manifest != staged_manifest {
        return Err(FilesystemEffectError::SourceChanged);
    }
    let planned_name = match operation.planned() {
        ExactPlannedState::Present {
            native_identity, ..
        } => native_identity,
        ExactPlannedState::Missing => return Err(FilesystemEffectError::MissingPlannedItem),
    };
    if !parsed.name.eq_ignore_ascii_case(planned_name.as_str()) {
        return Err(FilesystemEffectError::ManifestNameMismatch);
    }
    Ok(())
}

/// Tests the exact expected state at the locator implied by both operation states.
fn state_matches_expected(
    operation: &EffectOperation,
    plan: &FilesystemOperationPlan,
) -> Result<bool, FilesystemEffectError> {
    match operation.expected() {
        ExactPreviousState::Missing => {
            let path = planned_path(operation, plan)?;
            Ok(!path.exists())
        }
        ExactPreviousState::Present { .. } => state_matches_path(
            operation.expected(),
            &expected_path(operation, plan)?,
            operation.resource(),
        ),
    }
}

/// Tests the exact planned state at the locator implied by both operation states.
fn state_matches_planned(
    operation: &EffectOperation,
    plan: &FilesystemOperationPlan,
) -> Result<bool, FilesystemEffectError> {
    match operation.planned() {
        ExactPlannedState::Missing => {
            let path = expected_path(operation, plan)?;
            Ok(!path.exists())
        }
        ExactPlannedState::Present { .. } => state_matches_path(
            operation.planned(),
            &planned_path(operation, plan)?,
            operation.resource(),
        ),
    }
}

/// Compares fingerprint and marker proof for either exact present-state enum.
fn state_matches_path(
    state: &impl PresentState,
    path: &Path,
    resource: &EffectResourceId,
) -> Result<bool, FilesystemEffectError> {
    let Some((fingerprint_expected, managed_identity)) = state.present() else {
        return Ok(!path.exists());
    };
    if !path.exists() || fingerprint(path)? != *fingerprint_expected {
        return Ok(false);
    }
    Ok(read_marker(path).is_some_and(|marker| {
        marker.schema_version == MARKER_SCHEMA_VERSION
            && marker.resource == *resource
            && marker.managed_identity == *managed_identity
    }))
}

/// Supplies shared present-state access without collapsing the distinct expected/planned types.
trait PresentState {
    fn present(&self) -> Option<(&Fingerprint, &ManagedIdentity)>;
}

impl PresentState for ExactPreviousState {
    fn present(&self) -> Option<(&Fingerprint, &ManagedIdentity)> {
        match self {
            Self::Missing => None,
            Self::Present {
                fingerprint,
                managed_identity,
                ..
            } => Some((fingerprint, managed_identity)),
        }
    }
}

impl PresentState for ExactPlannedState {
    fn present(&self) -> Option<(&Fingerprint, &ManagedIdentity)> {
        match self {
            Self::Missing => None,
            Self::Present {
                fingerprint,
                managed_identity,
                ..
            } => Some((fingerprint, managed_identity)),
        }
    }
}

/// Resolves the expected native item path using a typed identity and Path::join.
fn expected_path(
    operation: &EffectOperation,
    plan: &FilesystemOperationPlan,
) -> Result<PathBuf, FilesystemEffectError> {
    match operation.expected() {
        ExactPreviousState::Present {
            native_identity, ..
        } => Ok(plan.resource_root.join(native_identity.as_str())),
        ExactPreviousState::Missing => Err(FilesystemEffectError::MissingExpectedItem),
    }
}

/// Resolves the planned native item path using a typed identity and Path::join.
fn planned_path(
    operation: &EffectOperation,
    plan: &FilesystemOperationPlan,
) -> Result<PathBuf, FilesystemEffectError> {
    match operation.planned() {
        ExactPlannedState::Present {
            native_identity, ..
        } => Ok(plan.resource_root.join(native_identity.as_str())),
        ExactPlannedState::Missing => Err(FilesystemEffectError::MissingPlannedItem),
    }
}

/// Reads a marker as untrusted evidence; malformed or absent markers establish no ownership.
fn read_marker(path: &Path) -> Option<ManagedItemMarker> {
    fs::read(path.join(MARKER_FILE_NAME))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
}

/// Produces a versioned idempotence receipt without exposing filesystem paths.
fn apply_receipt(operation: &EffectOperation) -> ApplyReceipt {
    ApplyReceipt {
        operation: operation.identity().clone(),
        proof: AdapterReceipt {
            version: 1,
            payload: json!({ "state": "applied_or_already_planned" }),
        },
    }
}

/// Converts the generic directory fingerprint into Effect's distinct observed-state type.
fn fingerprint(path: &Path) -> Result<Fingerprint, FilesystemEffectError> {
    let fingerprint: DirectoryFingerprint =
        fingerprint_directory(path, &[OsStr::new(MARKER_FILE_NAME)])?;
    Fingerprint::parse(fingerprint.as_str().to_string())
        .map_err(|_| FilesystemEffectError::InvalidDirectoryFingerprint)
}

/// Restores the previous tree when a swap cannot install its staging directory.
fn restore_backup(backup: &Path, previous: &Path) {
    if backup.exists() && !previous.exists() {
        let _ = fs::rename(backup, previous);
    }
}

/// Reports filesystem validation, observation, mutation, and recovery failures.
#[derive(Debug, Error)]
pub enum FilesystemEffectError {
    #[error(transparent)]
    Operation(#[from] crate::OperationTransitionError),
    #[error("Workspace root is unavailable: {path:?}")]
    WorkspaceUnavailable {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("unsafe Effect Resource path: {path:?}")]
    UnsafeResourcePath { path: PathBuf },
    #[error("unsupported entry in Skill directory Resource: {path:?}")]
    UnsupportedResourceEntry { path: PathBuf },
    #[error("invalid native Resource identity: {0}")]
    InvalidNativeIdentity(String),
    #[error("invalid Skill manifest")]
    InvalidSkillManifest,
    #[error("Skill manifest name does not match its native identity")]
    ManifestNameMismatch,
    #[error("source content changed while staging")]
    SourceChanged,
    #[error("materialization operation is missing source input")]
    MissingMaterializationInput,
    #[error("operation expected state does not name an item")]
    MissingExpectedItem,
    #[error("operation planned state does not name an item")]
    MissingPlannedItem,
    #[error("operation artifact paths are outside the Resource")]
    UnsafeOperationPath,
    #[error("operation staging state does not match durable intent: {path:?}")]
    StagingMismatch { path: PathBuf },
    #[error("target is occupied: {path:?}")]
    TargetOccupied { path: PathBuf },
    #[error("operation {operation} requires manual recovery")]
    RecoveryRequired { operation: EffectOperationId },
    #[error("operation {operation} did not reach exact planned state")]
    VerificationFailed { operation: EffectOperationId },
    #[error("artifact {artifact} no longer matches its cleanup authority")]
    ArtifactFingerprintMismatch { artifact: ArtifactId },
    #[error("invalid directory fingerprint")]
    InvalidDirectoryFingerprint,
    #[error("Effect Resource filesystem operation failed: {path:?}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error(transparent)]
    DirectoryTree(#[from] DirectoryTreeError),
    #[error("ownership marker serialization failed")]
    MarkerJson(#[source] serde_json::Error),
}
