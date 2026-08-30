# Agent 插件主线口述验收

## Learner response

用户从插件安装校验开始，依次串起 Manifest、Manager、InstalledPlugin、Supervisor、Plugin Lifecycle、Deno `main.js`、运行时注册、`agent/start`、Host Processes 拉起 Agent CLI，以及 Prompt 经 `agent/acp`、Plugin Runtime、ACP Peer、Session actor 和前端返回的完整链路。

## Mastered

- 能区分 Manifest 声明校验与 Manager kind-specific 包内容校验，并知道它们不证明插件运行逻辑绝对正确。
- 能说明 `InstalledPlugin` 是验证后供宿主消费的可信视图。
- 能说明 Supervisor 请求 Lifecycle 启动插件，而不是自行拥有插件进程。
- 能说明 Deno 插件进程与真实 Agent CLI 是两个进程，插件通过 Host Processes 请求 Ora 创建 CLI。
- 能说明 `agent/acp` 使用 notification 携带完整 ACP 消息，返回方向经过 Plugin Runtime、ACP Peer、Session actor，最终形成前端可渲染事件。

## Precision corrections

1. Supervisor 不是 Installer 直接生成的安装产物，而是 Ora 运行期的内存对象；启动时为已安装 Agent 插件创建，运行中安装后通过 `sync_plugin_agents()` 补建。
2. 插件运行时注册发生在 Deno Plugin Runtime 握手阶段，Ora 校验合同后才调用 `agent/start`。
3. `agent/start` 成功只表示插件声明 Agent CLI 已可接收 ACP；Supervisor 随后还会创建 `AcpPeer` 并完成 ACP `initialize`，成功后才发布 `Ready`。
4. `main.js` 通常不理解或转换 ACP 业务语义，只拆取外层 `agent/acp` payload 后透明转发，并在返回时重新包成 notification；内层 ACP 的 ID、响应与事件由 `AcpPeer` 解析和关联。

## Mastery status

Agent 插件从安装到对话渲染的主线口述验收通过。下一阶段可以进入 Agent、Skill、MCP、Webview 四类插件的静态贡献与运行方式对比。
