# 第 7 课：ACP Agent Runtime（完整）——三讲全部完成

> 状态：**已完成**（第一、二、三讲全部讲完）。
> 注：本课以 `main` 分支（5 个 CLI、warm 会话）为准；个别细节（prompt 路径的 30s 超时、title 获取链路）标注“待确认”。

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

## 九、会话生命周期完整故事（第三讲，全部完成）

### warm（预热）
- 打开聊天面 → warm_session → 在 CLI 上 `session/new`（**provider 侧创建**，发起是 client）；
- **不落库**（只活在内存 warm 池）；
- 用途：① 模型选择器（config_options 里的 model 选项——ACP 只在 session/new/load 回复里报告模型）；② 可用命令目录（available_commands）；③ 预热加速（attach 免重新握手）；④ 换绑时学习新模型列表的最后机会；
- **两个“创建”**：provider 侧（warm 时 session/new）vs Ora 数据库（attach 时）。

### attach（入住）
- 用户开始真正用 → attach_session：查 cwd → warm.take 认领 → 拿生命周期锁（串行）→ **写数据库**（Session 记录 Running）→ 开通道（**先注册路由**）→ **再创建 actor**（insert_actor）→ reservation.commit；
- **顺序讲究：先路由后 actor**——路由就位后消息有地方排队（信箱），actor 启动后一条不丢；注册前后两次代际检查（TOCTOU，防注册中途进程重启）；
- **认领机制**：take 预订 → 失败则 reservation drop（会话退回池子）→ commit 才正式生效；
- 每个角色 = 防一种乱子：cwd（跑错目录）、认领（没人接管）、落库（重启蒸发）、actor（消息没人处理/顺序乱）、路由（消息找不到主人）、commit（中途失败浪费 warm）。

### load（回房，恢复历史）
- run_load：① unload（清旧状态）→ ② 标记 Running（数据库）→ ③ 重建通道（open_session_channel → 注册新路由，新代新 token）→ ④ ACP session/load（provider 回放历史）；
- **两套历史**：Ora 的记录（transcript，前端显示用，在 ora-history 文件）vs provider 的记忆（Agent 工作上下文，在 provider 进程里）——load 要同时恢复两边；“Ora owns the transcript; the agent owns the model context”；
- **load ≠ 自动重发 prompt**：load 只恢复上下文，重不重试由用户决定。

### prompt（说话）
- 前置校验 **4 个**：① 空/全空白 → PromptEmpty；② 序列化 ≤16MiB → PromptTooLarge；③ 会话必须 Running → SessionStopped；④ 历史没 degraded（还能记录）→ 拒绝；
- 串行（lifecycle 锁 + actor 串行，同会话同时一个操作 → session_busy）；
- **30s 不活动超时**（SESSION_SETUP_TIMEOUT）：可重置闹钟 + select 循环——每条 update 重置闹钟，完全无声 30s 判死；thinking 块也是 update，正常思考永不超时；偏误杀“沉默的”，不挂死“真死的”；⚠️ 确认到 load 路径有此 deadline，prompt 路径待确认；
- **ContentBlock** = 一条消息的内容块（Text/Image/Audio/ResourceLink/EmbeddedResource），prompt = 有序 Vec<ContentBlock>。

### update 流 + 回合收尾
- update 三件事：① 观察（observe_session_update，title 等）→ ② **先记录**（record_update）→ ③ 转发前端；
- **记录在转发之前**：前端中途断开，不能连持久记录一起损失（“Record before forwarding”）；转发失败 → 取消回合（Cancelled）+ 隔离；
- **回合结束**：Agent 发 TurnEnded（带 StopReason）→ end_turn → 历史记“回合结束”+ StopReason；EndTurn/MaxTokens/Refusal/Cancelled；结束后 actor 回空闲、会话仍 Running。

