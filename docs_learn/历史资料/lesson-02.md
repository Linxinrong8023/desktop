# 第 2 课：同一个"项目"的三种样子 + 数据链路（总结）

> 对应对话内容：三种模型（数据库行 / 领域模型 / 契约 DTO）、转换链、routes.rs 阅读法、以及一轮 grill me 后达成的 6 条共识。

## 一、核心问题

> 同一个"项目"，系统里为什么有三种不同的样子？它们之间怎么转换？

## 二、三种模型对照（以 Project 为例）

### ① 数据库里的一行（`crates/db/src/migration/schema_v0001.rs`）

```sql
CREATE TABLE IF NOT EXISTS projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    root_path TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    is_deleted INTEGER NOT NULL DEFAULT 0
);
```

6 列。`created_at`、`updated_at`、`is_deleted` 是"审计三件套"。

### ② 领域模型（`crates/domain/src/project.rs`）

```rust
pub struct Project {
    pub id: ProjectId,             // 类型安全的新类型（ids.rs 的宏生成）
    pub name: String,
    pub root_path: String,
    pub audit_fields: AuditFields, // 审计三件套打包在一个盒子里
}
```

4 个字段。ID newtype 让 `ProjectId` 和 `TaskId` 不能混用（编译器拦截）。

### ③ 契约 DTO（`crates/contracts/src/project.rs`）

```rust
pub struct Project {
    pub id: String,       // 领域里是 ProjectId，这里变成普通 String
    pub name: String,
    pub root_path: String, // serde 出门时改成 rootPath
}
```

3 个字段。审计三件套完全消失——外部不需要知道内部私事。

### 四个关键差异

1. **字段多少**：数据库最全（6 列）→ 领域居中（4 字段）→ 契约最简（3 字段）；
2. **ID 类型**：数据库 TEXT → 领域 `ProjectId`（类型安全）→ 契约 `String`（JSON 世界没有 newtype 概念）；
3. **命名**：Rust 内部 `snake_case` → 契约出门穿 `camelCase`；
4. **行为**：领域可有构造函数校验（如 `Skill::new` 拒绝空名）；契约只是数据袋子。

## 三、转换链：三个"翻译官"各有分工

```
SQL 行 ──repository──▶ 领域模型 ──mapper──▶ 契约 DTO ──serde──▶ JSON
(ora-db)            (业务逻辑用)         (ora-application)     (出门)
```

| 转换 | 从 → 到 | 谁干的 |
|---|---|---|
| 反序列化 | JSON → 契约 | serde（web server 收包） |
| 变身 | 契约 → 领域 | handler + id_generator + clock |
| 存库 | 领域 → SQL | repository（INSERT） |
| 还原 | SQL → 领域 | repository 的 `map_project_row` |
| 映射 | 领域 → 契约 | mapper（ora-application） |
| 序列化 | 契约 → JSON | serde（web server 发包） |

**注意**：mapper 只做"领域→契约"；"数据库行→领域"是 repository 的 `map_project_row`（名字里的 row 就是"行"）。

## 四、完整数据链路标准答案卡（13 步）

**请求方向**（前端 → 数据库）：

| # | 转换 | 谁干的 |
|---|---|---|
| 1 | 前端填数据 → 调用 `client.project.create({...})` | 页面代码 |
| 2 | 编译期类型检查 | TypeScript 编译器 |
| 3 | 查 endpoints 手册 → 得到 POST + /api/projects | endpoints 常量 |
| 4 | 对象 → JSON（序列化） | `JSON.stringify` |
| 5 | HTTP 请求发出 | fetch transport |
| 6 | JSON → 契约 DTO（反序列化） | serde（web server） |
| 7 | 契约 → 领域模型 | handler（+id_generator 造 ID、clock 造时间） |
| 8 | 领域 → SQL（INSERT） | repository（ora-db） |

**响应方向**（数据库 → 前端）：

| # | 转换 | 谁干的 |
|---|---|---|
| 9 | SQL 行 → 领域模型 | repository 的 `map_project_row` |
| 10 | 领域 → 契约 DTO | mapper（ora-application） |
| 11 | 契约 → JSON（序列化） | serde |
| 12 | HTTP 响应 | web server |
| 13 | JSON → 对象（反序列化）→ 显示 | `JSON.parse` + 页面代码 |

**记忆口诀**：出门是"JSON→契约→领域→SQL"，进门是"SQL→领域→契约→JSON"，中间永远夹着领域模型，两头都是 JSON。

## 五、routes.rs 阅读法

```rust
.route(
    PROJECT_PATH,                                          // 第一个参数：路径常量（来自 ora-contracts！）
    get(projects::get_project)                             // 第二个参数：方法链
        .put(projects::update_project)
        .delete(projects::delete_project),
)
```

要点：

