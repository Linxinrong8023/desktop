# 第 7 课：ACP Agent Runtime（进行中）——第一、二讲 + 第三讲（warm/attach）总结

> 状态：**未完成**。第一、二讲已讲完；第三讲（会话生命周期）已覆盖 warm、attach、断线清理，**load/prompt/stop/delete 剩余部分待续**。
> 注：本课以 `main` 分支（5 个 CLI、warm 会话）为准。

## 〇、第 7 课是什么（与前面课程的分界）

前 6 课 = **Ora 自己的世界**（CRUD、数据库、Git）——Ora 完全可控、一问一答。
第 7 课起 = **外部世界**（Agent 子进程）——双向、异步、流式、进程不可控。

第 7 课是前 6 课的最大汇合点：契约、错误体系、生命周期、worktree cwd 全部在此收拢。

## 一、ACP 协议（第一讲）

- **ACP = Agent Client Protocol v1**：基于 JSON-RPC 2.0，走 **stdio**（子进程 stdin/stdout），每行一个 JSON（NDJSON）；
- 两个角色：**client = Ora**，**server = Agent CLI**（opencode/nga/codeagentcli/claude/codex，5 台各一进程）；
- 关键操作：`initialize`（握手协商能力）→ `session/new`（开房间）→ `session/load`（恢复）→ `session/prompt`（发消息）→ `session/update`（Agent 流式回）→ `request_permission`（Agent 要权限）→ stop/close/delete；
- **两个 id 别混**：`request_id`（JSON-RPC，一次请求↔响应的配对）；`provider session id`（ACP，一条消息属于哪个会话房间的寻址）；
- **ACP 支持是硬门槛**：启动时 `initialize` 握手"验货"（15s 超时），验过才 Ready，验不过 Unavailable（只影响自己）；验过后还有第二层：能力协商（load/close/delete 支持吗，逐个广告、逐个降级）。

## 二、进程监督（ConnectionSupervisor，第一讲）

- **每台 CLI 一个监督器，完全独立**：一台挂了不影响其他；
- 启动即拉起（eager），状态机 **Starting → Ready / Unavailable**；
- 重试：**250ms 翻倍封顶 30s**（防疯狂重启 + 等晚装 CLI）；
- 连接丢失：在途操作失败 → 该 CLI 会话标记 Stopped → **进程树清理** → 重启（generation+1）→ 会话按需 load；
- **不自动重放 prompt**：有副作用、历史已存、用户决定、防死循环；
- **进程管理在 ora-process**：`TokioProcessSpawner` 拉起、`ProcessTree` 树级终止（Unix 进程组 `kill(-pgid)` / Windows Job Object `TerminateJobObject`）——防止 Agent 拉起的孙进程变孤儿。

## 三、路由（第二讲）

- **问题**：一条 stdout 上多个会话消息混在一起 → 必须**按 provider session id 分房**；
- `RouteRegistry` = `HashMap<provider session id, RouteEntry>`（门牌号表）；
- `SessionEvent` 三类：**Update / Permission / Response**，都有 session_id() 用于分房；
- 每会话两个通道：**events（有界 256）** + **controls（无界）**——普通消息走邮箱，生死信号（ConnectionLost/QueueOverflow）走火警专线；
- **setup 缓冲**：session/new 响应回来前（provider id 未知），update 先放等待室（pending_setup_updates），注册时按 id 分发；
- **TOCTOU 竞态**：`route_event` 和 `register` 锁同一个 mutex——"查表+决定"必须原子，否则 update 错过投递被清掉（"看的时候没有，决定时有了"）。

## 四、generation vs token（易混，重点）

| | generation（代） | token（钥匙） |
|---|---|---|
| 粒度 | **进程级**（粗） | **注册级**（细，每次注册唯一） |
| 防什么 | 旧进程消息串入新进程 | 旧 actor 误删新路由 / 旧通道 Drop 误删新注册 |
| 在哪核对 | `active_generation` vs `connection.generation` | `entry.token` vs 注销时带的 token |
| 批量作废 | `fail_generation(g)`（按代全删 + 发 ConnectionLost） | — |
| 单条删除 | — | `remove_route(session_id, token)`（token 匹配才删） |

**为什么单条删除必须比 token 不能比 generation**：同代内也可能替换（register 无条件覆盖旧条目），generation 相同分不出新旧，token 永远分得出。

## 五、路由删除三路径（钉死）

| 路径 | 触发时机 | 依据 | 防什么 |
|---|---|---|---|
| 1 正常摘牌 | 通道被丢（stop/delete/switch/mark_stopped）→ RouteRegistration::drop | **token** | 旧通道 Drop 误删新路由 |
| 2 异常清理 | 进程死 | **generation**（fail_generation） | 死进程路由残留、新代不受影响 |
| 3 投递失败 | 队列满/通道关（try_send 失败） | **token** | 只删自己投递失败的那条 |

`fail_generation` 的实现模式："搬空表（std::mem::take）→ 按 entry.generation 分拣（partition）→ 留的放回、删的通知（发 ConnectionLost）"——因为 Rust 不能边遍历边改 HashMap。

## 六、背压与超时（第二讲）

