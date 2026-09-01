# Ora Workflow 模块学习总结

> 目标：能够向别人完整讲清 Workflow 模块，能够依据持久化事实定位故障，并能应对面试中的架构、状态机、并发和恢复追问。

## 1. 一句话理解整个模块

用户在 Draft 中编辑一张 React Flow 图，发布时冻结成 Snapshot；创建 Run 时把某个 Snapshot 与 Workspace 绑定；调度器根据冻结 DAG 和已经落库的 NodeRun 计算下一批 Ready 节点；Agent 节点通过独立 Session 执行，执行结果写回 NodeRun，随后再次触发 DAG 推进，直到整张图成功、失败、取消或等待人工输入。

```text
Draft → Publish Snapshot → Create Run → Parse DAG
                                      ↓
                    NodeRun ← Schedule Ready Nodes
                       ↓                ↑
                 Agent Session → completion callback
```

## 2. 五个核心概念

### Workflow / Draft

- Workflow 是一套流程的稳定身份，包含名称和当前发布版本指针。
- Draft 是用户正在编辑的图，可以不完整，也可以随时修改。
- 当前实现中 Draft 也保存在 `workflow_snapshots` 中，但它与不可变发布版本的产品语义不同。
- 草稿保存只负责保存作者文档，不要求每次保存时都已经能够执行。

### Snapshot

- Snapshot 是某次发布时冻结的图版本，可以理解成工作流配置的不可变截图。
- 发布之后继续修改 Draft，不会改变已经发布的 Snapshot。
- Run 必须钉住具体 `snapshot_id`；排查历史 Run 时不能看当前草稿。

### Run

- Run 是“这份工作流的一次执行实体”，不是“工作流正在运行这一状态”。
- 同一个 Snapshot 连续执行三次，会创建三个不同 Run；Snapshot 本身不变。
- Run 保存 Workspace、Workflow、Snapshot、整体状态、输入、输出、错误、运行状态 JSON 和时间。

### NodeRun

- NodeRun 是某次 Run 中某个节点的一次真实执行尝试。
- 只有节点真正被调度时才懒创建 NodeRun；未调度节点没有占位记录。
- 因此“没有某节点的 NodeRun”通常表示它从未进入成功落库的 Ready wave，而不是数据丢失。

### Session

- Session 是 Agent 节点真实对话和 ACP 执行过程的载体。
- Start、Output 等控制节点不创建 Session。
- NodeRun 保存供调度和下游消费的状态与标量结果；完整对话历史属于 Session。
- Agent NodeRun 可能已经创建，但 `session_id` 仍为空，因为 Session 的公开绑定发生得更晚。

## 3. 数据库设计

四张核心表的关系：

```text
workflows
  └─ workflow_snapshots
       └─ workflow_runs
            └─ workflow_node_runs
                   └─ session_id? → sessions
```

### `workflows`

回答“这是哪一套工作流”。重要字段：

- `id`
- `namespace`
- `name`
- `published_snapshot_id`

`published_snapshot_id` 是当前发布版本的便利指针，不是历史 Run 的执行证据。

### `workflow_snapshots`

回答“这个版本的图是什么样”。重要字段：

- `id`
- `workflow_id`：外键指向 `workflows.id`
- `version`
- `graph`：完整 React Flow JSON

保存完整 JSON 而不立即拆成 nodes/edges 表，是因为这里首先需要保存、复制和版本化作者文档，包括节点、边和画布元数据。这样写入模型简单、版本复制完整；代价是数据库难以直接查询节点关系，结构校验必须交给应用层图解析器。

### `workflow_runs`

回答“哪一次执行发生了什么”。重要字段：

- `workflow_id`：外键指向 `workflows.id`
- `snapshot_id`：外键指向 `workflow_snapshots.id`
- `workspace_id`
- `run_status`
- `state`：JSON，其中包含 `current_nodes`
- `input` / `output` / `error` / `payload`
- `started_at` / `finished_at`

同时保存 `workflow_id` 和 `snapshot_id`：前者方便表达业务归属和按 Workflow 查询；后者精确确定实际运行的版本。创建 Run 时，应用层还会检查 Snapshot 确实属于该 Workflow。

### `workflow_node_runs`

回答“这次执行中的某个节点发生了什么”。重要字段：

- `run_id`：外键指向 `workflow_runs.id`
- `node_id` / `node_type`
- `status`
- `session_id`（可空）
- `input` / `output` / `error` / `payload`
- `started_at` / `finished_at`

`payload` 用于保存节点执行产生的结构化附加信息，例如 Agent 的 stop reason、文件变更等；它不是完整会话历史。

