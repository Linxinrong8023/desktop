//! Pure round decisions, computed before a persistence adapter commits any advancement.

use super::condition::{ConditionError, ELSE_BRANCH_ID, evaluate_condition};
use super::graph::WorkflowGraph;
use super::loop_config::{LoopConfig, LoopInitialValue};
use super::variable_pool::{VariableSelector, WorkflowVariablePool, WorkflowVariablePoolError};
use super::variable_value::normalize_workflow_value;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use thiserror::Error;

/// A completed round either supplies the next round's inputs or exports the final results.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoopRoundDecision {
    Continue { carried: BTreeMap<String, Value> },
    Succeeded { outputs: BTreeMap<String, Value> },
}

/// Durable execution data private to one round scope.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoopRoundExecutionState {
    pub variable_pool: WorkflowVariablePool,
    pub condition_decisions: BTreeMap<String, String>,
}

/// Failures which must abort advancement without publishing a partially updated variable set.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum LoopRoundError {
    #[error("Loop container does not exist: {node_id}")]
    UnknownLoop { node_id: String },
    #[error("carried values do not match the Loop variable declarations")]
    InvalidCarriedVariables,
    #[error("invalid Loop round {round}; expected 1 through {max_iterations}")]
    InvalidRound { round: u32, max_iterations: u32 },
    #[error("Loop did not terminate within {max_iterations} rounds")]
    LimitReached { max_iterations: u32 },
    #[error("Loop variable {name} does not match declared type {value_type}")]
    TypeMismatch { name: String, value_type: String },
    #[error("Loop selector has no value in this round: {selector}")]
    UnsetValue { selector: String },
    #[error(transparent)]
    Variable(#[from] WorkflowVariablePoolError),
    #[error(transparent)]
    Condition(#[from] ConditionError),
}

impl WorkflowGraph {
    /// Builds a fresh round pool; only upstream outer values and explicit carried values are imported.
    pub fn loop_round_pool(
        &self,
        loop_id: &str,
        outer: &WorkflowVariablePool,
        carried: &BTreeMap<String, Value>,
    ) -> Result<WorkflowVariablePool, LoopRoundError> {
        let (config, body) =
            self.loop_body(loop_id)
                .ok_or_else(|| LoopRoundError::UnknownLoop {
                    node_id: loop_id.into(),
                })?;
        if carried.len() != config.variables.len()
            || config
                .variables
                .iter()
                .any(|variable| !carried.contains_key(&variable.name))
        {
            return Err(LoopRoundError::InvalidCarriedVariables);
        }
        let ancestors: std::collections::HashSet<_> = self
            .transitive_predecessors(loop_id)
            .into_iter()
            .map(|node| node.id.as_str())
            .collect();
        let globals: std::collections::HashSet<_> = self
            .global_variables()
            .iter()
            .map(|variable| variable.name.as_str())
            .collect();
        let mut pool = WorkflowVariablePool::from_graph(body);
        // Never clone the previous round's pool: absent branch outputs must remain unassigned.
        // Imported definitions retain their original writer, so child nodes cannot mutate them.
        for (key, definition) in &outer.catalog {
            if !ancestors.contains(definition.writer.as_str()) && !globals.contains(key.as_str()) {
                continue;
            }
            pool.catalog.insert(key.clone(), definition.clone());
            if let Some(value) = outer.values.get(key) {
                pool.values.insert(key.clone(), value.clone());
            }
        }
        for variable in &config.variables {
            let key = format!("{loop_id}.{}", variable.name);
            pool.declare(&key, &variable.value_type, loop_id);
            pool.set(&key, loop_id, carried[&variable.name].clone())?;
        }
        Ok(pool)
    }
}

impl LoopConfig {
    /// Freezes carried inputs once at container entry, without modifying the outer pool.
    pub fn initialize_carried(
        &self,
        outer: &WorkflowVariablePool,
    ) -> Result<BTreeMap<String, Value>, LoopRoundError> {
        self.variables
            .iter()
            .map(|variable| {
                let value = match &variable.initial {
                    LoopInitialValue::Constant(value) => value.clone(),
                    LoopInitialValue::Variable(selector) => resolve_required(outer, selector)?,
                };
                let value =
                    normalize_workflow_value(value, &variable.value_type).ok_or_else(|| {
                        LoopRoundError::TypeMismatch {
                            name: variable.name.clone(),
                            value_type: variable.value_type.clone(),
                        }
                    })?;
                Ok((variable.name.clone(), value))
            })
            .collect()
    }

    /// Decides advancement from one immutable completed-round snapshot using one-based rounds.
    pub fn complete_round(
        &self,
        round: u32,
        completed: &WorkflowVariablePool,
    ) -> Result<LoopRoundDecision, LoopRoundError> {
        if round == 0 || round > self.max_iterations {
            return Err(LoopRoundError::InvalidRound {
                round,
                max_iterations: self.max_iterations,
            });
        }
        // All feedback reads observe the same snapshot, including assignments that swap values.
        // Returning owned values lets the repository commit them together or discard them all.
        let carried = self
            .variables
            .iter()
            .map(|variable| {
                let value = resolve_required(completed, &variable.feedback)?;
                let value =
                    normalize_workflow_value(value, &variable.value_type).ok_or_else(|| {
                        LoopRoundError::TypeMismatch {
                            name: variable.name.clone(),
                            value_type: variable.value_type.clone(),
                        }
                    })?;
                Ok((variable.name.clone(), value))
            })
            .collect::<Result<BTreeMap<_, _>, LoopRoundError>>()?;
        if evaluate_condition(&self.until, completed)? != ELSE_BRANCH_ID {
            let outputs = self
                .outputs
                .iter()
                .map(|output| {
                    Ok((
                        output.name.clone(),
                        resolve_required(completed, &output.variable_selector)?,
                    ))
                })
                .collect::<Result<_, LoopRoundError>>()?;
            return Ok(LoopRoundDecision::Succeeded { outputs });
        }
        // A successful final permitted round still succeeds; only a false termination hits the cap.
        if round == self.max_iterations {
            return Err(LoopRoundError::LimitReached {
                max_iterations: self.max_iterations,
            });
        }
        Ok(LoopRoundDecision::Continue { carried })
    }
}

/// An inactive branch's unset value is an error, never a previous round's implicit fallback.
fn resolve_required(
    pool: &WorkflowVariablePool,
    selector: &VariableSelector,
) -> Result<Value, LoopRoundError> {
    pool.resolve(selector)?
        .cloned()
        .ok_or_else(|| LoopRoundError::UnsetValue {
            selector: std::iter::once(selector.qualified())
                .chain(selector.nested.iter().cloned())
                .collect::<Vec<_>>()
                .join("."),
        })
}

#[cfg(test)]
mod tests;
