use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Ranks how important one plan entry is relative to the rest of the plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "acp/plan.ts")]
pub enum PlanEntryPriority {
    High,
    Medium,
    Low,
}

/// Describes how far the agent has progressed through one plan entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "acp/plan.ts")]
pub enum PlanEntryStatus {
    Pending,
    InProgress,
    Completed,
}

/// Describes one step the agent intends to take during the current turn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "acp/plan.ts")]
pub struct PlanEntry {
    /// Describes the step in human-readable form.
    pub content: String,
    pub priority: PlanEntryPriority,
    pub status: PlanEntryStatus,
}

/// Carries the agent's complete plan for the current turn.
///
/// Each update replaces the previous plan outright, so entries are never merged.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "acp/plan.ts")]
pub struct Plan {
    pub entries: Vec<PlanEntry>,
}

#[cfg(test)]
mod tests {
    use super::{Plan, PlanEntry, PlanEntryPriority, PlanEntryStatus};
    use pretty_assertions::assert_eq;
    use serde_json::json;

    /// Verifies plan entry enums use the snake_case spellings the protocol defines.
    #[test]
    fn serializes_plan_entry_with_protocol_enum_spellings() {
        assert_serialized_json(
            &PlanEntry {
                content: "Check the code style in the repository".to_string(),
                priority: PlanEntryPriority::Medium,
                status: PlanEntryStatus::InProgress,
            },
            json!({
                "content": "Check the code style in the repository",
                "priority": "medium",
                "status": "in_progress",
            }),
        );
        assert_serialized_json(&PlanEntryPriority::High, json!("high"));
        assert_serialized_json(&PlanEntryPriority::Low, json!("low"));
        assert_serialized_json(&PlanEntryStatus::Pending, json!("pending"));
        assert_serialized_json(&PlanEntryStatus::Completed, json!("completed"));
    }

    /// Verifies a plan carries every entry, including the empty starting plan.
    #[test]
    fn serializes_plan_as_complete_entry_list() {
        assert_serialized_json(&Plan::default(), json!({ "entries": [] }));
        assert_serialized_json(
            &Plan {
                entries: vec![
                    PlanEntry {
                        content: "Find the login function".to_string(),
                        priority: PlanEntryPriority::High,
                        status: PlanEntryStatus::Completed,
                    },
                    PlanEntry {
                        content: "Add unit tests".to_string(),
                        priority: PlanEntryPriority::Low,
                        status: PlanEntryStatus::Pending,
                    },
                ],
            },
            json!({
                "entries": [
                    { "content": "Find the login function", "priority": "high", "status": "completed" },
                    { "content": "Add unit tests", "priority": "low", "status": "pending" },
                ],
            }),
        );
    }

    fn assert_serialized_json(value: &impl serde::Serialize, expected: serde_json::Value) {
        assert_eq!(serde_json::to_value(value).unwrap(), expected);
    }
}
