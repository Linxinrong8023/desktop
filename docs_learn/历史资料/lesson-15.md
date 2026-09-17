# 第 15 课：Workflow 定义与版本管理（总结）

> 互动式学习：从"不变的是什么/变的是什么"引导，到两个实体分离、draft/发布/回滚/激活生命周期、版本命名、删除保护，最后追问"为什么不级联删除 run"。

## 〇、核心问题

> workflow 怎么存？draft / publish / version 生命周期？为什么草稿可改、发布快照不可变？

一句话：**workflow 拆成"身份"和"内容"两个实体——身份几乎不变（一张表一行），内容按"版本"一版一版地存（一张表多行）；草稿（draft）是活的工作区，发布是冻结。**

## 一、两个实体、两张表

### Workflow（身份）—— 几乎不变

```rust
pub struct Workflow {
    pub id: WorkflowId,
    pub name: String,
    pub published_snapshot_id: Option<WorkflowSnapshotId>, // 当前生效版本指针
    pub audit_fields: AuditFields,
}
```

`workflows` 表：id, name, published_snapshot_id, created_at, updated_at, is_deleted

**注意：workflow 表不存内容（图）**——它只存身份 + 当前生效版本指针。

### WorkflowSnapshot（内容/版本）—— 经常变

```rust
pub struct WorkflowSnapshot {
    pub id: WorkflowSnapshotId,
    pub workflow_id: WorkflowId,
    pub version: String,          // "draft" 或 "v1.0.0" 等
    pub graph: String,            // 不透明 React Flow JSON
    pub created_at: i64,
    pub updated_at: Option<i64>,  // 发布后为 NULL（不可变标记）
    pub is_deleted: bool,
}
```

`workflow_snapshots` 表：每行 = 一个版本（含 draft）。

**为什么拆两个**：名字/当前生效版本几乎不变，图经常变——拆开各自演化：改名不动图、改图不动身份。

## 二、draft = 工作区（活），published = 冻结（死）

- **draft**：每个 workflow 恰好一个，`version` 永远 = 字符串 `"draft"`（保留字符串）。它存的是"用户编辑器里正在编辑的图"。
- **保存（修改）≠ 发布**：
  - 保存 = **就地改 draft 行的 graph**（不产生新行）；
  - 发布 = **复制 draft 的图，新建一行**不可变快照 + 更新 workflows 指针（产生新行）。
- **已发布快照的 `updated_at = NULL`** = 不可变标记：发布后永不更新，历史版本原样保留可回滚。

### 生命周期时间线（draft 与最新版本的关系）

```
t0：创建        → draft = [start]（无已发布版本）
t2：发布 v1.0.0 → 复制 draft 图 → 新快照；★此刻 draft == v1.0.0
t3：继续编辑    → draft 偏离 v1.0.0（v1.0.0 冻结不动）
t4：发布 v1.0.1 → 又相等；t5 后再次偏离……
```

**规律**：draft 与最新版本只在"发布那一刻"相等；draft 是"下一版"的草稿，一直在流；published 是"取一瓢水冻成的冰块"。

## 三、发布 / 回滚 / 激活（核心对比，重点！）

| 操作 | 改 draft（复制历史/当前图） | 改指针 published_snapshot_id | 线上生效版本 |
|---|---|---|---|
| **发布 publish** | 复制 draft → 新快照 | ✅ 指向新快照 | 变（新版本上线） |
| **回滚 rollback** | ✅ 复制某个历史快照的图进 draft | ❌ **不动** | **不变** |
| **激活 activate** | ✅ 同步目标快照的图进 draft | ✅ **切换** | **变** |

记忆口诀：**rollback = "草稿改回旧版，线上照旧"；activate = "旧版重新上线，草稿跟着变"**。只有 publish 和 activate 动指针；rollback 永远不动指针。

**为什么 activate 要同步 draft**：否则线上是 v1.0.0、编辑器草稿却还是老样子，用户再改一笔发布，就发布出"基于错误草稿"的版本。

**为什么 rollback 是"复制"而不是"指针指向历史"**：draft 必须独立拥有自己的图（接下来要被编辑），历史快照不可变——复制字符串进 draft 行，互不影响。

**杂志类比**：draft = 正在排版的草稿；发布 = 复印存档贴期号；rollback = "下期排版改成第 1 期版式，第 2 期照常卖"；activate = "停售第 2 期，第 1 期重新在售，排版同步"。

## 四、graph 字段：画布的完整截图（不透明 JSON）

graph 就是 React Flow 画布的原生格式（`toObject()` 直接产出），存为字符串：

