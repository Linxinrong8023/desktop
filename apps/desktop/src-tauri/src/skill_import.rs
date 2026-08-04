use crate::commands::run_backend;
use crate::error::CommandError;
use crate::state::DesktopState;
use ora_application::{
    ApplicationError, MAX_SKILL_FILES, MAX_SKILL_UPLOAD_BYTES, UploadedSkillFile,
};
use ora_backend::{Backend, BackendError, ErrorClassification};
use ora_contracts::{CreateSkillResponse, EmptyErrorParams, PublicError};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::State;
use thiserror::Error;

/// Carries the native directory selected for one Desktop skill import.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSkillFromDirectoryRequest {
    path: String,
}

/// Reads a native folder on the blocking executor and imports it through the shared Backend.
#[tauri::command]
pub async fn import_skill_from_directory(
    state: State<'_, DesktopState>,
    request: ImportSkillFromDirectoryRequest,
) -> Result<CreateSkillResponse, CommandError> {
    run_backend(
        "import_skill_from_directory",
        state.backend.clone(),
        request,
        import_selected_skill,
    )
    .await
}

/// Converts one selected directory into transport-neutral uploaded files before atomic import.
fn import_selected_skill(
    backend: &Backend,
    request: ImportSkillFromDirectoryRequest,
) -> Result<CreateSkillResponse, BackendError> {
    let files = read_uploaded_skill_folder(Path::new(&request.path))?;
    backend.import_skill(files)
}

/// Recursively materializes regular files without following links or exceeding shared limits.
fn read_uploaded_skill_folder(root: &Path) -> Result<Vec<UploadedSkillFile>, BackendError> {
    if !root.is_absolute() {
        return Err(directory_error(
            SkillImportDirectoryError::PathNotAbsolute {
                path: root.to_path_buf(),
            },
        ));
    }

    let root_metadata = match fs::symlink_metadata(root) {
        Ok(metadata) => metadata,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
            return Err(directory_error(
                SkillImportDirectoryError::PathNotDirectory {
                    path: root.to_path_buf(),
                },
            ));
        }
        Err(source) => {
            return Err(directory_error(SkillImportDirectoryError::FileSystem {
                path: root.to_path_buf(),
                source,
            }));
        }
    };
    if !root_metadata.is_dir() {
        return Err(directory_error(
            SkillImportDirectoryError::PathNotDirectory {
                path: root.to_path_buf(),
            },
        ));
    }

    let mut directories = vec![root.to_path_buf()];
    let mut files = Vec::new();
    let mut total_bytes = 0usize;

    while let Some(directory) = directories.pop() {
        let entries = fs::read_dir(&directory).map_err(|source| {
            directory_error(SkillImportDirectoryError::FileSystem {
                path: directory.clone(),
                source,
            })
        })?;
        let mut entries = entries.collect::<Result<Vec<_>, _>>().map_err(|source| {
            directory_error(SkillImportDirectoryError::FileSystem {
                path: directory.clone(),
                source,
            })
        })?;
        entries.sort_by_key(std::fs::DirEntry::file_name);

        for entry in entries {
            let path = entry.path();
            let file_type = entry.file_type().map_err(|source| {
                directory_error(SkillImportDirectoryError::FileSystem {
                    path: path.clone(),
                    source,
                })
            })?;

            // Directory links stay external to the selected package and file links cannot smuggle
            // contents from elsewhere into an otherwise self-contained skill.
            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                directories.push(path);
                continue;
            }
            if !file_type.is_file() {
                continue;
            }

            if files.len() >= MAX_SKILL_FILES {
                return Err(BackendError::from(
                    ApplicationError::SkillUploadTooManyFiles {
                        max_files: MAX_SKILL_FILES,
                    },
                ));
            }

            let metadata = entry.metadata().map_err(|source| {
                directory_error(SkillImportDirectoryError::FileSystem {
                    path: path.clone(),
                    source,
                })
            })?;
            let declared_bytes = usize::try_from(metadata.len()).unwrap_or(usize::MAX);
            if total_bytes
                .checked_add(declared_bytes)
                .is_none_or(|total| total > MAX_SKILL_UPLOAD_BYTES)
            {
                return Err(BackendError::from(ApplicationError::SkillUploadTooLarge {
                    max_bytes: MAX_SKILL_UPLOAD_BYTES,
                }));
            }

            let bytes = fs::read(&path).map_err(|source| {
                directory_error(SkillImportDirectoryError::FileSystem {
                    path: path.clone(),
                    source,
                })
            })?;
            total_bytes = total_bytes.checked_add(bytes.len()).ok_or_else(|| {
                BackendError::from(ApplicationError::SkillUploadTooLarge {
                    max_bytes: MAX_SKILL_UPLOAD_BYTES,
                })
            })?;
            if total_bytes > MAX_SKILL_UPLOAD_BYTES {
                return Err(BackendError::from(ApplicationError::SkillUploadTooLarge {
                    max_bytes: MAX_SKILL_UPLOAD_BYTES,
                }));
            }

            let relative_path = path.strip_prefix(root).map_err(|source| {
                directory_error(SkillImportDirectoryError::RelativePath {
                    path: path.clone(),
                    source,
                })
            })?;
            let relative_path = relative_path
                .components()
                .map(|component| component.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");

            files.push(UploadedSkillFile {
                relative_path,
                bytes,
            });
        }
    }

    Ok(files)
}

