# 第 17 课：Workflow 前端设计模式 + 模块全貌（总结）

> 关键专题 4。前端部分以**用户视角**写（不深入 UI 实现），后端**详细总结**（workflow 完整模块地图）。核心干货：三课整合主线（graph JSON 贯穿 / 快照冻结 / ports 接缝）。

## 〇、核心问题

> workflow 是个大模块，前后端加起来几十个文件。它的完整地图是什么？前端怎么画、怎么展示？后端哪些是"骨架"、哪些还没展开？

一句话：**workflow = 存储层（第 15 课：图怎么存/版本怎么管）+ 执行层（第 16 课：图怎么跑）+ 展示/交互层（本课：图怎么画、结果怎么看）——一份 graph JSON 贯穿三层，快照冻结锁定，ports 隔离两端。**

## 一、workflow 模块全貌地图（★ 已讲透，☆ 未展开）

```
【后端】（★ 第 15/16 课已讲透）
│  workflow 定义/版本（15）：Workflow/Snapshot、draft/publish/rollback/activate
│  run CRUD + 引擎（16）：冻结、worktree、调度、串行、幂等、崩溃恢复
│  crates/backend/src/workflow_run_prerequisites.rs（skills 物化）等

【前端 - 数据/接口层】（★ 第 17 课骨架）
│  workflow-mock（编辑器数据：factory/validation/capabilities）
│  workflow-runtime（ports + memory 适配器 + graph-codec + path-order）

【前端 - UI 层】（☆ 未展开，用户视角见第二节）
│  settings/workflow-flow/（React Flow 编辑器：canvas/layout/connection）
│  features/workflow/（OpenSpec 引导 + workflow-store）
│  features/workflow-run/（43 个文件：Theater 舞台/HITL/总览/产物……）
```

### 三个 surface（D5.2 边界，README 明确定义"不许互相碰"）

| Surface | 负责什么 | 不许干什么 |
|---|---|---|
| ① Settings React Flow 编辑器 | 定义编辑 + Deploy to project | 不许驱动 live run Theater |
| ② OpenSpec stepper + workflow-store | Spec 模式的 composer workflow | 不许写 GraphWorkflowRun、不共享 run 状态 |
| ③ workflow-run | 项目级 run 的 Theater / Overview | 不许复用 settings 的 canvas |

## 二、前端：用户视角（一个用户怎么用 workflow）

### 1. 画图（Settings 编辑器）

- 打开设置 → Workflow 编辑器：拖节点（start / agent / output，编辑器还支持 condition / junction / human / loop / subflow，**但后端 v1 只执行 3 种**——画得出 ≠ 跑得了，不支持的启动时报错）；
- 每个 agent 节点配置：选 CLI + 模型、角色、技能、自定义提示词；
- 连线 = 决定执行顺序（DAG）；
- 导入外部 workflow → 校验（唯一 id、合法端点、单 Start）——不合格不让进画布。

### 2. 版本管理（第 15 课用户视角）

- 保存草稿（自动，一直改）；
- **发布**：复制草稿成不可变版本（v1.0.0），成为"当前生效版本"；
- **回滚**：把草稿改回旧版本的样子（线上不变）；
- **激活**：让旧版本重新成为线上版本（草稿也同步）；
- 发布过的版本历史都在，能按版本查看/删除（受保护规则）。

### 3. 部署到项目（Deploy to project）

- 把 workflow **部署到某个项目**（挂载 mount）——项目关联 workflow 定义（引用，不是复制），之后 run 都在该项目下跑。

### 4. 运行（run）

- 在项目下启动 run：选发布快照（冻结）、可填 kickoff 输入；
- 系统建专属 run-task + Git worktree + 技能物化（用户无感）；
- 可取消、可 restart（从头重跑）、可看结果。

### 5. 看 Theater（戏剧化展示 run 过程）

