# 已掌握 Agent 插件双进程基本职责

## Learner response

用户说明 Agent 插件的 `main.js` 负责拉起、停止真实 Agent，列出模型列表并转发 ACP；Agent CLI 处理 ACP 消息和用户需求。用户也指出，如果把每种 CLI 的逻辑重新写进 Backend，就退化回内置 Agent，违背插件化目的。

## Demonstrated understanding

- 能区分 Deno 中的适配器进程与真实 Agent CLI 进程。
- 掌握 Agent 插件 v1 核心合同：start、stop、listModels、双向 agent/acp。
- 理解 Agent CLI 才负责模型上下文、工具和用户请求处理。
- 理解把 CLI 差异移出 Backend 是 Agent 插件化的核心价值。

## Precision added

- Agent 插件还可以声明可选 Effect surface，并实现 waitForIdle/restart 协调。
- `agent/start`、`agent/stop`、`agent/listModels` 是插件 JSON-RPC 控制调用，不是 ACP 消息。
- ACP payload 在 Ora 与插件间装入 `agent/acp` notification；插件再通过 CLI stdio 转发。两段承载方式不同，业务协议仍是 ACP。

## Mastery status

双进程基本模型已掌握。下一检查点：区分外层 Plugin JSON-RPC 与内层 ACP，以及控制调用和数据流为何分离。