### permission（要权限）
- **当前真实行为 = 自动允许**（无用户审批策略）：actor 收到 Permission → 记录 permissions 表 → pick_auto_allow_option 自动选“允许”选项 → 走和用户选择一样的 respond 路径；
- Agent 没提供“允许”选项 → 取消回合（Cancelled）+ 隔离；load 期间到达 → 自动取消（只有 prompt 能合法请求权限）；
- 前端弹窗/respond_to_permission 完整路径在契约层备好，未来审批策略启用时替换自动响应（设计先行、实现简化）。

### stop（暂停）
- stop_session → Stop 命令 → 取消在途 + 隔离通道（channel=None）+ 会话标记 Stopped；active 时 actor 结束，idle 时继续闲置；
- **会话保留**（历史在），可重新 load；**当前前端未接 stopSession**（Stopped 状态目前来自断线/溢出/workflow 结束）。

### delete（退房）
- delete_session：串行锁 → 查会话（不存在 → not_found）→ 停 actor（含 provider close）→ **软删数据库记录** → 从 actors 表移除 actor → **删历史文件**；
- delete ≠ stop：stop 保留（Stopped，可 load），delete 销毁（记录 + 历史全删）；
- **CLI 进程（共享）不受影响**——删单个会话绝不动 CLI 进程。

### 断线清理
- 进程死 → fail_generation（清路由 + 发 ConnectionLost）→ actor mark_stopped（Stopped + 丢通道，**继续闲置**）→ 重启 generation+1 → 用户 load 时**复用同一 actor** + 重建通道。

## 九·五、角色全景表（本模块所有角色，重点）

| 角色 | 代码 | 数量 | 归属 | 具体功能 | 数据流中的位置 |
|---|---|---|---|---|---|
| **Agent CLI 进程** | ora-process 拉起的子进程 | 每 CLI 1 个（5 个） | 外部 | 提供 AI 能力、执行 prompt、回 update | 消息源头（stdout） |
| **连接读取器** | AcpPeer + 读取循环 | 每连接 1 个 | ora-acp | 从 stdout 读 NDJSON 帧 → 交给路由器 | stdout → 路由器 |
| **监督器** | ConnectionSupervisor | 每 CLI 1 个 | backend | 管进程死活/重启/代际/能力协商；`open_session_channel` 建房间 | 生命周期 |
| **路由表** | RouteRegistry | 全局 1 个 | backend | `HashMap<provider session id, RouteEntry>`——按 id 分房；register/fail_generation/remove_route | 消息中枢 |
| **房间** | SessionChannel | 每会话 1 个 | backend | 连接 + 信箱（events 256）+ 火警（controls 无界）+ 路由凭证 | 路由 → actor 的通道 |
| **actor** | RuntimeActor（tokio 任务） | 每会话 1 个 | backend | 会话的串行管家：收命令（Load/Prompt/Stop…）、消费事件、观察/记录/转发、end_turn | 队列 → 前端/历史 |
| **actors 表** | ManagerInner.actors（HashMap） | 全局 1 个 | backend | session_id → actor 句柄；manager 显式增删 | 生命周期 |
| **会话记录** | Session（数据库） | 每会话 1 行 | ora-db | 持久档案（Running/Stopped、agent_session_id） | 生命周期 |
| **历史记录器** | SessionRecorder | 每会话 1 个 | backend | 对话 append-only 记录（ora-history，第 9 课） | actor → 文件 |
| **warm 池** | WarmSessions/WarmPool | 全局 1 个 | backend | 内存里的预热会话（不落库），take/claim/commit | attach/switch 的货架 |

**角色间的数据流（一条 update 的旅程）**：

```
Agent CLI stdout
  → 连接读取器（AcpPeer）
  → 路由器（route_event：按 provider session id 查 RouteRegistry）
  → 房间的信箱（events，有界 256）
  → actor（观察 → 先记录 → 转发）
      ├─ 历史记录器（ora-history 文件）
      └─ 前端（NDJSON 流）
控制流：ConnectionLost/QueueOverflow → 房间的火警（controls 无界）→ actor
命令流：manager → actors 表的句柄 → RuntimeCommand → actor
```

