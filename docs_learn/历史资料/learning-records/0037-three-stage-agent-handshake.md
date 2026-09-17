# Agent Ready 前的三层合同与能力协商

## Learner response

用户能够分别说明：`ora/register` 确认插件提供的方法，`agent/start` 确认 Agent 已启动并约定通信协议，ACP `initialize` 确认协议与 Agent 具备的能力。用户也理解“声明可以收发 agent/acp”不等于真实端到端握手已经成功。

## Mastered

- 能区分外层 Plugin RPC 与内层 ACP 中的三次确认。
- 知道 `ora/register` 固定 Deno adapter 的 methods、emits 和可选 Effect surface 合同。
- 知道 `agent/start` 让插件启动真实 CLI，并返回 `protocol = acp` 与 `acpVersion = 1`。
- 知道 ACP `initialize` 才真正验证 Ora AcpPeer 到 Agent CLI 的往返，并返回 Agent ACP capabilities。
- 能解释三次确认不可互相替代，因此 Agent Ready 不能仅由 Deno Running 或注册成功推导。

## Precision corrections

1. `agent/start` 不重新定义 Ora 与插件之间的外层通信格式；外层始终是 Plugin RPC。它声明的是 `agent/acp` payload 的协议与 major version。
2. `ora/register` 只声明会收发 `agent/acp`，并不证明真实通道已经打通。
3. ACP `initialize` 返回的是 Agent capabilities，例如 session load/list/close/delete，不是 MCP 插件清单；会话前模型另由 `agent/listModels` 获取。

## Mastery status

第 21 课通过。Agent 插件覆盖审计中的四项 P0——故障决策、安全边界、对称 teardown、合同与能力协商——均已完成主动复述。
