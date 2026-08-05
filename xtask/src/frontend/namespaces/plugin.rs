//! Endpoint declarations for the plugin generated-client namespace.

use crate::frontend::{FrontendEndpoint, FrontendHttpMethod, NO_PATH_PARAMS};
use ora_contracts::INSTALLED_PLUGINS_PATH;

const NAMESPACE: &str = "plugin";

pub(super) const ENDPOINTS: &[FrontendEndpoint] = &[FrontendEndpoint {
    operation_name: "listInstalledPlugins",
    namespace: NAMESPACE,
    member_name: "listInstalled",
    method: FrontendHttpMethod::Get,
    path_template: INSTALLED_PLUGINS_PATH,
    request_type: "ListInstalledPluginsRequest",
    response_type: "ListInstalledPluginsResponse",
    path_params: NO_PATH_PARAMS,
    has_json_body: false,
}];
