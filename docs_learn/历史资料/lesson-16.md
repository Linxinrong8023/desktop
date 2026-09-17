# 第 16 课：Workflow 运行引擎（总结）

> 互动式学习：从"并行 vs 串行"引导到 ready 集合/调度波，再到串行事务、幂等回调、取消、崩溃恢复、restart。中间用户追问"回调是什么""同时改会怎样"（并发覆盖残留）、"取消如何体现"，全部闭环。

## 〇、核心问题

> 一个 run 从"启动"到"结束"是怎么被驱动的？多节点并行/依赖怎么调度？并发报告怎么保证不错乱？崩溃了怎么办？

一句话：**引擎是"反应式调度器"——不按顺序走图，而是每次状态变化后重算"谁的前置齐了"，一波波放行；所有状态存数据库（可恢复），所有状态转换串行（防并发错乱）。**

## 一、run 是什么（回顾）

- run = workflow 的一次执行：**冻结发布快照**（第 15 课）、建**专属 run-task + Git worktree**、skills 物化到 `<worktree>/.agents/skills/`、三者一个事务落库（第 6 课）；
- 状态机（run 和节点共用 5 值）：**Pending → Running → Succeeded / Failed / Cancelled**；
- 引擎（`WorkflowRunEngine`）**同步、无状态**：每次命令从数据库重算集合，不做内存缓存。

## 二、引擎 = 反应式调度器（不是线性执行器）

### ready 集合（判断"轮到谁"）

> **一个节点的所有直接前驱（predecessors）都 Succeeded = 它 ready（可开工）**

```
        ┌─→ nodeB（agent）──┐
start ──┤                    ├─→ nodeD（agent）
        └─→ nodeC（agent）──┘

start 完成 → B、C ready → 并行开工
B 完成、C 完成 → D 的所有前驱（B 和 C）都 Succeeded → D ready
（只有 B 完成、C 还挂着 → D 不 ready，继续等）
```

- "谁是谁的前置"从 **edges** 推导（petgraph 有向图，前驱查询 = 所有指向该节点的边的起点）；
- **互不依赖的节点并行跑**（各自开真实 session，同时在 worktree 里干活）；
- **控制节点（Start/Output）同步完成**：没有 session，调度循环里当场算完，不经过异步执行器。

### 调度波（scheduling wave）

> **每完成一个节点，就触发一轮检查（一波），能开的新节点在这一波一起开工**（反应式，不是按顺序播放）。

```
第 1 波：start 开工 → 秒完成
第 2 波：B、C ready → 并行开工
第 3 波：B 完成 → 检查 D？C 还没完成 → 不开
第 4 波：C 完成 → D ready → 开工
第 5 波：D 完成 → ready 空 + 无在跑 → finish_run
```

**多米诺骨牌类比**：一张倒下（节点完成）→ 触发检查 → 能倒的跟着倒（新节点开工）。

## 三、为什么状态转换必须串行（并发安全）

### 问题：并行节点同时完成 → 同时改数据库

每个"完成报告"（回调）都要改**两张表**：
- `workflow_node_runs`：该节点行 status → Succeeded；
- `workflow_runs`：`current_nodes` 清单（把完成的节点划掉）。

**同时改同一个字段（current_nodes）→ 互相覆盖**（"读-算-写"被拆开，都基于旧值算，后写覆盖先写）：

```
回调 B：读 ["B","C"] → 划 B → 写 ["C"]     （剩下 C）
回调 C：读 ["B","C"] → 划 C → 写 ["B"]     （剩下 B）
后写覆盖先写 → 最终 = ["B"] 或 ["C"]（随机）
但真实情况两个都完成了，应该是 []
→ 残留已完成节点 → 引擎以为还有人在跑 → run 卡死
```

**注意**：冲突的结果是**残留**（不是空）；空反而是正确结果。另一个问题是"中间态"：改清单和改状态是两步，交错时会出现"清单说没在跑、状态行说在跑"的两本账对不上。

### 解法：三层保护

