# 第 8 课：Session 生命周期与 Warm Session 深入（总结）

> 对应对话内容：warm 池内部、WarmKey、gates 门锁、take vs claim、三个标识、前端真实流程、标题获取窗口，以及 Agent Runtime 全景（数据流与调用关系）。
> 注：第 7 课讲了 warm/attach 的"用途"，本课深入 warm 池"内部怎么管"。

## 〇、角色总览（谁是谁）

| 角色 | 文件 | 数量 | 一句话 |
|---|---|---|---|
| Agent CLI 进程 | （外部） | 每 CLI 1 个 | 消息源头（stdout）、干活的人 |
| 连接读取器 | ora-acp / connection.rs | 每连接 1 个 | 从 stdout 读 NDJSON 帧 |
| 监督器 | connection.rs ConnectionSupervisor | 每 CLI 1 个 | 管进程死活/重启/代际/握手 |
| 路由表 | routing.rs RouteRegistry | 全局 1 个 | 消息分房（provider id → 信箱） |
| 房间 | routing.rs SessionChannel | 每会话 1 个 | 信箱(256) + 火警(无界) + 路由凭证 |
| manager | mod.rs AgentRuntimeManager | 全局 1 个 | 会话操作入口（warm/attach/load/prompt...） |
| actors 表 | mod.rs HashMap | 全局 1 个 | session id → actor 句柄 |
| actor | actor.rs RuntimeActor | 每会话 1 个 | 串行管家（收命令/消费事件/记录转发） |
| warm 池 | warm.rs WarmSessions | 全局 1 个 | 内存预热会话（take/claim/门锁） |
| 会话记录 | ora-db | 每会话 1 行 | 持久档案（含 provider session id） |
| 历史记录器 | actor.rs SessionRecorder | 每会话 1 个 | 对话 append-only 记录 |

## 一、warm 池内部

### WarmSessions 三个字段

```rust
pub(super) struct WarmSessions {
    pool: StdMutex<WarmPool>,                            // ① 装着所有 warm 会话的池子
    gates: StdMutex<HashMap<WarmKey, Arc<Mutex<()>>>>,  // ② 每把钥匙（WarmKey）一把门锁
    connections: ConnectionSupervisors,                  // ③ 认识各台 CLI
    clock: SystemClock,
}
```

### WarmAttachment（warm 会话"带的东西"）

agent_cli（哪台 CLI）、agent_session_id（provider 房间号）、cwd（干活目录）、available_commands（命令）、config_options（模型等配置）。

### WarmReservation（认领的"房卡"，重点）

- **为什么做成"对象 + Drop"而不是两个函数调用**：调用方可能永远不调第二个调用（HTTP 断线丢 future、panic 跳过）——**对象被 drop 时 Drop 自动"退回池子"**，绝不泄漏；
- **take 取出（预订）→ commit 生效 / drop 自动退回**；
- 类比：房卡一放手就自动弹回前台，不用客人自觉。

## 二、WarmKey（warm 会话的"身份证"）

```rust
pub(super) struct WarmKey {
    pub target: WarmSessionTarget,  // 为谁服务（任务 / 项目）
    pub agent_cli: AgentCli,        // 用哪台 CLI
    pub client_id: String,          // 哪个聊天面（客户端）
}
```

- **三合一**：为谁干活 + 用哪家 + 谁订的，三个都对上才是一个 warm 会话；
- **client_id 不能省**：同一个任务开两个标签页 = 两个独立会话，不区分会抢同一个会话（第一个 attach 的拿走另一个的对话）；
- 类比：预约房的"预订人信息"。

## 三、gates 门锁（防重复预热）

- **问题**：同一聊天面被"打开"两次（React 开发模式 double-mount）→ 两个 warm 请求都去 session/new → 创建两个会话，一个变孤儿；
- **解法**：每个 WarmKey 一把门锁——同一 key 的 warm 请求排队，第二个发现"已经有了"直接用，不再创建；
- 类比：两人同时订同一间房 → 有锁则第二个排队，发现已订好用现成的。