## 4. 图的保存、解析与验证边界

数据库主要保证字段和外键约束，不理解 React Flow 图的拓扑语义。一个有环或缺节点的 JSON 文档在定义存储阶段可以被保存。

后端必须再次解析和验证，原因包括：

- 请求可能通过导入或 API 进入，不一定经过前端画布校验。
- Draft 允许阶段性不完整，不能把“必须一次画完”的负担交给用户。
- 发布时可以提前检查结构，给用户更早反馈。
- Run 启动时还要重新检查，因为 Agent、模型、Role、Skill、插件和 Workspace 环境可能已经变化。

因此合理的职责划分是：

```text
前端：即时反馈和编辑体验
发布：结构与发布质量门禁
创建 Run：解析 Snapshot、绑定 Workflow/Snapshot/Workspace
启动 Run：最终可执行性检查
数据库：可靠保存，不负责理解 DAG
```

## 5. DAG 调度模型

调度器不是依赖一份只存在内存里的神秘进度。每次推进都会读取：

1. Run 钉住的冻结图；
2. 该 Run 已经存在的 NodeRuns；
3. 根据两者在内存中重新计算 `completed`、`in_flight` 和 `ready`。

Ready 的核心条件可以简化为：

```text
节点尚未成功
且节点没有正在执行或等待输入
且所有直接前驱均已成功
```

例如：

```text
        ┌─ Agent A ─┐
Start ──┤           ├─ Output
        └─ Agent B ─┘
```

只有 A、B 都成功，Output 才 Ready。A 成功而 B 仍 Running 时，Ready 为空，但 `in_flight` 不为空，所以 Run 不能结束。

### `current_nodes` 的真实作用

- 它保存在 `workflow_runs.state` JSON 中。
- 节点进入成功落库的执行 wave 时被加入，完成后移除。
- 它是当前执行/等待节点的持久化锚点，方便展示、取消和状态投影。
- 它不是 Ready 计算的权威来源；Ready 主要由冻结 DAG 和 NodeRun 状态计算。

当 `ready` 和 `in_flight` 都为空，图才真正耗尽，Run 可以完成。

## 6. 为什么要先写 NodeRun，再调用外部 Agent

同一波 Ready 节点会先在数据库事务中批量创建 Running NodeRuns，并更新 Run 的 `current_nodes`，事务提交后才派发外部 Agent。

这条顺序建立了重要不变量：任何已经发生的外部执行，都有一个数据库 NodeRun 可以归属。

如果反过来先调用 Agent：

- Agent 可能已经改文件或产生回复；
- 随后数据库写入失败；
- 系统会出现“外部执行发生过，但没有 NodeRun 记录”的不可审计状态。

NodeRun 的条件状态更新还承担幂等保护：重复或迟到回调只有在节点仍处于允许的前置状态时才能推进，否则成为 no-op。

## 7. Agent NodeRun 与 Session 生命周期

一条 Agent 节点的关键时间线：

```text
创建 Running NodeRun，session_id = NULL
  → 异步派发 Executor
  → 解析 Agent / Role / Skill / 模型配置
  → warm / attach Session
  → 组装 owning prompt
  → prompt_session() 接受 Prompt
  → 把 session_id 绑定到 NodeRun
  → 消费 ACP 事件流
  → 写回 succeeded / failed / pending
  → 再次运行 DAG 调度
```

需要特别注意：

- Warm Session 不会在预热时就永久绑定某个 Workflow Node。
- Warm Pool 按 warm key 复用准备好的能力；真正执行节点时才领取/attach。
- Ora 只有在 owning prompt 被接受后，才把 Session 公开绑定到 NodeRun。
- 因此 `session_id = NULL` 不能证明 `session/new` 从未发生，只能证明尚未越过公开绑定检查点。

### 配置和 Prompt 如何进入 Agent

Agent 节点可能包含节点提示词、Role、Skill、模型配置；运行还会提供 Run input 和上游输出。执行器把这些解析并组装成该 Session 的 owning prompt，再通过 ACP `session/prompt` 发送。

Skill 不是简单依赖当前 UI 配置。创建 Run 时，当前实现会把需要的 Skill 物化到 Agent 声明的 discovery roots，并把物化回执写入冻结的 Run payload；执行时读取回执，而不是重新扫描配置。Effect 是更通用的 Desired state 到外部 Surface 的协调机制，目前最明确落地的是 Skill directory。MCP、Role、Agent 插件等能力不能一概视为已经采用同样的冻结策略，必须区分当前实现与目标设计。

## 8. HITL、状态与输出