- **Path Rail（轨道）**：按拓扑 + 画布位置排成的演出顺序条，显示每幕状态（未开始/运行/等待输入/成功/失败/取消）；
- **Stage（舞台）**：中间大屏，聚光灯自动聚焦"最需要人"的那幕（**HITL 等输入 > 最新开始 > 最近成功**）；
- **并行**：多个 agent 同时跑时，舞台上可切换看哪个（chips + 箭头键）；
- **Session dock**：点开某幕的会话，看这个 agent 完整干了啥（复用聊天气泡）；
- **HITL**：节点等输入时，舞台出现权限/澄清请求 + 输入框，用户回复继续；
- **Result act**：run 结束时显示最终结果（输出 + 总文件改动数）+ 返回总览；
- **Artifact reveal**：某个节点产出新产物时，聚光灯自动跳过去展示。

### 6. 三个入口的分工（用户视角）

- Settings = "造 workflow"（编辑 + 发布 + 部署）；
- OpenSpec stepper = "用 Spec 引导"（另一种 workflow 入口）；
- 项目里 = "跑 workflow + 看 Theater"。

## 三、后端详细总结（workflow 完整后端地图）

### 1. 定义与版本层（第 15 课，简略回顾）

- `crates/application/src/workflow/`：13 个 handler（Create/Get/List/Update/Delete workflow、GetDraft/UpdateDraft/Publish/Rollback/Activate、ListVersions/GetVersion/DeleteSnapshot）；
- `crates/application/src/workflow/ports.rs`：`WorkflowRepository` trait 以**领域操作**为单位（create 含初始 draft、publish 一个事务、rollback、activate…），不是逐条 SQL；
- `crates/db/src/migration/schema_v0006.rs`：workflows / workflow_snapshots / workflow_runs / workflow_node_runs 四张表。

### 2. run CRUD 层（第 15 课 run 部分 + 第 16 课）

- `crates/application/src/workflow_run/`：Create（冻结快照 + 建 run-task/worktree + skills 物化 + 一个事务落库）、Get/List（读模型不带图）、Delete（拒 ActiveRun + 连物理 worktree 一起删）；
- 快照保护：`SnapshotInUse`（被未删 run 引用）/ `ActiveRuns`（workflow 有未删 run）——第 15 课删除保护。

### 3. 引擎层（第 16 课，简略回顾）

- `crates/application/src/workflow_run/engine/`：engine.rs（调度波/ready 集合）、graph.rs（React Flow JSON → petgraph DAG + 校验）、node_type.rs（v1 只支持 Start/Agent/Output）、ports.rs（NodeExecutor / WorkflowRunEngineRepository / WorkflowRunWorktreeInitializer）；
- 四大机制：调度波（每次状态变化重算 ready）、串行（每 run 串行执行器 + Immediate 事务）、幂等（不是 Running 就 no-op）、崩溃恢复（boot sweep 清孤儿）；
- restart = 软删旧节点行 + 重置 Pending + 从头 start。

### 4. 后端门面与组装（backend 层）

| 文件 | 干什么 |
|---|---|
| `crates/backend/src/workflow.rs` | WorkflowApi（CRUD/版本门面） |
| `crates/backend/src/workflow_run.rs` | WorkflowRunApi（run CRUD 门面） |
| `crates/backend/src/workflow_run_engine.rs` | **组装**：回调 → executor → engine → control handler（第 16 课：callback 先建、engine 后建、`set_engine` 补挂） |
| `crates/backend/src/workflow_run_executor.rs` | 节点执行链：warm → attach → 选模型 → 组装 prompt（技能斜杠命令 + 节点 prompt + 角色指令 + upstream lineage + run input）→ prompt 流式 → 快照 diff 记账 → complete/fail 回调 |
| `crates/backend/src/workflow_run_prerequisites.rs` | deploy 时校验角色/技能 + 物化 skills 到 `<worktree>/.agents/skills/`（run 出生即完整） |
| `crates/backend/src/bootstrap.rs` | `run_workflow_run_boot_sweep`（启动清扫）+ `build_workflow_run_engine`（组合根） |

