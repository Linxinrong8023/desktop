//! Rejects references to excluded nodes before prerequisite preparation or session creation.

use super::graph::{GraphError, WorkflowGraph};
use super::loop_config::LoopInitialValue;
use super::variable_pool::VariableSelector;
use std::collections::HashSet;

/// Visits typed executable configuration, never arbitrary labels or JSON schema strings.
pub(super) fn validate(graph: &WorkflowGraph, unused: &[String]) -> Result<(), GraphError> {
    let unused: HashSet<&str> = unused.iter().map(String::as_str).collect();
    let check = |selector: &VariableSelector| require_used(&selector.node_id, &unused);
    for scope in graph.execution_scopes() {
        for node in scope.nodes() {
            for selector in node
                .condition_config
                .iter()
                .flat_map(|config| &config.cases)
                .flat_map(|case| &case.conditions)
                .map(|rule| &rule.variable_selector)
                .chain(
                    node.output_config
                        .iter()
                        .flat_map(|config| &config.outputs)
                        .map(|binding| &binding.variable_selector),
                )
            {
                check(selector)?;
            }
            if let Some(config) = &node.iteration_config {
                check(&config.iterator_selector)?;
                check(&config.collect_selector)?;
            }
            if let Some((config, _)) = scope.loop_body(&node.id) {
                for variable in &config.variables {
                    if let LoopInitialValue::Variable(selector) = &variable.initial {
                        check(selector)?;
                    }
                    check(&variable.feedback)?;
                }
                for selector in config
                    .outputs
                    .iter()
                    .map(|binding| &binding.variable_selector)
                    .chain(
                        config
                            .until
                            .cases
                            .iter()
                            .flat_map(|case| &case.conditions)
                            .map(|rule| &rule.variable_selector),
                    )
                {
                    check(selector)?;
                }
            }
            for text in node
                .agent_config
                .iter()
                .map(|config| config.prompt.as_str())
            {
                let mut remaining = text;
                while let Some((_, after_open)) = remaining.split_once("{{#") {
                    let Some((selector, rest)) = after_open.split_once("#}}") else {
                        break;
                    };
                    if let Some((node_id, _)) = selector.split_once('.') {
                        require_used(node_id, &unused)?;
                    }
                    remaining = rest;
                }
            }
        }
    }
    Ok(())
}

/// References never implicitly activate spare nodes or substitute empty values.
fn require_used(node_id: &str, unused: &HashSet<&str>) -> Result<(), GraphError> {
    if unused.contains(node_id) {
        return Err(GraphError::InvalidNode {
            reason: format!("variable references unused node {node_id}"),
        });
    }
    Ok(())
}
