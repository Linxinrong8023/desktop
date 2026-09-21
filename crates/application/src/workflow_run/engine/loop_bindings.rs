//! Enforces container visibility for explicit variable bindings before execution begins.

use super::graph::{GraphError, WorkflowGraph};
use super::loop_config::LoopInitialValue;
use super::variable_pool::VariableSelector;
use std::collections::HashSet;

/// Child values can leave a scope only through the enclosing Loop's named outputs.
pub(super) fn validate(root: &WorkflowGraph) -> Result<(), GraphError> {
    let globals: HashSet<_> = root
        .global_variables()
        .iter()
        .map(|item| item.name.clone())
        .collect();
    validate_node_bindings(root, &globals, &HashSet::new())?;
    for node in root.nodes_in_topological_order() {
        let Some((config, body)) = root.loop_body(&node.id) else {
            continue;
        };
        let ancestors: HashSet<_> = root
            .transitive_predecessors(&node.id)
            .into_iter()
            .map(|item| item.id.as_str())
            .collect();
        for variable in &config.variables {
            if let LoopInitialValue::Variable(selector) = &variable.initial {
                require_visible(selector, &globals, |id| ancestors.contains(id))?;
            }
        }
        let visible = |id: &str| id == node.id || body.node(id).is_some() || ancestors.contains(id);
        for selector in config
            .variables
            .iter()
            .map(|item| &item.feedback)
            .chain(config.outputs.iter().map(|item| &item.variable_selector))
            .chain(
                config
                    .until
                    .cases
                    .iter()
                    .flat_map(|case| &case.conditions)
                    .map(|rule| &rule.variable_selector),
            )
        {
            require_visible(selector, &globals, visible)?;
        }
        // Outer values are immutable inputs to a round, while child topology remains local.
        let mut inherited = ancestors;
        inherited.insert(node.id.as_str());
        validate_node_bindings(body, &globals, &inherited)?;
    }
    Ok(())
}

/// Applies the same boundary rule to ordinary Condition and Output nodes.
fn validate_node_bindings(
    graph: &WorkflowGraph,
    globals: &HashSet<String>,
    inherited: &HashSet<&str>,
) -> Result<(), GraphError> {
    for node in graph.nodes() {
        let ancestors: HashSet<_> = graph
            .transitive_predecessors(&node.id)
            .into_iter()
            .map(|item| item.id.as_str())
            .collect();
        let selectors = node
            .condition_config
            .iter()
            .flat_map(|config| &config.cases)
            .flat_map(|case| &case.conditions)
            .map(|rule| &rule.variable_selector)
            .chain(
                node.output_config
                    .iter()
                    .flat_map(|config| &config.outputs)
                    .map(|output| &output.variable_selector),
            );
        for selector in selectors {
            require_visible(selector, globals, |id| {
                ancestors.contains(id) || inherited.contains(id)
            })?;
        }
    }
    Ok(())
}

/// Topology controls availability; actual declarations and values remain owned by the variable pool.
fn require_visible(
    selector: &VariableSelector,
    inherited: &HashSet<String>,
    visible_node: impl Fn(&str) -> bool,
) -> Result<(), GraphError> {
    if visible_node(&selector.node_id) || inherited.contains(&selector.qualified()) {
        return Ok(());
    }
    Err(GraphError::InvalidNode {
        reason: format!(
            "selector {} is outside the visible Loop scope",
            selector.qualified()
        ),
    })
}
