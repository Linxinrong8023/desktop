use crate::workflow_run::engine::{
    AgentConfig, AgentExecutor, AgentSkill, GraphError, NodeType, OutputPolicy, UnknownNodeType,
    WorkflowGraph, WorkflowGraphNode,
};
use pretty_assertions::assert_eq;
use serde_json::{Value, json};
use std::collections::HashSet;
use std::str::FromStr;

/// Parses a JSON value as a frozen workflow graph, failing the test on error.
fn parse(value: Value) -> Result<WorkflowGraph, GraphError> {
    WorkflowGraph::parse(&value.to_string())
}

/// A linear chain matching the demo shape: start → agent a → agent b → output-1.
fn linear_chain() -> Value {
    json!({
        "id": "wf-1",
        "name": "Demo",
        "description": "A linear demo",
        "updatedAt": "2026-08-07T00:00:00Z",
        "viewport": { "x": 0.0, "y": 0.0, "zoom": 1.0 },
        "nodes": [
            { "id": "start", "type": "workflow", "data": { "kind": "start", "title": "开始", "instruction": "input" } },
            { "id": "a", "type": "workflow", "data": { "kind": "agent", "agentConfig": {
                "schemaVersion": 3,
                "executor": { "agentCli": "open_code", "modelId": "model-1" },
                "roleId": "Researcher",
                "skills": [{ "skillId": "explore", "enabled": true }],
                "prompt": "do a"
            } } },
            { "id": "b", "type": "workflow", "data": { "kind": "agent", "agentConfig": {
                "executor": { "agentCli": "open_code", "modelId": "model-1" },
                "roleId": "Reviewer",
                "skills": [],
                "prompt": "do b"
            } } },
            { "id": "output-1", "type": "workflow", "data": { "kind": "output", "title": "输出", "instruction": "" } }
        ],
        "edges": [
            { "id": "e1", "source": "start", "target": "a" },
            { "id": "e2", "source": "a", "target": "b" },
            { "id": "e3", "source": "b", "target": "output-1" }
        ]
    })
}

/// Two parallel branches merging into one node: start → left/right → merge → out.
fn fan_in() -> Value {
    json!({
        "nodes": [
            { "id": "start", "data": { "kind": "start" } },
            { "id": "left", "data": { "kind": "agent", "agentConfig": {
                "executor": { "agentCli": "open_code", "modelId": "m" }, "roleId": "R", "skills": [], "prompt": "left"
            } } },
            { "id": "right", "data": { "kind": "agent", "agentConfig": {
                "executor": { "agentCli": "open_code", "modelId": "m" }, "roleId": "R", "skills": [], "prompt": "right"
            } } },
            { "id": "merge", "data": { "kind": "agent", "agentConfig": {
                "executor": { "agentCli": "open_code", "modelId": "m" }, "roleId": "R", "skills": [], "prompt": "merge"
            } } },
            { "id": "out", "data": { "kind": "output" } }
        ],
        "edges": [
            { "source": "start", "target": "left" },
            { "source": "start", "target": "right" },
            { "source": "left", "target": "merge" },
            { "source": "right", "target": "merge" },
            { "source": "merge", "target": "out" }
        ]
    })
}

/// Returns the ids of the given nodes in order.
fn ids(nodes: &[&WorkflowGraphNode]) -> Vec<String> {
    nodes.iter().map(|node| node.id.clone()).collect()
}

// ── Parse: valid shapes ──

#[test]
fn parses_valid_envelope_with_metadata() {
    let graph = parse(linear_chain()).unwrap();
    assert_eq!(graph.node_count(), 4);
    assert_eq!(graph.edge_count(), 3);
    assert_eq!(graph.start_node().unwrap().id, "start");
    assert_eq!(ids(&graph.successors("start")), vec!["a"]);
    assert_eq!(ids(&graph.predecessors("output-1")), vec!["b"]);
}

