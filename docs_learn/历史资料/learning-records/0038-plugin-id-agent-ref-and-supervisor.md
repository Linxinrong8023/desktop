# PluginId、AgentRef 与 Supervisor 是两个不同问题

用户先质疑“为什么 Session 不直接保存完整 PluginId”，随后准确区分：直接保存 PluginId 技术上可以省掉 AgentRef 到插件来源的身份转换，但不能省掉 Supervisor；Supervisor 请求 Plugin Lifecycle 启动插件只是其职责之一，它还负责建立、确认和维持 Agent connection，以及故障重试等完整运行时监督。这个理解避免把 Supervisor 误认为单纯的 ID 转换层。

## Mastered

- 能说出 Lifecycle 使用完整 `PluginId = official/ora-space.opencode`，Session 当前保存 `AgentRef = ora-space.opencode`。
- 知道当前实现不是让 Session 手工拼接 `official/`，而是在插件发现时用 InstalledPlugin 构造同时包含两种身份的 AgentSource，并以 AgentRef 索引 Supervisor。
- 能评价替代设计：Session 直接保存 PluginId 是可行方案，可以降低身份复杂度，但需要改变既有持久化合同并让会话绑定插件安装地址。
- 能说明无论选择哪种身份设计，Supervisor 的连接初始化、Ready 状态、重试、generation 和共享服务职责都不能被 Lifecycle 替代。

## Mastery status

第 22 课通过。下一步区分 Plugin process generation、Agent connection generation 与 Effect generation。