### 5. 数据层（db）

- `crates/db/src/repository/workflow.rs`：workflow/snapshot 持久化（各领域操作一个事务）；
- `crates/db/src/repository/workflow_run_engine.rs`：引擎持久化（start_run / complete_node / fail_node / cancel_run / restart_run / update_run_input / 当前节点锚点重写），每个状态转换一个 Immediate 事务；
- 引擎 repository 与 CRUD repository **刻意分离**：引擎只暴露"状态机操作"（每次转换一个事务 + 维护锚点），不暴露通用 overwrite——防止外部乱改 run 状态。

### 6. 相关（未展开但存在）

- `GitCleanupWorker`（bootstrap 里的 durable git 清理：回放上次进程留下的 cleanup job 和过期租约）；
- 前端 `workflow-run` 43 个文件（Theater/总览/HITL——用户视角见第二节）。

## 四、架构主线（三课怎么连成一条线——最重要）

| 贯穿物 | 角色 |
|---|---|
| **graph JSON（一份真相）** | 第 17 课画出来 → 第 15 课存进 draft/snapshot → 第 16 课引擎 serde 解析成 petgraph DAG 执行——三种视图（画布/存储/执行），无第二份 DTO |
| **快照冻结** | 第 15 课 publish 把 draft 复制成不可变版本 → 第 16 课 run 创建时钉住这个冻结版本——**引擎跑的是不会变的图**（可复现、可审计、删除受保护） |
| **ports（接缝）** | 第 17 课 `WorkflowRuntime` 接口定义 run 操作；现在用 memory 适配器（前端 UI 假数据），将来 F2 写 HTTP 适配器实现同一接口——UI 一行不改（依赖倒置 + 组合根） |

## 五、前端设计模式（骨架，用户视角可忽略的实现细节）

1. **画布数据不复制 DTO**：React Flow 原生形状 = 单一数据源（画布怎么存，后端就怎么存，引擎就怎么解析）；
2. **workflow-mock = 编辑器数据包**：node-data（结构/图纸）、node-factory（生产节点带默认值）、capabilities（可选项目录）——三者靠类型互相咬合；
3. **workflow-runtime = 执行包（UI-free）**：ports（接口合同）+ memory（内存假员工）+ graph-codec（存储 envelope 约定 + 容错解析）+ workflow-path-order（拓扑 + 位置排序）；不含 React，可移植可测试；
4. **graph-codec 的容错**：parse 取认识的字段、**保留不认识的字段**（前向兼容——旧版本不弄坏新版本数据），坏 JSON 兜底空数组；
5. **ports + 内存适配器 = "合同 + 临时工"**：UI 只认接口，组合根注入实现；将来换 HTTP 后端只改组合根一行（同第 3 课后端模式）；
6. **事件投影**（run-projection.ts）：前端展示状态从后端状态**派生**（HITL = Pending+非空 current_nodes → awaiting_input；无 node-run 行 → idle；无 skipped/partial_failed 概念）。

## 六、检查题（详细答案版）

**1. 为什么画布数据用 React Flow 原生形状（不复制 DTO）？各用各的会怎样？**
→ 一份真相三种用途（画/存/执行），不存在同步问题。各用各的 = 两份手写的会漂移（第 1 课"母版 + 复印机"）：改画布格式忘改存储格式 → 画出来的存进去变样 / 引擎读不懂。执行视图（petgraph DAG）不是第二份 DTO，是同一份 JSON 的另一种解析。

**2. node-data / node-factory / capabilities 三者关系？**
→ node-data = 结构图纸（类型定义）；node-factory = 工厂（按图纸生产节点 + 默认值）；capabilities = 目录（可选模型/角色/技能 + 标签 + 默认配置）。factory 依赖 data 的类型保证产出合法，依赖 capabilities 拿标签/默认值；capabilities 也依赖 data 的结构。

