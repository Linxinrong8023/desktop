# Runtime 与 Supervisor 的理解缺口

## Learner response

用户认为 Agent 插件需要 Runtime，是因为它需要运行 JavaScript 脚本并形成进程；但无法清晰描述 Supervisor 的职责，请求教师详细回答。

## Correction

- “需要运行 JavaScript”是当前实现方式，不是根本原因。根本原因是 Agent 插件提供长期动态服务：它需要持续通信、保存连接状态、处理并发 Session，并面对进程退出与协议故障。
- Plugin Runtime 负责让 Deno `main.js` 运行并承载外层 Plugin JSON-RPC；Agent Runtime 在此基础上建立 ACP Peer 和可供 Session 使用的 Agent connection。
- Connection Supervisor 负责建立并维持 Agent connection：请求 Lifecycle `ensure_running()`、校验 Agent 合同、调用 `agent/start`、完成 ACP `initialize`、发布连接状态、绑定 generation，并在连接丢失后退避重试或熔断。
- Plugin Lifecycle 仍是 Deno 插件进程的唯一所有者；Supervisor 监督 Agent 服务的可用性，不取代 Lifecycle，也不取代每个 Session actor。
- Skill 插件贡献静态文件与声明，由 Host 校验、索引和物化；它没有自己的长期进程、协议连接或 Ready/掉线状态，因此不需要 Supervisor。

## Mastery status

已通过两句话口述。用户能够说明：Lifecycle 管插件进程，而 Supervisor 请求、确认和维持 Agent connection，并负责故障重试。

仍需保持一个术语精度：Lifecycle 不只是“启动”插件，它是 Deno 插件进程整个生命周期的唯一所有者，包含启动、停止、状态、generation 替换和回收；Supervisor 管的是建立在其上的 Agent 服务可用性。