**背压 = 有界队列 + 明确失败**：满了不静默丢、不无限堆、不阻塞别人。

- 队列满 → try_send 失败 → 删路由 + 控制线发 QueueOverflow → actor 把 `agent_event_overflow` 错误帧发给前端 → 隔离会话 → **队列里没消费的 256 条随通道丢弃**；
- **保住 vs 丢掉**："已入账的"（已观察已记录的 prompt+update）load 能恢复；"没入账的"（队列里的）丢弃——因为 load 从历史账本重建，不从队列恢复；
- **load ≠ 自动重发 prompt**：load 只恢复上下文，重不重试由用户决定。

超时数字：initialize 15s / setup 30s / 非活动 30s / 取消宽限 5s / 队列 256 / 帧 8MiB / prompt 16MiB / 退避 250ms→30s。

## 七、actor 是什么（深入追问）

- **actor = 每会话一个的"串行管家"**（tokio::spawn 任务）：拥有会话状态、cwd、连接、通道、recorder，通过 commands 通道收命令、events 收事件，**一次处理一条**（同会话串行，不同会话并行）；
- **actor vs session**：actor = 内存里的活化身（可创建/销毁多次）；session = 数据库里的持久档案（永久，直到删除）；
- **进程死了 actor 不清理、复用**：mark_stopped（Stopped + 丢通道）后继续闲置，load 时**同一个 actor 重建通道**恢复；actor 真正结束只有三个场景：换 agent、删会话/恢复历史（manager 显式移除）、Backend 关停；
- **换 agent = 换 actor**：actor 的 `connection` 字段绑定 CLI，换 CLI 必须重建 actor；同 CLI 进程重启则复用。

## 八、Cancel vs Stop（易混，重点）

| | Cancel（取消回合） | Stop（停止会话） |
|---|---|---|
| 影响级别 | 回合 | 会话 |
| 用户操作 | 点"停止生成"按钮（中止流 → session/cancel） | （当前前端未接 stopSession） |
| 结束后状态 | 会话还是 Running | 会话变 Stopped |
| 对应 | `session/cancel` → ACP `StopReason::Cancelled` | `session/stop` → `SessionStatus::Stopped` |

**三个概念别混**：`SessionStatus`（Running/Stopped，会话级持久状态）、`StopReason`（EndTurn/MaxTokens/Refusal/Cancelled，回合级结束原因，ACP 定义）、`RuntimeCommand`（Cancel/Stop，操作命令）。
**现状**：前端没有调用 stopSession 的代码；Stopped 状态目前来自断线、溢出、workflow run 结束（内部场景）。

## 九、会话生命周期（第三讲，已完成 warm + attach）

### warm（预热，已讲）
- 打开聊天面 → warm_session → 在 CLI 上 `session/new`（**provider 侧创建**，发起是 client）；
- **不落库**（只活在内存 warm 池）；
- 用途：① 模型选择器（config_options 里的 model 选项——ACP 只在 session/new/load 回复里报告模型）；② 可用命令目录（available_commands）；③ 预热加速（attach 免重新握手）；④ 换绑时学习新模型列表的最后机会；
- **两个"创建"**：provider 侧（warm 时 session/new）vs Ora 数据库（attach 时）。

### attach（入住，已讲）
- 用户开始真正用 → attach_session：查 cwd → warm.take 认领 → 拿生命周期锁（串行）→ **写数据库**（Session 记录 Running）→ 开通道（**先注册路由**）→ **再创建 actor**（insert_actor）→ reservation.commit；
- **顺序讲究：先路由后 actor**——路由就位后消息有地方排队（信箱），actor 启动后一条不丢；注册前后两次代际检查（TOCTOU，防注册中途进程重启）；
- **认领机制**：take 预订 → 失败则 reservation drop（会话退回池子）→ commit 才正式生效；
- 每个角色 = 防一种乱子：cwd（跑错目录）、认领（没人接管）、落库（重启蒸发）、actor（消息没人处理/顺序乱）、路由（消息找不到主人）、commit（中途失败浪费 warm）。

### 断线清理（已讲）
- 进程死 → fail_generation（清路由 + 发 ConnectionLost）→ actor mark_stopped（Stopped + 丢通道，继续闲置）→ 重启 generation+1 → 用户 load 时复用 actor + 重建通道。

### 未完成：load / prompt / stop / delete 的详细故事（第三讲剩余）

## 十、术语表新增

ACP（Agent Client Protocol）、ConnectionSupervisor（监督器）、Actor（会话管家）、Provider Session Id（provider 会话号）、RouteRegistry（路由表）、SessionChannel（房间通道）、控制线（controls，无界）、Generation（进程代）、Token（注册钥匙）、Backpressure（背压，已深化）、TOCTOU（检查-行动竞态）、StopReason（回合结束原因）、warm session（预热会话）、attach（挂靠/入住）、ProcessTree（进程树）、Job Object（Windows 进程组）。详见桌面 software technical terms.md。

## 十一、下一讲预告

> 第三讲剩余：load（回房恢复历史）→ prompt（说话，校验/串行/超时）→ update 流（路由分房→队列→观察记录转发前端）→ permission → delete（退房）。把 warm → attach → load → prompt → stop → delete 串成"会话的一天"完整装配线。