#[test]
fn parses_agent_config_into_the_model() {
    let graph = parse(linear_chain()).unwrap();
    let expected = WorkflowGraphNode {
        id: "a".to_string(),
        node_type: NodeType::Agent,
        title: String::new(),
        description: String::new(),
        instruction: None,
        agent_config: Some(AgentConfig {
            executor: AgentExecutor {
                agent_cli: "open_code".to_string(),
                model_id: "model-1".to_string(),
            },
            role_id: Some("Researcher".to_string()),
            skills: vec![AgentSkill {
                skill_id: "explore".to_string(),
                enabled: true,
            }],
            prompt: "do a".to_string(),
            // The linear_chain fixture omits `interactive`, so the default must be false.
            interactive: false,
            // The linear_chain fixture omits `outputPolicy`, so the default must be `None`.
            output_policy: OutputPolicy::None,
        }),
    };
    assert_eq!(*graph.node("a").unwrap(), expected);
}

#[test]
fn parses_interactive_agent_flag_with_a_default_of_false() {
    let graph = parse(json!({
        "nodes": [
            { "id": "start", "data": { "kind": "start" } },
            { "id": "a", "data": { "kind": "agent", "agentConfig": {
                "executor": { "agentCli": "open_code", "modelId": "m" },
                "roleId": "R", "skills": [], "prompt": "a", "interactive": true
            } } },
            { "id": "b", "data": { "kind": "agent", "agentConfig": {
                "executor": { "agentCli": "open_code", "modelId": "m" },
                "roleId": "R", "skills": [], "prompt": "b"
            } } },
            { "id": "out", "data": { "kind": "output" } }
        ],
        "edges": [
            { "source": "start", "target": "a" },
            { "source": "a", "target": "b" },
            { "source": "b", "target": "out" }
        ]
    }))
    .unwrap();
    assert_eq!(
        graph
            .node("a")
            .unwrap()
            .agent_config
            .as_ref()
            .unwrap()
            .interactive,
        true
    );
    assert_eq!(
        graph
            .node("b")
            .unwrap()
            .agent_config
            .as_ref()
            .unwrap()
            .interactive,
        false
    );
}

#[test]
fn parses_output_policy_with_a_default_of_none() {
    let graph = parse(json!({
        "nodes": [
            { "id": "a", "data": { "kind": "agent", "agentConfig": {
                "executor": { "agentCli": "open_code", "modelId": "m" },
                "roleId": "R", "skills": [], "prompt": "a", "outputPolicy": "none"
            } } },
            { "id": "b", "data": { "kind": "agent", "agentConfig": {
                "executor": { "agentCli": "open_code", "modelId": "m" },
                "roleId": "R", "skills": [], "prompt": "b", "outputPolicy": "final_agent_response"
            } } },
            { "id": "c", "data": { "kind": "agent", "agentConfig": {
                "executor": { "agentCli": "open_code", "modelId": "m" },
                "roleId": "R", "skills": [], "prompt": "c"
            } } }
        ],
        "edges": []
    }))
    .unwrap();
    let policy = |id: &str| {
        graph
            .node(id)
            .unwrap()
            .agent_config
            .as_ref()
            .unwrap()
            .output_policy
    };
    assert_eq!(policy("a"), OutputPolicy::None);
    assert_eq!(policy("b"), OutputPolicy::FinalAgentResponse);
    // Omitting `outputPolicy` defaults to `None`, so nodes withhold output unless opted in.
    assert_eq!(policy("c"), OutputPolicy::None);
}

#[test]
fn output_policy_apply_withholds_or_passes_output() {
    assert_eq!(OutputPolicy::None.apply(Some("text".to_string())), None);
    assert_eq!(OutputPolicy::None.apply(None), None);
    assert_eq!(
        OutputPolicy::FinalAgentResponse.apply(Some("text".to_string())),
        Some("text".to_string())
    );
    assert_eq!(OutputPolicy::FinalAgentResponse.apply(None), None);
}

#[test]
fn parses_empty_graph_as_legal() {
    let graph = parse(json!({ "nodes": [], "edges": [] })).unwrap();
    assert_eq!(graph.node_count(), 0);
    assert_eq!(graph.edge_count(), 0);
    assert_eq!(graph.start_node(), None);
}

// ── Parse: structural errors ──

#[test]
fn rejects_invalid_json() {
    assert_eq!(
        WorkflowGraph::parse("not json").unwrap_err(),
        GraphError::InvalidJson
    );
    assert_eq!(
        WorkflowGraph::parse("[]").unwrap_err(),
        GraphError::InvalidJson
    );
    assert_eq!(
        WorkflowGraph::parse("\"text\"").unwrap_err(),
        GraphError::InvalidJson
    );
}

