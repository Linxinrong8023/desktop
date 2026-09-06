use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use ts_rs::TS;

/// Immutable capabilities published by a plugin after its registration is validated.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PluginRegistration {
    pub methods: HashSet<String>,
    pub emits: HashSet<String>,
    pub effect_resources: Vec<PluginEffectResource>,
}

/// Wire payload of the `ora/register` notification.
#[derive(Debug, Clone, Default, Deserialize, Serialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "registration.ts")]
pub struct PluginRegistrationParams {
    pub methods: Vec<String>,
    #[serde(default)]
    pub emits: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub effect_resources: Option<Vec<PluginEffectResource>>,
}

/// One Workspace-relative Effect Resource included in immutable plugin registration.
#[derive(Debug, Clone, Deserialize, Serialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "registration.ts")]
pub struct PluginEffectResource {
    pub workspace_relative_path: String,
    pub materialization_format: String,
    pub coordination: PluginEffectCoordination,
}

/// Selects the runtime barrier required before Ora mutates one declared Resource.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "registration.ts")]
pub enum PluginEffectCoordination {
    Uninterrupted,
    QuiesceBeforeMutation,
}

/// Parses a strict registration payload without losing duplicate method declarations.
pub fn parse_registration(params: Option<&Value>) -> Result<PluginRegistration, String> {
    let methods = parse_method_list(params, "methods")?
        .ok_or_else(|| "plugin registration is missing a methods array".to_string())?;
    let emits = parse_method_list(params, "emits")?.unwrap_or_default();
    let effect_resources = parse_effect_resources(params)?;
    Ok(PluginRegistration {
        methods,
        emits,
        effect_resources,
    })
}

/// Parses Effect descriptors as a strict registration contract rather than accepting opaque JSON.
fn parse_effect_resources(params: Option<&Value>) -> Result<Vec<PluginEffectResource>, String> {
    let Some(value) = params.and_then(|params| params.get("effectResources")) else {
        return Ok(Vec::new());
    };
    let entries = value
        .as_array()
        .ok_or_else(|| "plugin registration field effectResources must be an array".to_string())?;
    entries
        .iter()
        .map(|entry| {
            let object = entry.as_object().ok_or_else(|| {
                "plugin registration effectResources entry must be an object".to_string()
            })?;
            let required_string = |field: &str| {
                object
                    .get(field)
                    .and_then(Value::as_str)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string)
                    .ok_or_else(|| {
                        format!("plugin registration effectResources entry has invalid {field}")
                    })
            };
            let coordination = match required_string("coordination")?.as_str() {
                "uninterrupted" => PluginEffectCoordination::Uninterrupted,
                "quiesce_before_mutation" => PluginEffectCoordination::QuiesceBeforeMutation,
                value => {
                    return Err(format!(
                        "plugin registration effectResources entry has unknown coordination {value}"
                    ));
                }
            };
            Ok(PluginEffectResource {
                workspace_relative_path: required_string("workspaceRelativePath")?,
                materialization_format: required_string("materializationFormat")?,
                coordination,
            })
        })
        .collect()
}

/// Reads one optional registration array into a duplicate-free method set.
fn parse_method_list(
    params: Option<&Value>,
    field: &str,
) -> Result<Option<HashSet<String>>, String> {
    let Some(value) = params.and_then(|params| params.get(field)) else {
        return Ok(None);
    };
    let entries = value
        .as_array()
        .ok_or_else(|| format!("plugin registration field {field} must be an array"))?;
    let mut parsed = HashSet::with_capacity(entries.len());
    for entry in entries {
        let entry = entry
            .as_str()
            .filter(|entry| !entry.is_empty())
            .ok_or_else(|| {
                format!("plugin registration field {field} contains an invalid entry")
            })?;
        if !parsed.insert(entry.to_string()) {
            return Err(format!("plugin registered duplicate {field} entry {entry}"));
        }
    }
    Ok(Some(parsed))
}

/// Exports the registration DTO family into one TypeScript module.
pub(crate) fn export(config: &ts_rs::Config) -> Result<(), ts_rs::ExportError> {
    PluginRegistrationParams::export(config)?;
    PluginEffectResource::export(config)?;
    PluginEffectCoordination::export(config)?;
    Ok(())
}