## 四、take vs claim（两种取会话）

| | take | claim |
|---|---|---|
| 谁用 | attach（用户开始用会话） | switch_agent（换 CLI） |
| 靠什么找 | **Ora session id**（报编号） | **WarmKey**（报预订信息） |
| 调用者手里有什么 | 有 warm 会话的 id（warm 时返回的） | 没有（选择器预热的 id 在池子内部，没给过客户端） |

- **为什么 switch 用 key**：选择器预热 claude 时没给 id（那个会话只是给选择器看模型的，编号是池子内部的）→ 换绑只能报"预订信息"（任务+claude+聊天面）——**选择器用 key 预热，换绑用同一 key 认领，两头对上**；
- claim 更宽容：key 对应的会话没了 → 现场新建，不报错（"调用者不会仅仅因为预热的没了而失败"）。

## 五、三个标识（易混，重点）

| 名称 | 谁发的 | 是什么 | 切换 agent 时 |
|---|---|---|---|
| Ora session id | Ora 自己 | 会话在 Ora 数据库的身份证 | **不变**（对话保留） |
| provider session id（agent_session_id） | Agent/CLI | 会话在 CLI 进程里的房间号 | **变**（opencode 的 → claude 的） |
| warmkey | —— | 不是 id，找预热会话的预订信息 | 用来找 claude 的预热会话 |

- **agent 回复只认 provider session id**（它分配的房间号）；Ora 用路由表把 provider id → 信箱（Ora 内部按 Ora id 处理）；
- 类比：provider id = 房间门牌号（快递员只认这个）；Ora id = 酒店客户档案号（内部用）；路由表 = 前台的门牌号→档案对照表。

## 六、前端真实流程（重要修正）

```
T0：在 opencode 上聊天（当前会话：已绑定，不在池子）
T1：点开选择器 → warm【当前目标 CLI】（有缓存就直接读缓存）
T2：点选 claude → 【记录选择】(pending) + 【预热 claude】+ 菜单不关（继续选模型）
T3：发下一条消息 → 【真正执行 switch】：claim 认领 claude 会话 + 搬历史
```

- **不是所有 CLI 都预热**——只 warm 当前目标（`useWarmSession(selection, agentCli)`，一个 CLI）；
- **模型列表有缓存**（useAgentModelStore.known[agentCli]）——同一 CLI 之前握手过直接读缓存；
- **点选 ≠ 立即切换**——怕打断正在回复的 agent，真正切换（commit）在发下一条消息时。

## 七、标题获取窗口（title acquisition）

- 标题从 Agent 来：**push**（SessionInfoUpdate 带 title）/ **poll**（session/list 能力）；
- 四种状态：**Disabled**（load 恢复的会话，不获取）/ **AwaitingFirstEligiblePrompt**（新 attach，等第一个回合）/ **Polling**（第一个回合后，push+poll 双通道）/ **Locked**（换 agent 后/窗口完成，固定）；
- poll 两个尝试：First + Final（独立生命周期，first 失败 final 还能兜底；final 被抢占就 Locked）；
- **为什么标题没变化**：load 是 Disabled、依赖 Agent（不 push 不支持 list 就没来源）、窗口会 Locked；
- ⚠️ 待确认：observe_session_update 里 title 落库 + 前端读取链路未追完。

## 八、Agent Runtime 全景（数据流与调用关系）

### 启动流程（一次）

```
Backend::open → AgentRuntimeManager::new(pool, home, clock)
  → reconcile_running_sessions（Running → Stopped）
  → ConnectionSupervisors::start → 5 台 ConnectionSupervisor::start
      → 每台 spawn_runtime_thread（独立线程）
          → run_supervisor 循环：spawn 子进程 → initialize(15s) → Ready / 重试(250ms→30s)
```

### 消息数据流（Agent → 前端）

