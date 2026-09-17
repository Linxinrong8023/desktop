# Skill 来源与 Agent Effect 消费者

## Learner question

用户提出：Skill 对每个 Agent 都有用，是否应该要求每个 Agent 插件都声明 Effect。
随后明确从 Ora 产品定位出发，认为每个官方 Agent 插件理论上都应声明 Skill surface；并提出支持热加载的 Claude Code 只需物化文件，不应重启。

## Verified answer

需要区分产品策略与底层插件契约：

- Skill 插件贡献的是 Ora 的声明式 Skill 来源或 Desired State，并不会自动知道每种 Agent 从哪里、何时以及以什么格式读取 Skill。
- Agent 插件只有声明 `effectSurfaces`，才表示自己是某个 Skill 文件表面的消费者，并告诉 Ora 投影路径、物化格式和变更协调策略。
- 如果官方产品要求所有官方 Agent 都能使用 Ora 安装的 Skill，那么每个支持 Skill 的官方 Agent 插件原则上都应接入 Effect；但这不是所有 Agent 插件都天然具备的能力。
- 第三方 Agent 可能完全不支持 Skill、通过 API/ACP 动态注入能力、使用非文件形式、或自行管理配置，因此底层 SDK 将 `effects` 设计为可选能力。

从 Ora 当前产品目标看，应进一步收紧表述：官方 Agent 若承诺能使用 Ora 管理的 Skill，就应声明至少一个 Effect surface；SDK 的可选性只是底层扩展边界和兼容空间，不代表官方插件可以漏接 Skill。

热加载能力决定的是 coordination，而不是要不要声明 surface：理论上，能安全观察文件变更的 Agent 可以声明 `uninterrupted`；只在启动或新建会话时解析 Skill 的适配器则需要 `wait_for_idle_and_restart`。

当前 Claude Code 官方文档说明，已经存在的 `.claude/skills` 目录会监听新增、修改和删除，但运行时首次创建顶层 Skill 目录仍需要重启。Ora 当前 Claude 插件源码虽然明确声明 `wait_for_idle_and_restart`，并声称适配器在创建会话时解析 Skill 目录，但继续核对新版 `claude-agent-acp` 后发现：它为每个 ACP Session 保存一个长期 Claude Agent SDK `Query`，使用 `user`、`project`、`local` setting sources，并把 SDK 的 `commands_changed` 事件作为 `available_commands_update` 转发给 ACP Client。由此可见，适配器本身并不必然阻断 Claude Code 的运行时 Skill 发现；当前 Ora 插件的重启策略更可能是保守兼容、保证 generation 边界，或沿用了旧行为假设，不能仅凭“使用中间适配器”证明重启必要。

真正仍需决策的是一致性语义：如果接受 Claude Code 的 watcher 在物化后自行收敛，可声明 `uninterrupted`，直接写文件且不重启；如果要求任何新 turn 都不得撞上一个尚未完整物化的多文件 generation，则仍需要 idle barrier，但未必需要重启。当前 coordination 只有 `uninterrupted` 和 `wait_for_idle_and_restart`，缺少“等待 idle、热加载后恢复但不替换进程”的独立策略，这是比 Claude 能否热加载更深的协议设计问题。

当前实现还有一个需要后续开发课讨论的接口问题：`AgentEffectDefinition` 对所有 coordination 都要求 `waitForIdle` 和 `restart`，Effect worker 在物化完成后也统一通过名为 `effect/restart` 的方法推进 consumer readiness。若未来真正接入 `uninterrupted` consumer，应考虑将“应用完成通知/恢复”与“实际重启”拆开，或让 `uninterrupted` 的 handler 明确只确认 generation 而不重启进程。

核心关系不是“安装一个 Skill 后广播给所有 Agent”，而是“Skill 形成 Desired State，Effect 系统只向主动声明兼容 Surface 的消费者投影”。

## Mastery status

已掌握产品层结论：要让每个官方 Agent 使用 Ora Skill，每个官方 Agent 插件都应声明自己的消费 surface；不同 Agent 的差别主要在 surface 路径和 coordination，而不是是否需要 Skill。

已能主动识别：文件直接物化到 Claude Code 的标准 Skill 路径，ACP adapter 只负责驱动 SDK，因此不能仅因存在 adapter 就推出需要重启。

待继续巩固：热加载能力与 generation 原子可见性是两个问题；后者可能需要 idle barrier，但不必然需要进程 restart。