1. **一个路径可挂多个方法**：先按路径找表项，再按方法选函数；
2. **路径是常量不是字符串**：`PROJECT_PATH` 来自 `ora-contracts`，与前端 `endpoints.ts` 同源 → 路径永不漂移；
3. **handler 是"门卫层最薄的函数"**：只做 4 件事——接住框架注入的 State/Path、组装契约请求、调 backend、包装响应和转错误；
4. **方法没注册 → `405 Method Not Allowed`**（协议规定了错误说法）；
5. 细节印证不变量：`UpdateProjectBody` 只有 `name`，没有 `root_path`——项目根路径不可变，接口直接不给你传的机会。

## 六、Grill me 达成的 6 条共识

| # | 共识 |
|---|---|
| 1 | 契约层存在的**根本原因**：内部/外部世界的防火墙，两边可各自演化（生成 TS 只是附带好处） |
| 2 | **ID 和时间由系统生成**：身份签发权和档案权在系统手里，不在前端 |
| 3 | **丢字段发生在翻译层（mapper）**，数据库无损；改名片 ≠ 毁档案 |
| 4 | 新字段要**穿透 8 层**（迁移→SQL→map_row→领域→mapper→契约→生成→前端），每层放行；已有字段只需改暴露层 |
| 5 | 路由匹配 = **路径 + 方法**两步；方法没挂就回 `405` |
| 6 | 传值标准：**业务内容（意图）由客户端提供，系统元数据（记录）由系统签发** |

## 七、本课新增/深化术语

Enum（枚举）、Newtype（新类型）、Mapper（映射器）、Serde、HTTP Method/Status Code（405 等）、Route（路由）；"Migration"条目补充"新字段第一步是迁移"。详见桌面临时术语表。

## 七·五、检查题答案（主课 5 题）

1. **三份 Project 字段**：数据库 6 列（id/name/root_path/created_at/updated_at/is_deleted）、领域 4 字段（id/name/root_path/audit_fields）、契约 3 字段（id/name/rootPath）；审计字段在数据库和领域，契约没有；
2. **契约 id 用 String**：JSON 世界没有 newtype 概念；类型安全是 Rust 编译期的事，离开 Rust 就只剩通用 String；
3. **转换**：数据库行→领域 = ora-db（map_project_row）；领域→契约 = ora-application（mapper）；
4. **Task 状态**：数据库 0/1/2、领域枚举 Todo/Doing/Done、契约字符串 "todo"/"doing"/"done"；存 99 → from_database_value 报 InvalidTaskStatus；
5. **显示创建时间**：契约加字段 → 重新生成 TS → mapper 传递 → 前端显示（数据库和领域不用改——数据一直在，只是没印上名片）。

## 八、B 补全课：领域层全貌 + 契约错误 + 租约概念

### 0. 分支事故的教训（意外收获的实战课）

- 不同分支 = 不同版本的代码：同路径文件在不同分支内容可能完全不同；
- 读代码/讨论代码前必须先确认分支（本次教学曾误用 import-skill-frontend 的 routes.rs）；
- 文档可能滞后：docs/ 描述的是 project_learn 状态；task_diff 功能只存在于 import-skill-frontend 分支；
- 结论：学习以当前分支（project_learn）为准。

### 1. 领域层全实体地图（当前分支 11 个实体）

| 实体 | 关键字段（除审计外） | 一句话用途 |
|---|---|---|
| `Project` | id, name, root_path | 项目 |
| `Task` | id, project_id, title, status, **worktree_id(可选)** | 任务，可选关联 Git 工作树 |
| `Worktree` | id, task_id, **branch_name(可选)**, activity | 任务专属的 Git 工作树 |
| `Session` | id, task_id, agent_cli, **agent_session_id**, status | 一次 Agent 对话 |
| `Skill` | id, name, description | 可复用技能（**名字有校验**） |
| `AgentDefinition` | id, name, description | 可配置的 Agent 类型（**名字有校验**） |
| `ProjectWorkContext` | id, surface, window_id, project_id, **lease_expires_at** | 窗口→项目的占座（**例外：无审计字段**） |
| `VirtualFolder` | id, project_id, name, mount_point | 虚拟文件夹（暂无处理器） |
| `VirtualEntry` | id, virtual_folder_id, parent_entry_id, name, kind, content_ref | 虚拟条目（暂无处理器） |
| `Artifact` | id, task_id, content(可选) | 任务产物（暂无处理器） |

**规律**：绝大多数实体 = 类型安全 ID + 业务字段 + `AuditFields`，`new()` 不失败（纯快照）。

### 2. 两个例外

**例外 1：Session 的不可变路由 + 先有鸡后有蛋**

- `task_id`、`agent_cli`、`agent_session_id` 构造后不可变（路由信息）；只有 `status` 可变（`with_status`）；
- `agent_session_id` **必填**：一个 CLI 进程被多个会话共享，provider 靠它区分消息属于哪个会话；所以创建时先向 provider 要会话 ID，再落库；
- 共享连接 + 每会话独立 ID 的并发模型，是后面 Agent Runtime 课的重点。