#[test]
fn rejects_missing_nodes() {
    assert_eq!(
        parse(json!({ "edges": [] })).unwrap_err(),
        GraphError::MissingNodes
    );
}

#[test]
fn rejects_missing_edges() {
    assert_eq!(
        parse(json!({ "nodes": [] })).unwrap_err(),
        GraphError::MissingEdges
    );
}

#[test]
fn rejects_node_missing_id() {
    assert_eq!(
        parse(json!({ "nodes": [{ "data": { "kind": "start" } }], "edges": [] })).unwrap_err(),
        GraphError::InvalidNode {
            reason: "missing id".to_string()
        }
    );
}

#[test]
fn rejects_node_missing_node_type() {
    assert_eq!(
        parse(json!({ "nodes": [{ "id": "a", "data": { "title": "x" } }], "edges": [] }))
            .unwrap_err(),
        GraphError::InvalidNode {
            reason: "node a has no node type".to_string()
        }
    );
}

#[test]
fn rejects_unknown_node_type() {
    assert_eq!(
        parse(json!({ "nodes": [{ "id": "a", "data": { "kind": "bogus" } }], "edges": [] }))
            .unwrap_err(),
        GraphError::UnknownNodeType {
            node_id: "a".to_string(),
            value: "bogus".to_string()
        }
    );
}

#[test]
fn rejects_dangling_edge() {
    assert_eq!(
        parse(json!({
            "nodes": [{ "id": "start", "data": { "kind": "start" } }],
            "edges": [{ "source": "start", "target": "missing" }]
        }))
        .unwrap_err(),
        GraphError::DanglingEdge {
            node_id: "missing".to_string()
        }
    );
}

#[test]
fn rejects_cycle() {
    assert_eq!(
        parse(json!({
            "nodes": [
                { "id": "a", "data": { "kind": "start" } },
                { "id": "b", "data": { "kind": "agent", "agentConfig": {
                    "executor": { "agentCli": "c", "modelId": "m" }, "roleId": "R", "skills": [], "prompt": "b"
                } } }
            ],
            "edges": [
                { "source": "a", "target": "b" },
                { "source": "b", "target": "a" }
            ]
        }))
        .unwrap_err(),
        GraphError::CycleDetected
    );
}

#[test]
fn rejects_self_loop_as_cycle() {
    assert_eq!(
        parse(json!({
            "nodes": [{ "id": "a", "data": { "kind": "start" } }],
            "edges": [{ "source": "a", "target": "a" }]
        }))
        .unwrap_err(),
        GraphError::CycleDetected
    );
}

#[test]
fn rejects_multiple_start_nodes() {
    assert_eq!(
        parse(json!({
            "nodes": [
                { "id": "a", "data": { "kind": "start" } },
                { "id": "b", "data": { "kind": "start" } }
            ],
            "edges": []
        }))
        .unwrap_err(),
        GraphError::MultipleStartNodes
    );
}

#[test]
fn rejects_duplicate_node_id() {
    assert_eq!(
        parse(json!({
            "nodes": [
                { "id": "a", "data": { "kind": "start" } },
                { "id": "a", "data": { "kind": "output" } }
            ],
            "edges": []
        }))
        .unwrap_err(),
        GraphError::DuplicateNodeId {
            node_id: "a".to_string()
        }
    );
}

// ── Topology ──

#[test]
fn transitive_successors_follow_flow_order() {
    let graph = parse(linear_chain()).unwrap();
    assert_eq!(
        ids(&graph.transitive_successors("start")),
        vec!["a", "b", "output-1"]
    );
}

#[test]
fn transitive_predecessors_follow_upstream_first_order() {
    let graph = parse(linear_chain()).unwrap();
    assert_eq!(
        ids(&graph.transitive_predecessors("output-1")),
        vec!["start", "a", "b"]
    );
}

#[test]
fn all_nodes_follow_topological_execution_order() {
    let graph = parse(linear_chain()).unwrap();
    assert_eq!(
        ids(&graph.nodes_in_topological_order()),
        vec!["start", "a", "b", "output-1"]
    );
}

#[test]
fn transitive_successors_exclude_the_seed() {
    let graph = parse(fan_in()).unwrap();
    let successors = ids(&graph.transitive_successors("start"));
    let mut sorted = successors.clone();
    sorted.sort();
    assert_eq!(sorted, vec!["left", "merge", "out", "right"]);
    assert!(!successors.contains(&"start".to_string()));
}

