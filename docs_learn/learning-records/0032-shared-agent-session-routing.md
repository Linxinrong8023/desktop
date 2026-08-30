# 共享 Agent connection 的 Session 路由

## Learner request

用户在掌握 Supervisor 的连接维护职责后，主动指出一个 Agent 可以同时服务多个 Session，需要继续学习如何准确路由而不串线。

用户随后提出自己的模型：可能存在 `agent_session_id → ora_session_id` 的 HashMap，并猜测每条 Agent 回复都携带 Agent Session ID；同时追问普通 response 的 session identity 如何由 pending correlation 恢复。

## Verified model

- 每个 Agent 有一个 application-scoped Supervisor 和共享 RuntimeConnection；多个 Session actor 可以并发复用它。
- Ora 自己的 `ora_session_id` 是持久业务身份；Agent 在 `session/new` 时返回的 `agent_session_id` 是 ACP 侧地址，load 时复用已有值。中心 `RouteRegistry` 以 `agent_session_id` 为键。
- 每个 Session actor 持有独立 `SessionChannel`，其中正常事件使用容量 256 的有界 FIFO，连接丢失与队列溢出使用独立控制通道。
- `session/update` 和权限请求自身携带 Agent Session ID；普通 response 只有 request ID，`AcpPeer` 用 pending correlation 恢复其 Agent Session ID，再交给 RouteRegistry。
- 主路由不是依赖一张 `agent_session_id → ora_session_id` 表完成，而是两阶段：`PendingRequests` 保存 `request_id → agent_session_id`，`RouteRegistry` 保存 `agent_session_id → SessionChannel`。另有 `SessionTraceRegistry` 保存 Agent ID 到 Ora ID 的映射，但主要服务日志追踪。
- generation 使旧 connection 的路由整体失效；route token 防止旧 actor 析构时误删新注册；setup buffer 保存 `session/new` 返回 ID 之前提前到达的 update。
- 单一 Session 队列溢出只移除该路由，不终止共享 Agent connection，也不影响其他 Session。

## Mastery status

用户已掌握两阶段 correlation：发送 session request 前记录 `request_id → agent_session_id`；普通 response 只回显 request ID 时先恢复 Agent Session，而 notification 可直接使用自带的 Agent Session ID。

用户进一步指出：找到 SessionChannel 就找到了专门负责该 Ora Session 的 actor，因此从概念模型上说“Agent Session 映射到 Ora 会话消费者”已经足够解释为什么不会串线。该理解判定为掌握。

用户随后主动推导出 Channel 代表当前 Agent 绑定：切换 Agent 时 Ora Session ID 保持不变，但目标 Supervisor、provider `agent_session_id`、connection generation 和 SessionChannel 都会替换。该推导正确。补充范围：同一 Agent 断线重连或 stopped session 重新 load 时，也需要重新建立 generation-bound Channel，因此 Channel 不只服务 Agent 切换。

源码精度作为补充而非纠错门槛：正常分发实际使用 `RouteRegistry: agent_session_id → SessionChannel`；`SessionTraceRegistry: agent_session_id → ora_session_id` 主要服务日志追踪。直接保存 channel 可减少一次二级查找，并把当前 actor 实例、队列、generation 和 route token 绑定在同一个 live route 中。