**生命周期管理流（谁删谁）**：

```
路由 ← token（正常摘牌/投递失败）/ generation（进程死批量）
actor ← manager（换 agent/删会话/恢复历史）显式 remove
CLI 进程 ← ora-process（ProcessTree 树级终止）
```

## 九·六、检查题答案（第一、二讲）

### 第一讲 6 题

1. **ACP 是什么**：JSON-RPC 2.0 + stdio（每行一个 JSON）；client=Ora、server=Agent CLI；
2. **initialize 作用**：握手协商协议版本 + 双方能力（load/close/delete/config 等）；
3. **为什么独立**：每台 CLI 一个监督器，一台挂了不影响其他；只有针对该 CLI 的操作报 agent_runtime_unavailable；
4. **退避策略**：250ms 翻倍封顶 30s；防疯狂重启 + 等晚装 CLI；
5. **连接丢失后**：在途失败 → 会话 Stopped → 进程树清理 → 重启（generation+1）→ 按需 load；不重放 prompt（副作用、历史在、用户决定、防死循环）；
6. **generation**：防旧进程消息被当新进程的（active_generation vs connection.generation）。

### 第二讲 6 题

1. **为什么 RouteRegistry**：一条连接多会话共享，消息要按 provider session id 分房，否则串房；
2. **SessionEvent 三类**：Update / Permission / Response，靠 session_id() 分房；
3. **events/controls 分开**：数据有界（可堆积，防内存爆炸）+ 控制无界（信号极少且"堵住了"必须还能送达）；
4. **setup 缓冲**：session/new 响应前 provider id 未知，update 先放等待室，注册时按 id 分发；
5. **generation 干什么**：进程代标记，fail_generation 按代批量清路由；token 单条删除防误删（正常摘牌/投递失败）；
6. **背压哲学**：有界队列 + 明确失败——不静默丢、不无限堆、不阻塞别人；宁可牺牲一个会话（可重载），不拖死整条连接。

## 十、检查题答案要点（第三讲 8 题）

1. warm 不落库（provider 侧，供模型选择）；attach 落库（Ora 侧正式会话，用户开始用时从 warm pool 认领）；
2. 先路由后 actor：路由先挂上，update 才有信箱可投，不丢失；
3. 两套历史：Ora 的记录（前端显示，ora-history）+ provider 的记忆（Agent 工作上下文）——数据库里没有 Agent 的记忆；
4. prompt 校验 4 个：空/全空白、≤16MiB、会话 Running、历史没 degraded；
5. 30s = 可重置闹钟 + select，每条 update 重置；thinking 也是 update，正常思考永不超时；
6. update 三件事：观察 → 先记录 → 转发；先记录防“前端断时记录不丢”；
7. delete 销毁（软删记录 + 删历史），stop 保留（Stopped，可 load）；CLI 进程（共享）都不受影响；
8. permission 当前 = 自动允许（无审批策略）；前端弹窗路径在契约层备好，未来启用。

## 十一、术语表新增

ACP（Agent Client Protocol）、ConnectionSupervisor（监督器）、Actor（会话管家）、Provider Session Id（provider 会话号）、RouteRegistry（路由表）、SessionChannel（房间通道）、控制线（controls，无界）、Generation（进程代）、Token（注册钥匙）、Backpressure（背压，已深化）、TOCTOU（检查-行动竞态）、StopReason（回合结束原因）、warm session（预热会话）、attach（挂靠/入住）、ProcessTree（进程树）、Job Object（Windows 进程组）。详见桌面 software technical terms.md。

## 十二、下一课预告

> 第 8 课（按 LESSON-PLAN）：Session 生命周期与 Warm Session 深入——warm 键（target, agent_cli, client_id）、attach 按 identifier 命名 vs 切换按 key 命名、Running/Stopped 状态、会话标题获取窗口、warm 只活在内存不落库、同会话操作串行。