```
Agent stdout → 连接读取器(AcpPeer) → 路由表 route_event(provider id 查 HashMap)
  → 找到路由 → try_send 信箱(256)
      ├─ 满 → 删路由 + 火警 QueueOverflow + 会话失败
      └─ 空位 → 投进去
  → 找不到 + setup 窗口 → 等待室；无窗口 → 丢弃
  → 信箱 → actor：① 观察 ② 先记录(ora-history) ③ 转发前端(NDJSON)
  → 30s 闹钟：每消费一条重置
```

### 命令调用流（前端 → Agent）

```
前端 chat store → contracts client → transport → 门卫 handler
  → Backend::prompt_session（错误/请求生命周期）
  → manager：校验(空/16MiB/Running/历史) → lifecycle 锁 → actor_for → 命令通道
  → actor run_prompt：ACP session/prompt → SessionEventStream → 循环等事件
```

### 控制流（异常）

```
进程死 → fail_generation(清路由+ConnectionLost)；溢出 → 投递失败(QueueOverflow)
  → 火警(controls 无界) → actor：失败/隔离/标记 Stopped
```

### 生命周期（一天）

```
warm(内存池) → attach(落库+actor+路由) → load(恢复历史+重建通道)
  → prompt → update 流 → permission(自动允许) → TurnEnded → stop(保留)/delete(销毁)
[意外] 进程死 → fail_generation → actor mark_stopped(复用) → 重启 → load 重建
```

### 清理体系（三层）

```
路由 ← token(正常摘牌/投递失败) / generation(进程死)
actor ← manager(换 agent/删会话/恢复历史)
CLI 进程 ← ora-process(ProcessTree 树级终止)
```

### 两个 id 桥接

```
Agent 世界：只有 provider session id
Ora 世界：Ora session id(数据库) + provider id(存 agent_session_id 字段)
桥：路由表 HashMap<provider session id, 信箱>
切换 agent：Ora id 不变、provider id 换
```

## 九、检查题答案（8 题）

1. **WarmReservation 为什么做成对象+Drop**：调用方可能永远不调第二个调用（HTTP 断线丢 future、panic）——Drop 自动退回，绝不泄漏；
2. **WarmKey 三样**：target（任务/项目）+ agent_cli（CLI）+ client_id（聊天面）；client_id 防两个标签页抢同一会话；
3. **gates 门锁**：同一 key 的 warm 请求排队，第二个发现已有直接用——防重复创建留孤儿（React 双挂载）；
4. **take vs claim**：take（attach）有 id 报 id；claim（switch）没有 id（选择器预热的 id 在池子内部）只能报 key——选择器用 key 预热、换绑用同一 key 认领；
5. **三个标识**：Ora id（数据库身份证，切换不变）、provider id（CLI 房间号，切换换）、warmkey（预订信息，不是 id）；
6. **agent 回复靠 provider id**：agent 只认自己分配的房间号；Ora 用路由表把 provider id → 信箱（桥接）；
7. **点选 CLI**：记录选择(pending) + 预热目标 + 菜单不关；**真正切换在发下一条消息时**（怕打断正在回复的 agent）；
8. **标题获取窗口**：新 attach 才开（AwaitingFirstEligiblePrompt）；load 恢复的是 Disabled（标题应已持久化，不重新要）。

## 十、术语表新增

Warm Session（预热会话，已深化）、WarmKey（预热键）、WarmPool（预热池）、Reservation（预订凭证）、Gate（门锁）、take/claim（取/认领）、Provider Session Id（已深化）、Ora Session Id（Ora 会话号）、Title Acquisition（标题获取）、PollAttempt（轮询尝试）。详见桌面 software technical terms.md。

## 十一、下一课预告

> 第 9 课（按 LESSON-PLAN，⭐ 关键专题 1）：保存上下文信息——ora-history 会话历史与 transcript。对话怎么存？为什么自己记不靠 agent 复述？文件格式/顺序规则？写失败（degraded）怎么办？换 agent 时上下文怎么带过去（handoff transcript）？
