use ora_contracts::Skill as ContractSkill;
use ora_domain::Skill as DomainSkill;

/// Projects a domain skill into its audit-free public contract form.
pub(crate) fn map_skill(skill: DomainSkill) -> ContractSkill {
    ContractSkill {
        id: skill.id.to_string(),
        name: skill.name,
        description: skill.description,
    }
}
