use super::*;
use pretty_assertions::assert_eq;
use std::path::PathBuf;

#[test]
fn target_watermarks_reject_unproven_readiness() {
    assert_eq!(
        TargetProgress::restore(
            Generation::new(3),
            Generation::new(2),
            Generation::new(1),
            Generation::new(2),
        ),
        Err(StatusTransitionError::InvalidTargetWatermarks)
    );
}

#[test]
fn operation_intent_rejects_a_mutation_label_that_disagrees_with_exact_states() {
    let native_identity = NativeResourceIdentity::parse("skill")
        .unwrap_or_else(|error| panic!("native identity: {error}"));
    let managed_identity = ManagedIdentity::new("managed-1");
    let result = EffectOperation::prepare(
        EffectOperationId::new("operation-1"),
        EffectOperationIntent {
            attempt: ReconcileAttemptId::new("attempt-1"),
            resource: EffectResourceId::new("resource-1"),
            generation: Generation::new(1),
            sequence: 0,
            mutation: EffectMutation::Delete,
            expected: ExactPreviousState::Present {
                native_identity: native_identity.clone(),
                fingerprint: Fingerprint::sha256(b"previous"),
                managed_identity: managed_identity.clone(),
            },
            planned: ExactPlannedState::Present {
                native_identity,
                fingerprint: Fingerprint::sha256(b"planned"),
                managed_identity,
            },
            payload: VersionedAdapterPlan::FilesystemDirectoryV1(FilesystemOperationPlan {
                workspace_root: PathBuf::from("/workspace"),
                resource_relative_path: ResourcePath::parse(".agents/skills")
                    .unwrap_or_else(|error| panic!("resource path: {error}")),
                resource_root: PathBuf::from("/workspace/.agents/skills"),
                source_root: None,
                staging_path: PathBuf::from("/workspace/.agents/staging"),
                backup_path: PathBuf::from("/workspace/.agents/backup"),
            }),
        },
        LocalTimestamp::from_millis(1),
    );

    assert_eq!(result, Err(OperationTransitionError::InvalidMutationStates));
}
