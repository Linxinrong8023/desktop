# Ora 插件系统学习资源

## Knowledge

- [`ora-plugin-manifest` README](../crates/plugin-manifest/README.md)
  Manifest schema 与语义校验的权威边界。用于回答“插件声明了什么、哪些组合在解析期就不可能出现”。
- [`ora-plugin-manager` README](../crates/plugin-manager/README.md)
  安装包发现、磁盘验证和 `PluginContribution` 建模的主资料。用于理解“包”怎样变成可信的已安装贡献。
- [`ora-plugin-runtime` README](../crates/plugin-runtime/README.md)
  Deno 插件进程、二进制帧、JSON-RPC 双向通信和不可变能力注册的权威说明。
- [`ora-plugin-lifecycle` README](../crates/plugin-lifecycle/README.md)
  插件控制面与数据面的核心资料。用于理解唯一进程所有者、按需启动、generation、storage 与 child-process host。
- [Agent 插件适配层 README](../crates/backend/src/agent_runtime/plugin_agent/README.md)
  Agent 插件怎样被适配成上层无感知的 `RuntimeConnection`，以及 Agent 契约、ACP 转发和失败分类。
- [ACP Agent Runtime 设计](../docs/agent-runtime.md)
  Agent Supervisor、Session actor、路由与故障隔离的系统级上下文。
- [Plugin SDK README](../packages/plugin-sdk/README.md)
  插件作者视角的进程协议、Host request、Agent helper 和子进程管理接口。
- [Plugin Surface 设计](../docs/surface.md)
  Workbench 与 Webview 两种 UI contribution 的隔离、授权、导航和下载边界。
- [`ora-plugin-config` README](../crates/plugin-config/README.md)
  不可变设置声明与可变存储值，以及 MCP/Hook configuration shape 的边界。
- [当前分支源码](../crates/plugin-manager/src/validation.rs)
  最终裁决依据。README 与代码冲突时，以当前分支实现和测试为准。

## Wisdom (Communities)

- 本仓库的插件相关 Git 历史与代码评审
  用于理解设计为何从内置 Agent 演进到插件化，以及每个边界解决过什么真实问题。
- Ora 团队的架构评审与面试演练
  用于检验表达能否经受追问；课程后段会把设计解释压缩成白板叙述并进行反向质询。

## Gaps

- MCP 插件目前完成安装期编译与设置编辑，但 `ResolvedMcp`、Agent 物化和 Workspace 选择仍是后续切片；课程必须区分当前实现与目标设计。
- `docs/desktop-runtime.md` 仍残留已删除的 `ui` 插件表述；Surface 课程以 `docs/surface.md`、crate README 和当前代码为准。
- `docs/effect-skill-state.md` 的“没有真实 Agent Runtime 集成”已经落后于当前 Effect worker 与 Agent coordination 实现。
