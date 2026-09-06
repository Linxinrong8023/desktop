# ora-effect-skill

`ora-effect-skill` integrates Skill Effects with the generic convergence machinery in
`ora-effect`.

It owns Skill target projection, shared Skill directory planning, filesystem observation and
mutation, ownership markers, staged `SKILL.md` validation, and managed-directory fingerprinting.
Callers compose one `SkillPlanner` and one `SkillDirectoryResourceAdapter` with `EffectReconciler`;
`ora-effect` does not re-export either implementation.

The planner carries the materialization contract from the Consumer template through each Target
binding and Resource requirement. Shared contributors with different contracts block before an
operation can be prepared.
