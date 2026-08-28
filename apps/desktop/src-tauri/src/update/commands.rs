//! Tauri command seam for the Desktop updater.

use super::DesktopUpdateStatus;
use crate::error::CommandError;
use crate::state::DesktopState;
use ora_backend::BackendError;

/// Exposes the current status so a freshly mounted webview does not have to wait for an event.
#[tauri::command]
pub async fn get_desktop_update_status(
    state: tauri::State<'_, DesktopState>,
) -> Result<DesktopUpdateStatus, CommandError> {
    Ok(state.update.status())
}

/// Installs the verified cached update through the updater service.
#[tauri::command]
pub async fn install_desktop_update(
    state: tauri::State<'_, DesktopState>,
) -> Result<(), CommandError> {
    state.update.install().await.map_err(|error| {
        CommandError::from_backend(BackendError::internal(
            "failed to install Desktop update",
            error,
        ))
    })
}

/// Runs an update check on demand, outside the scheduled delayed and cron checks.
#[tauri::command]
pub async fn check_desktop_update(
    state: tauri::State<'_, DesktopState>,
) -> Result<(), CommandError> {
    state.update.check_and_download().await;
    Ok(())
}
