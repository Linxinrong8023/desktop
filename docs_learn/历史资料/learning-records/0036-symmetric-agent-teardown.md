# Agent connection generation 的对称 teardown

## Learner response

用户最初把停止理解为统一的强制 kill，因此质疑 Agent CLI 归 Ora 管理后为何还需要 `agent/stop`。经过多轮追问，用户最终准确复述：不同 Agent 有自己的停止前置动作，插件负责告知/编排专属停止策略，Ora Host 持有真实进程并执行通用 OS 操作与最终兜底。用户还主动纠正了 `agent/stop` 与 `ora/shutdown` 的层级混淆。

## Mastered

- 能区分 Agent CLI 进程与 Deno `main.js` 插件进程。
- 能区分 `agent/stop`：停止真实 Agent CLI；`ora/shutdown`：停止 Deno 插件进程。
- 能说明 `agent/stop` 是业务控制入口，插件 handler 通过 generation-scoped `processId` 请求 Host 执行 close/kill，而不是直接持有 OS process handle。
- 能解释为什么不一开始统一强杀：不同 CLI 的优雅退出语义属于 adapter，硬编码进 Ora 会重新退化成内置 Agent。
- 能区分 exit、kill 和 wait/reap：正常退出后无需再 kill，但 Host 仍需确认并回收；强杀只兜底仍存活的进程。
- 能说明替代 generation 必须等待旧进程树真正退出，generation 标签本身不能阻止旧进程继续产生副作用。

## Precision corrections

1. Host 不是每次都会强杀整棵树；它只对仍存活的进程强杀，但始终需要等待并回收退出状态。
2. 正式的 `ora/shutdown` 不是 Agent CLI 的专属退出指令，而是 Plugin Runtime/Lifecycle 停止 Deno 插件的通用协议。
3. `agent/stop` handler 可以关闭 CLI stdin、发送 CLI 私有退出指令或请求 Host kill；最终 OS 操作仍由 Host 执行。

## Mastery status

第 20 课通过。用户已从“停止就是 kill”建立到“adapter 负责停止策略、Host 负责执行机制与最终保证”的分层模型。
