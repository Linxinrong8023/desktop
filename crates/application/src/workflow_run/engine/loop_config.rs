//! Frozen Loop configuration, compiled using the existing workflow value and condition owners.

use super::condition::{ConditionConfig, WireConditionCase};
use super::graph::OutputBinding;
use super::variable_pool::VariableSelector;
use super::variable_value::{is_supported_variable_type, normalize_workflow_value};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashSet;

/// A bounded feedback container's validated configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct LoopConfig {
    pub max_iterations: u32,
    pub variables: Vec<LoopVariable>,
    pub until: ConditionConfig,
    pub outputs: Vec<OutputBinding>,
}

/// One typed carried value, with an explicit initializer and simultaneous feedback assignment.
#[derive(Debug, Clone, PartialEq)]
pub struct LoopVariable {
    pub name: String,
    pub value_type: String,
    pub initial: LoopInitialValue,
    pub feedback: VariableSelector,
}

/// Separates literal JSON (including null) from a reference evaluated at container entry.
#[derive(Debug, Clone, PartialEq)]
pub enum LoopInitialValue {
    Constant(Value),
    Variable(VariableSelector),
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WireLoopConfig {
    max_iterations: u32,
    variables: Vec<WireLoopVariable>,
    until: WireUntil,
    outputs: Vec<WireBinding>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WireLoopVariable {
    name: String,
    value_type: String,
    initial: WireInitialValue,
    feedback: Vec<String>,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
enum WireInitialValue {
    Constant { value: Value },
    Variable { selector: Vec<String> },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireUntil {
    logic: String,
    conditions: Vec<super::condition::WireConditionRule>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WireBinding {
    name: String,
    variable_selector: Vec<String>,
}

impl LoopConfig {
    /// Rejects malformed configuration before any execution or dependency preparation takes place.
    pub(super) fn parse(value: Value) -> Result<Self, String> {
        let wire: WireLoopConfig =
            serde_json::from_value(value).map_err(|error| error.to_string())?;
        if !(1..=100).contains(&wire.max_iterations) {
            return Err("maxIterations must be between 1 and 100".into());
        }
        if wire.until.conditions.is_empty() {
            return Err("until must contain at least one condition".into());
        }
        let until = ConditionConfig::from_wire(vec![WireConditionCase {
            id: Some("until".into()),
            logic: Some(wire.until.logic),
            conditions: wire.until.conditions,
        }])
        .map_err(|error| error.to_string())?;
        let mut names = HashSet::new();
        let mut variables = Vec::new();
        for variable in wire.variables {
            validate_name(&variable.name)?;
            if !names.insert(variable.name.clone()) {
                return Err(format!("duplicate loop variable {}", variable.name));
            }
            if !is_supported_variable_type(&variable.value_type) {
                return Err(format!(
                    "unsupported loop variable type {}",
                    variable.value_type
                ));
            }
            let initial = match variable.initial {
                WireInitialValue::Constant { value } => LoopInitialValue::Constant(
                    normalize_workflow_value(value, &variable.value_type)
                        .ok_or_else(|| format!("invalid initial value for {}", variable.name))?,
                ),
                WireInitialValue::Variable { selector } => {
                    LoopInitialValue::Variable(parse_selector(&selector)?)
                }
            };
            variables.push(LoopVariable {
                name: variable.name,
                value_type: variable.value_type,
                initial,
                feedback: parse_selector(&variable.feedback)?,
            });
        }
        let mut names = HashSet::new();
        let mut outputs = Vec::new();
        for output in wire.outputs {
            validate_name(&output.name)?;
            if !names.insert(output.name.clone()) {
                return Err(format!("duplicate loop output {}", output.name));
            }
            outputs.push(OutputBinding {
                name: output.name,
                variable_selector: parse_selector(&output.variable_selector)?,
            });
        }
        Ok(Self {
            max_iterations: wire.max_iterations,
            variables,
            until,
            outputs,
        })
    }
}

/// Names form selector segments, so dots and blank segments would make references ambiguous.
fn validate_name(name: &str) -> Result<(), String> {
    if name.trim().is_empty() || name.trim() != name || name.contains('.') {
        return Err(format!("invalid loop variable or output name {name}"));
    }
    Ok(())
}

/// Uses the shared selector grammar rather than accepting free-text variable paths.
fn parse_selector(parts: &[String]) -> Result<VariableSelector, String> {
    VariableSelector::try_from_parts(parts)
        .ok_or_else(|| format!("invalid loop selector {parts:?}"))
}