**例外 2：ProjectWorkContext 没有 AuditFields（租约制）**

- 租约制：窗口占座项目，租期由后端定（120 秒），到期不续租自动释放；
- 无 `is_deleted`：过期行直接删，不存在的东西不需要删除标记（让非法状态不可表示）。

### 3. 枚举统一模式

所有枚举 = 领域枚举 + `database_value()` + `from_database_value()`（未知值拒绝，报 DomainModelError）：

| 枚举 | 数据库存 | 领域用 |
|---|---|---|
| `TaskStatus` | 整数 0/1/2 | Todo/Doing/Done（契约 "todo"/"doing"/"done"） |
| `SessionStatus` | 0/1 | Running/Stopped |
| `WorktreeActivity` | 0/1 | Inactive/Active |
| `VirtualEntryKind` | 0/1 | File/Directory |
| `AgentCli` | **文本** "ora-space.opencode" 等 | OpenCode/Nga/CodeAgentCli |
| `ProjectWorkContextSurface` | 文本 "web"/"tauri" | Web/Tauri |

**为什么 AgentCli 存文本而不是整数**：整数含义依赖枚举声明顺序，将来增删/调整声明顺序会导致旧数据静默错位（如 0 从 OpenCode 变成新 CLI）；带命名空间的稳定文本（"ora-space." 前缀）与顺序无关，还直接对应外部可执行文件名。其他枚举用整数是因为 0/1/2 是契约中稳定写死的映射，且枚举本身高度稳定。这是权衡，不是教条。

### 4. 构造函数校验

- `Skill::new`、`AgentDefinition::new` 会失败：trim 后空名 → `EmptySkillName` / `EmptyAgentDefinitionName`；
- 名字是给人看、用来搜索的，空名字无意义；非法状态在构造时就被拒绝。

### 5. 契约错误（PublicError）

```json
{"code": "project_not_found", "params": {}, "requestId": "550e8400-..."}
```

- 判别联合：每个错误 = `code` + 类型化的 `params`（大多数为空，少数带参数如 `maxFiles`）；
- 没有 message、没有外层信封——契约不暴露内部错误文本；
- 前端用途（真实代码 `packages/app-shell/src/i18n/contract-error.ts`）：
  - **code → 查 i18n 错误字典**，翻译成给用户看的友好消息（`t("errors." + code, params)`）；
  - **requestId → 关联后端日志**，出问题靠它找到同一条请求；
  - 未知 code → `UnknownRemoteError` 兜底，显示通用错误 + requestId；
- 观察：契约错误可先于功能定义（当前有 SkillUpload 系列错误，但 routes 里还没有上传路由）。

### 6. 租约（lease）概念详解

**ProjectWorkContext 到底记录什么**："哪个 UI 窗口当前在前台工作于哪个项目"——是 UI 层的占座，不是任务/代码/对话。

**窗口 vs worktree vs session（分层）**：

| 概念 | 层 | 回答的问题 |
|---|---|---|
| Project/Task | 数据层 | 有哪些项目/任务 |
| Worktree | Git 层 | 任务的代码在哪个目录 |
| Session | Agent 层 | Agent 正在跟哪个任务对话 |
| Window | UI 层 | 用户眼前的界面在显示哪个项目 |

**为什么租约会过期**："窗口在用哪个项目"是实时状态（像共享单车占用、聊天软件在线状态），不是档案。窗口崩溃/断线时，租约自动过期 = 僵尸占座自动让位，项目不会被永久锁死。

**为什么独占规则与 git 无关**：worktree 已经解决了 git 目录并发（每任务独立目录+分支），窗口独占协调的是项目级工作视图（两个前台抢同一个工作台）。独占只发生在 Tauri 窗口之间；Web 永不冲突。

**现状（设计先行、接线滞后）**：唯一调用者是 Web 启动时创建合成占座（web/main），无前端续租，2 分钟后过期；Desktop 完全不实现这三个操作（transport 拒绝 `unsupported_operation`）；清理过期行是 `ora-scheduler` 的待办。这是真实项目常见状态：规则定好、未接线。

### 7. B 课检查题答案要点

1. 实体四件套：ID + 业务字段 + AuditFields + `new()`；
2. 校验：`Skill`、`AgentDefinition`（trim + 拒绝空名）；
3. Session 不可变路由：task_id、agent_cli、agent_session_id；必填原因：共享 CLI 进程靠它区分消息归属，串台就全乱；
4. PWC 无 is_deleted：租约过期直接删，不需要删除标记；
5. AgentCli 存文本：整数依赖声明顺序，调整顺序会导致旧数据静默错位；
6. 错误码用途：code → i18n 字典给用户看；requestId → 关联日志排查；未知 code 有兜底。

## 九、下一课预告

> `ora-application` 的 handler、ports（接口）、测试——"业务规则"这层到底怎么写的？为什么它可以不依赖数据库？（第一课检查题第 2 题的"为什么"会在这里看到真实代码。）
