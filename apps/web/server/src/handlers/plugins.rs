use crate::app_state::AppState;
use axum::Json;
use axum::extract::State;
use ora_contracts::{InstalledPlugin, InstalledPluginAgent, ListInstalledPluginsResponse};

/// Returns the immutable installed-plugin snapshot captured during server bootstrap.
pub async fn list_installed_plugins(
    State(app_state): State<AppState>,
) -> Json<ListInstalledPluginsResponse> {
    let plugins = app_state
        .plugin_manager()
        .installed_plugins()
        .iter()
        .map(|plugin| InstalledPlugin {
            id: plugin.id.clone(),
            package_name: plugin.package_name.clone(),
            display_name: plugin.display_name.clone(),
            version: plugin.version.to_string(),
            kind: plugin.kind.as_str().to_string(),
            main: plugin.main.to_string_lossy().to_string(),
            agents: plugin
                .agents
                .iter()
                .map(|agent| InstalledPluginAgent {
                    id: agent.id.clone(),
                    display_name: agent.display_name.clone(),
                    contract_version: agent.contract_version,
                })
                .collect(),
        })
        .collect();

    Json(ListInstalledPluginsResponse { plugins })
}