/// Projects native path failures without exposing operating-system diagnostics to the frontend.
fn directory_error(error: SkillImportDirectoryError) -> BackendError {
    let (classification, public_error, context) = match &error {
        SkillImportDirectoryError::PathNotAbsolute { .. }
        | SkillImportDirectoryError::PathNotDirectory { .. } => (
            ErrorClassification::InvalidRequest,
            PublicError::InvalidRequest(EmptyErrorParams {}),
            "selected skill import path is invalid",
        ),
        SkillImportDirectoryError::FileSystem { .. }
        | SkillImportDirectoryError::RelativePath { .. } => (
            ErrorClassification::Internal,
            PublicError::InternalError(EmptyErrorParams {}),
            "failed to read selected skill folder",
        ),
    };
    BackendError::with_source(classification, public_error, context, error)
}

/// Retains native directory diagnostics for correlated logs while public errors stay bounded.
#[derive(Debug, Error)]
enum SkillImportDirectoryError {
    #[error("selected skill path must be absolute: {path:?}")]
    PathNotAbsolute { path: PathBuf },
    #[error("selected skill path must be a directory: {path:?}")]
    PathNotDirectory { path: PathBuf },
    #[error("failed to access selected skill path {path:?}")]
    FileSystem {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("selected file escaped the skill root: {path:?}")]
    RelativePath {
        path: PathBuf,
        #[source]
        source: std::path::StripPrefixError,
    },
}

#[cfg(test)]
mod tests {
    use super::read_uploaded_skill_folder;
    use ora_application::{MAX_SKILL_FILES, MAX_SKILL_UPLOAD_BYTES, UploadedSkillFile};
    use ora_backend::ErrorClassification;
    use ora_contracts::{PublicError, SkillUploadTooLargeParams, SkillUploadTooManyFilesParams};
    use pretty_assertions::assert_eq;
    use std::fs;
    use tempfile::TempDir;

    /// Verifies hidden and nested regular files retain skill-root-relative slash paths.
    #[test]
    fn reads_regular_files_recursively() {
        let root = TempDir::new().expect("create skill root");
        fs::write(
            root.path().join("SKILL.md"),
            "---\nname: demo\ndescription: Demo\n---\n",
        )
        .expect("write manifest");
        fs::create_dir(root.path().join("references")).expect("create nested directory");
        fs::write(root.path().join("references").join(".hidden"), b"hidden")
            .expect("write hidden file");

        let mut actual =
            read_uploaded_skill_folder(root.path()).expect("read selected skill folder");
        actual.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));

        assert_eq!(
            actual,
            vec![
                UploadedSkillFile {
                    relative_path: "SKILL.md".to_string(),
                    bytes: b"---\nname: demo\ndescription: Demo\n---\n".to_vec(),
                },
                UploadedSkillFile {
                    relative_path: "references/.hidden".to_string(),
                    bytes: b"hidden".to_vec(),
                },
            ]
        );
    }

    /// Verifies a stale or removed picker result is rejected as an invalid request.
    #[test]
    fn rejects_missing_directories() {
        let root = TempDir::new().expect("create parent directory");
        let missing = root.path().join("missing");

        let error = read_uploaded_skill_folder(&missing).expect_err("reject missing directory");

        assert_eq!(error.classification(), ErrorClassification::InvalidRequest);
        assert_eq!(
            error.public_error(),
            &PublicError::InvalidRequest(ora_contracts::EmptyErrorParams {})
        );
    }

    /// Verifies Desktop rejects a sparse oversized file before materializing it in memory.
    #[test]
    fn rejects_oversized_folders() {
        let root = TempDir::new().expect("create skill root");
        let file = fs::File::create(root.path().join("large.bin")).expect("create sparse file");
        file.set_len((MAX_SKILL_UPLOAD_BYTES + 1) as u64)
            .expect("size sparse file");

        let error = read_uploaded_skill_folder(root.path()).expect_err("reject oversized folder");

        assert_eq!(error.classification(), ErrorClassification::PayloadTooLarge);
        assert_eq!(
            error.public_error(),
            &PublicError::SkillUploadTooLarge(SkillUploadTooLargeParams {
                max_bytes: MAX_SKILL_UPLOAD_BYTES,
            })
        );
    }

    /// Verifies Desktop stops enumeration at the same file-count limit as HTTP imports.
    #[test]
    fn rejects_folders_with_too_many_files() {
        let root = TempDir::new().expect("create skill root");
        for index in 0..=MAX_SKILL_FILES {
            fs::write(root.path().join(format!("{index}.txt")), []).expect("write skill file");
        }

        let error = read_uploaded_skill_folder(root.path()).expect_err("reject excessive files");

        assert_eq!(error.classification(), ErrorClassification::Unprocessable);
        assert_eq!(
            error.public_error(),
            &PublicError::SkillUploadTooManyFiles(SkillUploadTooManyFilesParams {
                max_files: MAX_SKILL_FILES,
            })
        );
    }

    /// Verifies file links are not copied into the imported package when Windows permits creation.
    #[cfg(windows)]
    #[test]
    fn does_not_follow_file_links() {
        use std::os::windows::fs::symlink_file;

        let root = TempDir::new().expect("create skill root");
        let external = TempDir::new().expect("create external root");
        let target = external.path().join("outside.txt");
        fs::write(&target, b"outside").expect("write external file");
        if symlink_file(&target, root.path().join("linked.txt")).is_err() {
            return;
        }

        let actual = read_uploaded_skill_folder(root.path()).expect("read selected skill folder");

        assert_eq!(actual, Vec::<UploadedSkillFile>::new());
    }
}