#[test]
fn transitive_predecessors_of_fan_in_are_stable_and_upstream_first() {
    let graph = parse(fan_in()).unwrap();
    let predecessors = ids(&graph.transitive_predecessors("merge"));
    let mut sorted = predecessors.clone();
    sorted.sort();
    assert_eq!(sorted, vec!["left", "right", "start"]);
    // The start node has the lowest topological rank and must lead the lineage.
    assert_eq!(predecessors[0], "start");
    // Two queries return the identical order, pinning determinism.
    assert_eq!(predecessors, ids(&graph.transitive_predecessors("merge")));
}

#[test]
fn ready_set_starts_with_the_start_node() {
    let graph = parse(linear_chain()).unwrap();
    let completed = HashSet::new();
    assert_eq!(ids(&graph.ready_set(&completed)), vec!["start"]);
}

#[test]
fn ready_set_advances_along_the_chain() {
    let graph = parse(linear_chain()).unwrap();
    let completed = HashSet::from(["start"]);
    assert_eq!(ids(&graph.ready_set(&completed)), vec!["a"]);
}

#[test]
fn ready_set_handles_fan_in() {
    let graph = parse(fan_in()).unwrap();
    let completed = HashSet::from(["start", "left", "right"]);
    assert_eq!(ids(&graph.ready_set(&completed)), vec!["merge"]);
}

#[test]
fn fan_in_is_not_ready_until_every_predecessor_completes() {
    let graph = parse(fan_in()).unwrap();
    let completed = HashSet::from(["start", "left"]);
    // `right` becomes ready, but `merge` must wait for `right`.
    assert_eq!(ids(&graph.ready_set(&completed)), vec!["right"]);
}

#[test]
fn first_unsupported_node_reports_condition() {
    let graph = parse(json!({
        "nodes": [
            { "id": "start", "data": { "kind": "start" } },
            { "id": "c", "data": { "kind": "condition", "condition": "x" } }
        ],
        "edges": []
    }))
    .unwrap();
    assert_eq!(graph.first_unsupported_node().unwrap().id, "c");
}

#[test]
fn first_unsupported_node_is_none_for_supported_graphs() {
    let graph = parse(linear_chain()).unwrap();
    assert_eq!(graph.first_unsupported_node(), None);
}

#[test]
fn unreachable_from_start_reports_isolated_nodes() {
    let graph = parse(json!({
        "nodes": [
            { "id": "start", "data": { "kind": "start" } },
            { "id": "a", "data": { "kind": "output" } },
            { "id": "orphan", "data": { "kind": "agent", "agentConfig": {
                "executor": { "agentCli": "c", "modelId": "m" }, "roleId": "R", "skills": [], "prompt": "orphan"
            } } }
        ],
        "edges": [{ "source": "start", "target": "a" }]
    }))
    .unwrap();
    assert_eq!(graph.unreachable_from_start(), vec!["orphan"]);
}

#[test]
fn unreachable_from_start_is_empty_for_a_connected_graph() {
    let graph = parse(linear_chain()).unwrap();
    assert_eq!(graph.unreachable_from_start(), Vec::<String>::new());
}

// ── Node type registry ──

#[test]
fn node_type_round_trips_all_variants() {
    for (value, expected) in [
        ("start", NodeType::Start),
        ("agent", NodeType::Agent),
        ("prompt", NodeType::Prompt),
        ("condition", NodeType::Condition),
        ("tool", NodeType::Tool),
        ("output", NodeType::Output),
    ] {
        let parsed = NodeType::from_str(value).unwrap();
        assert_eq!(parsed, expected);
        assert_eq!(parsed.as_str(), value);
    }
}

#[test]
fn node_type_rejects_unknown_values() {
    assert_eq!(
        NodeType::from_str("bogus"),
        Err(UnknownNodeType("bogus".to_string()))
    );
}

#[test]
fn node_type_reports_the_v1_supported_set() {
    let supported: Vec<&str> = [
        NodeType::Start,
        NodeType::Agent,
        NodeType::Prompt,
        NodeType::Condition,
        NodeType::Tool,
        NodeType::Output,
    ]
    .iter()
    .filter(|node_type| node_type.supported())
    .map(|node_type| node_type.as_str())
    .collect();
    assert_eq!(supported, vec!["start", "agent", "output"]);
}
