use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Describes a public configurable-agent payload without persistence audit metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "agent.ts")]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[cfg(test)]
mod tests {
    use super::Agent;
    use pretty_assertions::assert_eq;
    use serde_json::json;

    /// Verifies public agent payloads exclude persistence-owned audit fields.
    #[test]
    fn serializes_agent_contract_without_audit_fields() {
        let agent = Agent {
            id: "agent-1".to_string(),
            name: "opencode".to_string(),
            description: "OpenCode agent configuration".to_string(),
        };

        assert_eq!(
            serde_json::to_value(agent).unwrap(),
            json!({
                "id": "agent-1",
                "name": "opencode",
                "description": "OpenCode agent configuration",
            })
        );
    }
}