**3. workflow-mock 管"画"、workflow-runtime 管"跑"？为什么 runtime 要 UI-free？**
→ 对：mock = 编辑器世界（画），runtime = 执行世界（跑）。UI-free（不含 React）三个理由：① 接口不该被 UI 库绑架（合同不能被"纸的材质"限制——内存实现、HTTP 实现、Node 测试都要能用）；② 可移植可测试（没 React 的包任何环境能跑）；③ 职责分离（UI 归 app-shell，逻辑归 runtime 包）。

**4. ports + 内存适配器解决什么？换 HTTP 后端 UI 改多少？**
→ 解决"UI 不依赖具体实现"（依赖倒置 + 组合根，同第 3 课）：UI 只 import 接口类型，调用 runtime.runs.start(...)；换实现 = 组合根（注入点）改一行，UI 零改动。类比：墙上的插座（ports）固定，插临时工电器（memory）还是正式工电器（HTTP）随便换——房间不用改。**注意：前端 workflow 目前确实用 memory 假数据跑 UI，接真后端的契约适配器是 F2 待办（README 明说 "Not the generated-contract adapter yet"）——后端引擎是真的（git log #303 real execution），只是前端↔后端那条线还没铺。**

**5. graph-codec 的 parse 遇到不认识的字段：丢还是保留？为什么？**
→ **保留**（`...record` 原样带过）。前向兼容：v2 存的图带新字段（如 mcps）→ v1 打开 → v1 保存 → 若丢字段，v2 再打开数据就坏了。保留 = 数据在版本间安全往返。**不是"为了回滚"**——回滚是第 15 课版本管理，这里说的是数据格式兼容。

**6. Theater 聚光灯（focus）怎么决定"看谁"？为什么 HITL 优先？**
→ activeIds = 所有 running/awaiting_input 节点（并行可能有多个）；primaryId 挑选：用户钉住的优先 → 否则 active 里"等待输入(HITL) > 最新开始 > 最近成功 > 第一个节点"。HITL 优先因为它**卡住需要人**（别的一幕还在自己跑，这一幕在等你点按钮）——聚光灯自动打到"最需要人"的地方，这是注意力管理。

**7. 为什么 HITL（awaiting_input）是前端"投影"出来的，不是后端真状态？**
→ 后端状态模型是引擎视角（5 值），前端展示模型是用户视角。投影：后端 Pending + 非空 current_nodes → awaiting_input；无 node-run 行 → idle（条件分支没走到 = 无行 = idle，不是 skipped）。wire 状态保持纯净，展示层做翻译（同契约 mapper 思想）。

**8. 三课怎么连成一条线？**
→ 一份 graph JSON 贯穿三层（画/存/执行）；快照冻结是主线（15 课 publish 冻结 → 16 课 run 钉住它，引擎跑不变的图）；ports 是两端接缝（17 课接口，memory 现在，F2 HTTP 将来）。

## 七、术语表新增

Theater（舞台 UI）、Act（幕 = 一个 agent 节点）、Path Rail（路径轨道）、Spotlight/Focus（聚光灯 = 当前聚焦哪幕）、HITL（人机交互等待）、Session Dock（会话坞）、Result Act（结果幕）、Artifact Reveal（产物揭示）、Port（接口合同，已深化）、Adapter（适配器 = 插头）、Memory Adapter（内存假员工）、UI-free（接口不含 UI 库）、Projection（投影 = 展示状态派生）、Forward Compatibility（前向兼容 = 保留未知字段）、Surface（三个 UI 面的边界）、Deploy to project（部署挂载）。

## 八、下一课预告

> 第 18 课：前端契约 SDK（@ora/contracts）——生成/手写边界、endpoints manifest、fetch vs Tauri transport、错误解码。**注意：第 18~19 课偏前端**（你对前端不关心，可选跳过或粗读）；第 20 课 Web 服务器运行时、第 21 课桌面运行时（Tauri）是后端/运行时，值得细学。
