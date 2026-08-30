# ora-effect-skill

`ora-effect-skill` integrates Skill Effects with the generic convergence machinery in
`ora-effect`.

It owns Skill target projection, shared Skill directory planning, filesystem observation and
mutation, ownership markers, staged `SKILL.md` validation, and package fingerprinting. Callers
compose `SkillPlanner` and `SkillDirectoryResourceAdapter` with `EffectReconciler`; `ora-effect` does
not re-export either adapter.