```json
{
  "nodes": [
    { "id": "node-1", "type": "workflow", "position": {"x":120,"y":260},
      "data": { "kind": "start", "title": "开始" } },
    { "id": "node-2", "type": "workflow", "position": {"x":400,"y":260},
      "data": { "kind": "agent", "title": "写代码",
                "agentConfig": { "executor": {"agentCli":"opencode","modelId":"gpt-5"},
                                 "roleId":"engineer",
                                 "skills":[{"skillId":"s1","enabled":true}],
                                 "prompt":"请实现登录功能" } } },
    { "id": "node-3", "type": "workflow", "position": {"x":680,"y":260},
      "data": { "kind": "output", "title": "输出结果" } }
  ],
  "edges": [ {"id":"e1","source":"node-1","target":"node-2"},
             {"id":"e2","source":"node-2","target":"node-3"} ]
}
```

字段地图：

| 字段 | 是什么 | 谁用 |
|---|---|---|
| `nodes[]` / `edges[]` | 节点 / 连线（连线 = 执行顺序） | 引擎建 DAG |
| 节点 `id` | 节点身份 | 引擎 |
| 节点 `type`、`position` | 渲染组件名、画布坐标——**执行无关** | 前端渲染 |
| `data.kind` | **真正的节点类型**（start/agent/output；serde 改名为 node_type） | 引擎 |
| `data.agentConfig` | agent 节点执行契约：executor(CLI+模型)、roleId、skills、prompt | 引擎（"一个节点 = 一个完整 agent 任务"） |

**设计要点**：前端怎么存后端就怎么存（无第二份 DTO）；后端当**不透明字符串**，存时不校验结构；真正解析（serde → 校验 DAG）发生在**运行时引擎**（第 16 课），GraphError 是 run 时才报的。

## 五、版本命名规则

- **用户自定义**（如 "v1.0.0"）：须非空、≤128 字节、**URL 路径段安全**（不能是 `.`/`..`，不能含 `/` 等）、不能撞已有版本（`VersionAlreadyExists`）；
- **自动生成**：`v{时间戳}`（如 v1700000000123），同一毫秒撞车 → **加数字后缀重试**（`MAX_AUTOMATIC_VERSION_COLLISION_RETRIES = 100`）；
- **为什么 URL 安全**：版本号会出现在 `GET /api/workflows/{id}/versions/{version}` 路径里，`..` 会变成路径穿越；
- **为什么自动的加后缀、用户的直接拒绝**：自动是系统内部行为，静默改后缀无伤大雅；用户输入是用户意图，系统只能拒绝不能擅自改；
- **部分唯一索引**：同一 workflow 内未删除的版本名唯一 → **软删除的版本名可以复用**。

## 六、删除保护（"删之前先问有没有别的东西指着它"）

### 删除单个版本（DeleteSnapshotHandler）三条限制

| 检查 | 拒绝原因（防什么） |
|---|---|
| ① 是 draft？ | 工作区没了，系统**没有重建 draft 的机制**（创建时原子建，之后所有操作都假设它在）→ workflow 废了 |
| ② 是当前激活的（published_snapshot_id 指向它）？ | 指针悬空——"当前生效版本"指向不存在的行 |
| ③ 被任何**未删除的 run**（is_deleted=0）引用？ | run 的结果页要渲染这张图（查看/restart），**已跑完的 run 也算**——只有 run 本身被删了才可删 |

### 删除整个 workflow（DeleteWorkflowHandler）

- 只要还有任何未删除的 run（`ActiveRuns`）→ **拒绝**；否则级联软删所有快照。

### 两个删除粒度（易混！）

| 操作 | 是什么 | 允不允许 |
|---|---|---|
| 删整个 workflow | "这个工作流我不想要了" | ✅（无未删 run 时） |
| 删单个版本 | workflow 留着，清理某个历史版本 | ⚠️ 三条限制 |

### 为什么不级联删 run（追问结论）

- run 是**档案**（冻结图、节点账本、会话、worktree 产物），不该被"顺带"删掉；
- "删 workflow"是单一 API 请求、无"确认连带删除"参数 → 级联 = **隐式破坏**，违反"破坏性操作要显式"；
- API 调用方不只 UI（脚本/别的客户端），不能靠"前端会提示"；
- 正确姿势**两步走**：先删 run（专门 API，会删 worktree）→ 再删 workflow。每步显式、可预期。

## 七、检查题（详细答案版）

**1. 为什么拆两个实体、两张表？**

一个 workflow 里有两类信息，变化频率完全不同：**身份**（id、name、当前生效版本指针）几乎不变；**内容**（图）经常变。拆开后各自演化：改名不动图、改图不动身份。**关键：workflow 表不存图**——它只有 id、name、published_snapshot_id + 审计字段；图全在 workflow_snapshots 表的 graph 列。类比：杂志登记簿（身份）vs 每期存档（内容）。代码：`crates/domain/src/workflow.rs`（Workflow 4 字段 / WorkflowSnapshot 7 字段）、`crates/db/src/migration/schema_v0006.rs`。

**2. draft 的 version 是什么？和已发布的本质区别？**