| 层 | 机制 | 解决什么 |
|---|---|---|
| 应用层 | **串行执行器**：每个 run 的所有命令/回调排队，一次一个 | 后一个基于前一个的结果，覆盖不丢 |
| 数据库层 | **Immediate 事务**：改节点状态 + 改清单 + 重写锚点 = 一个事务（写锁） | 两处修改焊成一体，中间态不存在 |
| 幂等 | 事务里先查状态，**不是 Running 就 no-op** | 迟到/重复回调不造成二次伤害 |

**类比**：工人（agent）可以**并行干活**，但"交账"必须排队（账本只有一本）；交账时锁账本（事务）；过期的旧收据直接作废（幂等）。

## 四、取消（cancel）

```
用户点取消 → cancel_run（一个事务）：
  ① 检查 run 是 Running（否则 NotActive）
  ② 该 run 下所有 Pending/Running 节点 → Cancelled
  ③ current_nodes 清空 → []
  ④ run → Cancelled
然后 backend 异步停止 run 的所有活跃 session（停进程，需要时间）
```

**迟到的回调**：agent 在"被停止前"刚好干完 → 回调到达 → 查状态发现已 Cancelled（不是 Running）→ **no-op**。run 一旦取消就不可能再变回成功。

**设计顺序：先落库、后停进程**——数据库先定死"取消"这个事实，停进程慢慢来；任何迟到报告都被幂等检查挡住。**类比**：先在公告栏贴通知（数据库），再一个个通知工人停工；没看到通知干完活来交差的，看公告栏"已取消"→ 不收。

## 五、崩溃恢复（boot sweep）

**场景**：进程崩溃（强杀/断电），数据库留下"死状态"（run=Running、节点=Running），永远不会有人回调了 → 僵尸 run。

**处理**：每次 Backend 启动时（营业前）跑 `run_workflow_run_boot_sweep`：
```
① list_recoverable_runs：run_status IN (Running, Failed)（没跑完的）
② 这些 run 里所有 Pending/Running 节点 → Failed（"INTERRUPTED_BY_RESTART"）
③ 仍为 Running 的 run → Failed
```
- 已 Succeeded 的节点**不动**；
- **为什么 Failed 也算**：一个节点失败时 run 变 Failed，但并行节点可能还挂着（孤儿），也要清；
- 清扫后 run 显示"因中断失败"，**可以 restart**。

**为什么能恢复**：引擎**无状态 + 所有状态在数据库**——`current_nodes` 锚点存数据库（不是内存），崩溃后重启从数据库就能知道"当时哪些节点在跑"。**类比**：内存 = 便签纸（人走就没），数据库 = 账本（人走了账还在）；引擎选择记在账本上，为的是崩溃后能恢复现场。

## 六、restart（重启）：从头开始，不是断点恢复

```rust
// restart = restart_run + start
restart_run：① 检查 run 不是 Running（Running 不可 restart）
             ② 旧节点行全部软删（is_deleted = 1，历史保留可查）
             ③ run 重置：Pending、current_nodes=[]、output/error/时间全清空
然后 start → 从 start 节点重新跑整张图
```

- **从头开始**：新的调度波会**新建一批全新节点行**（新 id），不是把旧行改回来；
- 旧行软删（不可见但物理存在）→ 能看出"第 1 轮跑到 B 就中断、第 2 轮重跑成功"（审计）；
- **为什么从头而不是断点恢复**：① 快照冻结 = 可复现（同一张图再跑一遍，可预期）；② 断点恢复要恢复 agent 上下文（进程死了 session 历史没了）、worktree 中间态，复杂度爆炸且不可靠。**类比**：菜做一半停电，重做 = 从头炒一盘，不是接着半熟的食材炒。

## 七、四大机制总表

| 机制 | 解决什么 | 一句话 |
|---|---|---|
| 调度波 / ready 集合 | 图怎么跑起来 | 每次状态变化重算"谁的前置齐了"，一波波放行，互不依赖就并行 |
| 串行（排队 + 事务） | 并发写乱账 | 报告排队、一个事务，后一个基于前一个的结果 |
| 幂等（状态检查） | 迟到/重复报告 | 不是 Running 就 no-op；取消/失败后不可能被改回成功 |
| 崩溃恢复（boot sweep） | 进程死了留僵尸 | 启动时把孤儿节点/Running run 标 Failed，可 restart |

## 八、检查题（详细答案版）

**1. ready 集合怎么判断节点可开工？**