- 普通 Agent 完成后，NodeRun 进入 `Succeeded`，调度器继续推进。
- 交互 Agent 第一轮结束后可以进入 `Pending`，同时保留 `session_id`，表示等待用户继续输入，而不是“尚未调度”。
- HITL Pending 仍属于 `in_flight`，所以后继节点不会 Ready，Run 也不会完成。
- 用户继续交互并最终确认完成后，节点进入 `Succeeded`，后续推进与普通 DAG 完全一致。

`NodeRun.output` 是否有值取决于 output policy。Agent 成功但 output 为空，不等于 Agent 没有回复；完整回复仍可能存在于 Session 中，只是没有被选为供下游消费的节点输出。

Output 节点不是智能总结 Agent。它是控制节点，按现有策略汇总前驱输出。如果希望模型生成最终总结，应在 Output 前放一个真正的汇总 Agent。图可以没有 Output，但最终 Run output 的产品语义会依赖当前引擎的汇总策略。

## 9. 当前节点能力边界

当前可执行闭环主要支持：

- Start：把 Run input 带入图中，同步完成，不创建 Session。
- Agent：创建 Session，通过 ACP 执行。
- Output：汇总前驱结果，同步完成，不创建 Session。

必须区分两种失败：

1. 节点类型根本不在协议枚举中：图解析失败。
2. 类型能被解析，但当前执行引擎不支持：图能被识别，启动时因不可执行而拒绝。

Condition、Junction、分支激活边等属于尚未完整实现的设计议题。不能把设想中的 RoutePlanner、Active/Inactive 边或新数据库表描述成当前已经完成的功能。

## 10. 故障定位方法

### 第一步：锁定正确的 Run 与 Snapshot

```text
run_id
  → workflow_runs：run_status / snapshot_id / workspace_id / error / state
  → workflow_snapshots：当次真正执行的冻结图
  → workflow_node_runs：逐节点状态、错误、输出、session_id
  → sessions：仅在 session_id 已绑定时继续追 ACP 和完整对话
```

不要拿当前 Draft 解释历史故障。

### 第二步：根据最后一个持久化事实缩小范围

| 现象                                | 已经能够证明                         | 优先检查                                                  |
| ----------------------------------- | ------------------------------------ | --------------------------------------------------------- |
| Agent NodeRun 不存在                | 节点没有进入成功落库的 Ready wave    | 前驱状态、DAG、启动校验、调度事务                         |
| `Running + session_id NULL`         | 节点已调度，尚未越过公开绑定点       | Agent/模型配置、warm/attach、Role/Skill、Prompt admission |
| `Running + session_id 有值`         | owning prompt 已接受，Session 已绑定 | ACP 事件流、Agent CLI、stop reason、完成回调              |
| `Pending + session_id 有值`         | 通常是交互节点等待用户               | Frozen graph 的 interactive 配置、Session                 |
| `Failed`                            | 先读 NodeRun.error                   | 再用 session_id 区分绑定前还是绑定后                      |
| 前驱 Succeeded、后继 NodeRun 不存在 | 完成已落库但后续调度可能断档         | callback 后的 `run_schedule`、启动恢复                    |

日志用于补充过程证据；数据库用于确定已经成功越过哪些持久化检查点。

## 11. 启动调用链与同步/异步边界

```text
WorkflowRunWorkspace.handleStart
  → useStartWorkflowRun
  → Contract Client
  → Tauri command
  → Backend::start_workflow_run
       → per-run lock
  → Application WorkflowRunEngine::start
       → parse / validate frozen graph
       → repository.start_run
       → run_schedule
  → repository.start_ready_nodes
  → WorkflowRunNodeExecutor::dispatch
       → tokio::spawn(drive_agent_node)
  → 启动请求返回 Run = Running
```

因此“启动请求成功”只说明当前 Ready wave 已经持久化并派发，不保证 Agent 最终成功。模型、warm、Prompt、ACP 等错误可能在后台任务中稍后把 Run 改成 Failed。

## 12. per-run lock、数据库事务与幂等

- 数据库事务保证一组状态修改要么全部提交，要么全部回滚。
- per-run lock 保证同一个 `run_id` 的 start、cancel、restart、完成回调等操作按顺序进入状态机。
- 两者不是同一件事：事务只覆盖数据库内部原子性；锁还覆盖跨事务的读取、Ready 计算和外部派发编排。
- 条件更新负责最后一道幂等保护，例如 Cancelled NodeRun 收到迟到成功回调时不能再次变成 Succeeded。

## 13. Cancel、Restart 与 Boot Recovery

### Cancel

Cancel 在一个事务里修改：

