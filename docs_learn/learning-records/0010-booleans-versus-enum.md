# 正在理解“可实现”与“非法状态不可表示”的区别

## Learner question

用户继续追问：如果缺少 `main.js` 时不能得到 Agent contribution，这与 `is_agent / is_skill` 形式究竟有什么关系，为什么布尔形式不行。

## Interpretation

教学中把布尔/可选字段方案说成“不行”过于绝对。该方案可以实现相同功能，只是允许矛盾状态进入内存，必须依赖运行时校验和调用者纪律。用户需要理解的是可靠性差异，而不是语法可行性。

## Clarification

- 外部 TOML 已由唯一 `kind` 避免多类型声明；布尔反例讨论的是解析后的 Rust 内部建模。
- `is_agent/is_skill + Option data` 允许两者同时为 true、Agent 为 true 但数据缺失、Agent 为 false 却携带 Agent 数据。
- `kind: PluginKind + Option data` 虽避免多个标签，也仍允许 `kind = Agent` 与 `agent_data = None`。
- `PluginContribution::Agent(AgentData)` 把标签和必需数据原子地绑定；编译器拒绝缺失数据和多 variant 状态。

## Mastery status

已掌握。用户先理解 enum 保证 Agent、Skill 等 variant 唯一且互斥，随后确认 Agent variant 必须携带 `InstalledPluginAgent` 及已验证 entrypoint，否则无法构造。“可信”仅限验证过的结构与宿主规则，而非插件代码绝对安全或磁盘永不变化。
