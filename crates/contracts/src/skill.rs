use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Describes a public skill payload without persistence audit metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "skill.ts")]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[cfg(test)]
mod tests {
    use super::Skill;
    use pretty_assertions::assert_eq;
    use serde_json::json;

    /// Verifies public skill payloads exclude persistence-owned audit fields.
    #[test]
    fn serializes_skill_contract_without_audit_fields() {
        let skill = Skill {
            id: "skill-1".to_string(),
            name: "review".to_string(),
            description: "Reviews implementation changes".to_string(),
        };

        assert_eq!(
            serde_json::to_value(skill).unwrap(),
            json!({
                "id": "skill-1",
                "name": "review",
                "description": "Reviews implementation changes",
            })
        );
    }
}