version 永远是字符串 `"draft"`（保留字符串，`DRAFT_VERSION` 常量）。本质区别：**draft 是活的**（update_draft 就地改 graph，不新建行；用户保存=改 draft 行），**published 是冻结的**（发布后 `updated_at = NULL`，永不再变）。draft 与最新版本只在“发布那一刻”相等（发布=复制 draft），之后 draft 继续变——它是“下一版”的草稿。删除保护第一条就是 draft 不可删：系统没有重建 draft 的机制（创建时原子建，之后所有操作都假设它在），删了 workflow 就废了。

**3. 发布（publish）做哪两件事？**

① 把 draft 的 graph **复制**一份，新建一行不可变快照（version = 用户填或自动 `v{时间戳}`）；② 更新 `workflows.published_snapshot_id` 指向新快照。**关键：是“复制”**，不是“把 draft 冻结”——draft 行继续活（当下一版的草稿），发布行是复印出来的存档（`updated_at = NULL`）。一个事务完成。类比：复印草稿纸存进档案柜，并宣布“这是正式版”。

**4. rollback vs activate（本课最易答反！）**

两者都“把某个历史版本的图复制进 draft”，但：**rollback 不动指针** → 线上生效版本**不变**（“草稿改回旧版，线上照旧”）；**activate 切指针 + 同步 draft** → 线上生效版本**变**（“旧版重新上线，草稿跟着变”）。只有 publish 和 activate 动指针，rollback 永远不动。**为什么 activate 要同步 draft**：否则线上是 v1.0.0、草稿却是老样子，用户再编辑发布就发布出“基于错误草稿”的版本。**为什么是“复制”而不是“指针指向历史”**：draft 必须独立拥有自己的图（接下来要被编辑），历史快照不可变——复制字符串进 draft 行，互不影响。

**5. 为什么已发布快照 updated_at = NULL？**

不可变标记：发布后任何 UPDATE 都不再更新它（更新语句不动该列），历史版本原样保留——保证回滚/激活拿到的就是发布那一刻的图。对比：draft 的 updated_at 非 NULL（可编辑、每次保存更新）。语义上：`NULL` = “这个东西不再会被更新”，与 draft 的“持续更新”形成对比。

**6. 删单个版本三条 + 删整个 workflow 一条？**

删单个版本（`DeleteSnapshotHandler`）三条拒绝，按顺序检查：① 是 draft？→ 拒（工作区没了，系统不重建）；② 是当前激活的（`published_snapshot_id` 指向它）？→ 拒（指针悬空，“当前生效版本”指向不存在的行）；③ 被任何**未删除的 run**（`is_deleted=0`）引用？→ 拒（run 的结果页要渲染这张图，**已跑完的也算**——只有 run 本身被删才可删）。删整个 workflow（`DeleteWorkflowHandler`）：还有任何未删除的 run（`ActiveRuns`）→ 拒（否则 run 的图孤儿）；无 run 时级联软删所有快照。统一逻辑：**“删之前先问有没有别的东西指着它”**（指针、run、工作区）。注意用户视角的两种删除：UI 上“不想要这个工作流”= 删 workflow（允许）；“只清理某个版本”= 删单个快照（受限）。删 workflow **不级联删 run**——run 是档案（冻结图+账本+worktree），破坏性操作要显式；正确姿势两步走：先删 run（会删 worktree）再删 workflow。

**7. graph 是什么格式？后端存时校验吗？**

React Flow 画布**原生 JSON**（`toObject()` 直接产出），存为字符串：`nodes[]`（id、type 渲染组件名、position 画布坐标、data）+ `edges[]`（连线 = 执行顺序）。执行相关的在 `data.kind`（真正的节点类型 start/agent/output）和 `data.agentConfig`（executor 的 agentCli/modelId、roleId、skills、prompt）；渲染相关的（type、position）执行时忽略。后端当**不透明字符串**存，**不校验结构**；真正解析（serde → petgraph DAG 校验）发生在引擎 **run 时**（第 16 课 GraphError 那些校验错误是 run 才报的）。为什么前端怎么存后端就怎么存：**无第二份 DTO**，单一数据源（第 17 课展开）。

## 八、术语表新增

Draft（草稿/工作区）、Snapshot（冻结快照）、Publish（发布=复制+冻结+指针）、Rollback（回滚=只改草稿）、Activate（激活=切指针+同步草稿）、Version Label（版本标签）、Immutable Snapshot（不可变快照）、Partial Unique Index（部分唯一索引）、ActiveRuns / SnapshotInUse / ActiveSnapshot / DraftSnapshot（删除保护结果枚举）、Graph（不透明 React Flow JSON）。

## 九、下一课预告

> Workflow 运行引擎：一个 run 怎么跑起来？run CRUD（冻结快照、专用 run-task + worktree）、引擎（petgraph DAG 调度、串行状态机、NodeExecutor 委托 agent 会话、complete/fail 回调）、HITL、崩溃恢复（boot sweep）。第 6 课学的"workflow 用 git 三件事"在这里落地。
