# 三种 Generation 是三只独立时钟

用户能够从“只启用一个支持热加载的新 Skill”推导：必然推进的是 Workspace Effect generation，因为期望状态变化；Agent CLI 不必重启，因此 Plugin process generation 与 Agent connection generation 都可以保持不变。这个回答证明用户已经不再把 `effect/restart` 的 generation、插件进程代次和 ACP connection 代次当成同一个计数器。

## Mastered

- Plugin process generation 由 Plugin Lifecycle 拥有，描述一次 Deno `main.js` 进程生命。
- Agent connection generation 由 Connection Supervisor 拥有，描述一条完成 ACP initialize 的 Ready connection。
- Workspace Effect generation 由 Effect 系统拥有，描述一个 Workspace 的完整 Skill 期望状态版本。
- 三者可能因为重启策略联动变化，但数值不可比较，也不要求同步推进。

## Mastery status

第 23 课通过。下一步区分插件级 `agent/listModels` 与 ACP Session `configOptions` 两条模型发现路径。