条件：该节点的**所有直接前驱都 Succeeded**。前驱从哪来：graph 的 `edges`（petgraph 有向图，前驱 = 所有指向该节点的边的起点——像查地图“能进这座城的路从哪来”）。引擎每次状态变化重算三个集合：completed（已成功）、in_flight（正在跑）、ready（前驱全在 completed 且自己不在 in_flight）。ready 为空且 in_flight 为空 → `finish_run`（跑完）。互不依赖的节点同时 ready → **并行开工**（各自开真实 session，同时在同一 worktree 干活）。

**2. 为什么节点并行跑没事、但完成报告必须排队？**

并行跑 = 各自改自己的 worktree 文件、各自的 session，**互不碰同一份数据**。但完成报告（回调）要改**同一处**：`workflow_runs` 表的 `current_nodes` 字段（把完成的节点从清单划掉）。两个回调同时改 → “读-算-写”被拆开，都读到旧值 `["B","C"]`，B 算 `["C"]`、C 算 `["B"]`，后写覆盖先写 → 最终清单残留一个已完成节点（真实应为 `[]`）→ 引擎以为还有人在跑 → **run 卡死**。类比：工人可以并行干活，但只有一个收银台，交账必须排队（账本只有一本）。

**3. 串行 + 事务解决哪两个问题？**

① **覆盖残留**：排队后，后一个回调基于前一个改完的**新值**计算（C 读到 `["C"]` 而不是旧 `["B","C"]`），不会把 B 的更新弄丢 → 最终清单正确为空。② **中间态**：“改节点状态行”和“改 current_nodes”是两步，交错时别人会看到“清单说没在跑、状态行说在跑”的两本账对不上的瞬间；一个 Immediate 事务把两步**焊成一体**（要么都做要么都没做），中间态不存在。

**4. run 取消后迟到的完成回调会怎样？**

`cancel_run` 事务先把 run 及所有未完成节点标 **Cancelled**、清空 current_nodes（**先落库**）；然后 backend 异步停 session（**后停进程**）。agent 在“被停止前”刚好干完 → 回调到达 → 事务里查状态：已 Cancelled（不是 Running）→ 返回 `NotRunning` → **no-op**（什么都不改）。为什么重要：若不检查，迟到报告会把 Cancelled 改回 Succeeded → run 已取消却显示节点成功，**状态自相矛盾**。类比：老板先在公告栏贴“项目取消”（数据库），再一个个通知工人停工；没看到通知干完活来交差的，看公告栏“已取消”→ 不收。

**5. 为什么 current_nodes 存数据库、引擎无状态？**

“当前在跑哪些节点”若放**内存**：进程崩溃 → 内存没了 → 重启后数据库里只有孤零零的“run = Running”，不知道哪些节点在跑 → boot sweep 无从精确清扫。放**数据库**（`workflow_runs.state` 字段）：崩溃后重启 → 一查数据库就知道“当时 B、C 在跑” → 它们是孤儿（永远不会回调）→ 标 Failed。引擎**无状态**（每次命令从数据库重算）是配套设计：不信任内存，一切以数据库为准；崩溃只丢“最后一个事务之后”的东西。类比：内存 = 便签纸（人走就没），数据库 = 账本（人走了账还在）。

## 九、术语表新增

Reactive Scheduler（反应式调度器）、Ready Set（就绪集合）、Scheduling Wave（调度波）、Serial Executor（串行执行器）、Immediate Transaction（立即写锁事务）、Idempotent Callback（幂等回调）、No-op（空操作）、Orphan Node Run（孤儿节点运行）、Boot Sweep（启动清扫）、INTERRUPTED_BY_RESTART、Current Nodes Anchor（当前节点锚点）、State Machine（5 值状态机）、Restart（重置重跑 vs 断点恢复）。

## 十、下一课预告

> ⭐ Workflow 前端设计模式（关键专题 4）：`packages/workflow-mock`（React Flow 原生形状 = 单一数据源、agentConfig 版本化、校验、demo 执行）与 `packages/workflow-runtime`（Host/Run ports、graph-codec 归一化、workflowPathOrder、内存适配器、UI-free runtime），以及 Theater 舞台 UI。纯前端/设计模式视角，无后端。