```text
workflow_node_runs
  Pending / Running → Cancelled
  finished_at / updated_at → 当前时间

workflow_runs
  run_status → Cancelled
  finished_at / updated_at → 当前时间
  state.current_nodes → []
```

事务提交后，再 best-effort 停止已经绑定的 Sessions。先提交数据库终态，是为了避免“Session 已停止但 Run 仍显示 Running”。迟到回调看到节点已不是 Running/Pending，会成为 no-op。

### Restart

- 当前 Running 的 Run 不允许 Restart。
- Restart 不创建新 Run，而是复用同一个 `run_id`、`snapshot_id` 和 `workspace_id`。
- 旧的可见 NodeRuns 被软删除。
- Run 重置为 Pending，清空 output、error、时间与 `current_nodes`。
- 事务提交后立即从 Start 重新执行，并创建新 NodeRun ids。

若需要保留两次独立且同时可见的执行，应创建新的 Run，而不是 Restart。

### Boot Recovery

进程启动恢复根据数据库现场分三类处理：

1. 存在 Running NodeRun：外部 Agent 是否执行完、是否产生副作用无法确定，因此 Run 和非终态 NodeRuns 标记 `Failed`，错误为 `interrupted_by_restart`，不盲目重放。
2. A 已 Succeeded，但崩溃发生在创建后继 B 之前：保留 A，重新读取冻结 DAG 与 NodeRuns，重算 Ready 并调度 B。
3. 合法 HITL Pending：只有 interactive Agent、已有 `session_id` 且冻结图能证明其配置时才保留等待；其他无法解释的 Pending 记录 fail closed。

Restart 是用户主动“清账重跑”；Recovery 是系统自动“相信已提交事实并修复调度断档”，两者不能混为一谈。

## 14. 两分钟对外讲解模板

> Ora Workflow 把定义、版本和执行严格拆开。用户编辑的是 Draft，发布后形成不可变 Snapshot；每次执行创建独立 Run，并钉住 Snapshot 与 Workspace，所以后续修改草稿不会污染历史运行。运行引擎读取冻结 DAG 和 NodeRun 记录，在内存中计算 Ready，而不是依赖 `current_nodes` 决定调度。节点真正被调度时才创建 NodeRun，且必须先落库再调用外部 Agent，保证任何外部执行都有持久化归属。Agent 节点通过独立 Session 和 ACP 执行，Session 只有在 owning prompt 被接受后才绑定到 NodeRun。完成回调写回 NodeRun 后再次触发调度。排障时先定位 run_id 与 snapshot_id，再根据 NodeRun 是否存在、状态和 session_id 判断故障发生在调度、Session 绑定前还是 ACP 执行后。Cancel、Restart 和启动恢复分别负责终止本次执行、清账从头重跑，以及在进程崩溃后依据持久化事实安全恢复。

## 15. 当前学习边界

已经覆盖并形成稳定心智模型：

- Draft / Snapshot / Run / NodeRun / Session
- 四张核心表及故障查询路径
- DAG Ready 计算、并行 Join、懒创建与 `current_nodes`
- Agent Session 的 warm/attach/prompt/bind/callback 生命周期
- HITL、输出策略与当前节点能力
- 启动调用链与同步/异步边界
- 幂等、事务、per-run lock 的职责区别
- Cancel、Restart 与 Boot Recovery

仍适合在后续通过源码和事故题深化：

- 同一 Run 多操作竞态的完整时间线
- 更细的事务失败注入与恢复测试
- Condition/Junction/激活边的未来设计
- 前端对 Run/NodeRun 状态的完整投影
- 一套从 UI、SQLite、Backend 日志到 ACP trace 的实战排障演练

## 课程入口

- [第 1 课：一条 Run 的四层生命线](lessons/0001-one-run-four-layers.html)
- [第 2 课：从四张表重建一次 Run](lessons/0002-reconstruct-a-run-from-four-tables.html)
- [第 3 课：响应式 DAG 调度](lessons/0003-reactive-dag-scheduling.html)
- [第 4 课：Agent NodeRun 与 Session](lessons/0004-agent-node-session-lifecycle.html)
- [第 5 课：节点能力矩阵](lessons/0005-node-capability-matrix.html)
- [第 6 课：按持久化事实定位 Agent 故障](lessons/0006-locate-agent-failure-by-durable-fact.html)
- [第 7 课：启动调用链](lessons/0007-trace-start-across-layers.html)
- [第 8 课：Cancel、Restart 与启动恢复](lessons/0008-cancel-restart-and-boot-recovery.html)

配套速查资料位于 [`reference/`](reference/)。
