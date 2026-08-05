# 第 3 课：`ora-application`——接口、handler、组合根与完整流程（总结）

> 对应对话内容：handler 语法、ports（接口）、依赖注入、泛型、Fake 测试、Backend::open 接线、AppState 状态注入、从 main 到 SQL 的完整流程、"为什么这样设计"的动机。

## 一、五件套模块结构

每个业务功能（project/task/session/skill...）在 `ora-application` 里都是同一套布局：

```
crates/application/src/project/
├── ports.rs          ← 接口定义（合同）：ProjectRepository、ProjectIdGenerator、Clock
├── handlers.rs       ← 用例编排：CreateProjectHandler 等
├── id_generator.rs   ← 接口的一种实现：UUID 造 ID
├── mapper.rs         ← 翻译官：领域 → 契约
└── tests.rs          ← 用"内存假仓储"跑通所有用例
```

## 二、接口（ports.rs）= 合同，只规定做什么

```rust
pub trait ProjectRepository {
    fn create_project(&self, project: Project) -> Result<Project, RepositoryError>;
    fn find_project(&self, project_id: &ProjectId) -> Result<Option<Project>, RepositoryError>;
    fn list_projects(&self) -> Result<Vec<Project>, RepositoryError>;
    fn update_project(&self, project: Project) -> Result<Project, RepositoryError>;
    fn soft_delete_project(&self, project_id: &ProjectId, deleted_at: i64) -> Result<bool, RepositoryError>;
}

pub trait ProjectIdGenerator { fn generate_project_id(&self) -> ProjectId; }
pub trait Clock { fn now_timestamp_millis(&self) -> i64; }
```

- 接口里全是业务语言，没有一个字提到 SQLite/SQL；
- 两个适配器：真实现（ora-db 的 SqliteProjectRepository）+ 假实现（tests.rs 的 FakeProjectRepository）——有**两个**适配器，说明这条接缝是真实被使用的。

## 三、handler = 依赖注入 + 泛型（语法已拆解）

```rust
pub struct CreateProjectHandler<Repository, IdGenerator, ClockSource> {
    repository: Repository,
    id_generator: IdGenerator,
    clock: ClockSource,
}
```

- **泛型** = 占位符：`<Repository, IdGenerator, ClockSource>` 是三个"插槽"，用的时候填具体类型；
- `impl<A,B,C> Foo<A,B,C>`：第一对尖括号声明占位符，第二对说明针对哪个类型；
- `where Repository: ProjectRepository` = 门槛：插槽里的东西必须实现接口（这样编译器才知道方法存在）；
- `new` = 打包（不是特殊语法，只是约定俗成的构造函数名）；
- `handle` 5 步：① clock 要时间 → ② id_generator 造 ID → ③ 构造领域模型（AuditFields）→ ④ repository 存库（接口调用）→ ⑤ mapper 映射回契约。

## 四、Fake 测试（为什么能不依赖数据库）

```rust
let repository = Rc::new(FakeProjectRepository::default());   // 内存 Vec 当数据库
let handler = CreateProjectHandler::new(
    repository.clone(),
    FixedProjectIdGenerator::new("project-1"),  // 固定 ID
    FixedClock::new(1_700_000_000_000),         // 固定时间
);
```

- Fake 实现了同一个 ProjectRepository 接口 → 可以互换；
- 固定 ID/时钟 → 断言精确；
- `fail_next(RepositoryError)` → 能测错误路径；
- 断言两头：契约响应（对外）+ 领域模型（内部存储）。

## 五、组合根（Backend::open）= 总装车间

`crates/backend/src/bootstrap.rs` 的 `Backend::open` 是唯一接线处：

1. 建目录（数据目录、worktree 根）；
2. `default_migration_catalog()` 加载迁移清单；
3. `DatabaseBootstrapper`：打开 SQLite → 跑迁移（建表）→ 返回连接池；
4. `SystemClock`（真时钟）；
5. `AgentRuntimeManager::new(pool, home, clock)`（拉起 Agent 子进程）；
6. 组装各 Api：`ProjectApi::new(pool, clock)` → `SqliteProjectRepository::new(pool)` → `CreateProjectHandler::new(repo, idgen, clock)`（**焊死真线**）。

`Backend::open` 的 4 个调用点：Web 服务器（bootstrap.rs）、桌面应用（src-tauri/lib.rs）、backend 集成测试（临时目录，x2）。

## 六、核心认知：接线 vs 干活（两条路）

- **接线（构造）**：`Backend::open` 只是 new 对象、装抽屉。这条路上唯一碰数据库的是 `DatabaseBootstrapper`（打开文件 + 建表）；
- **干活（调用）**：请求来了才走：handler → 接口调用 → 真 SQL。

**"传送点"真相**：没有运行时传送。编译器把泛型 handler 按具体类型**复制成多份**（静态分发/单态化）：
- 副本 A（backend）：`self.repository` = SqliteProjectRepository → 调用真 SQL；
- 副本 B（测试）：`self.repository` = FakeProjectRepository → 往 Vec 塞。
- 类比：接口调用 = 按钮；实现 = 出厂焊死的线；组合根 = 焊线的工人；编译器 = 焊线本身。

## 六·五、数据库操作到底在哪？（SQL 的藏身处，已用 grep 验证）

全仓库搜索（`INSERT INTO` / `rusqlite`）的结论：**整个 workspace 只有 `crates/db` 一个 crate 直接操作数据库**。其他 crate（application/backend/web server/desktop）一律通过接口，自己不写 SQL。

**crates/db 内部的两大类：**

| 文件 | 干什么 | SQL 类型 |
|---|---|---|
| `src/bootstrap.rs` | 打开 SQLite 文件、连接配置 | 打开连接 |
| `src/migration/schema_v0001.rs` 等 | 数据库表结构蓝图 | CREATE TABLE |
| `src/migration/runner.rs` | 按版本执行迁移 | CREATE TABLE / 变更 |
| `src/repository/project.rs` | 项目的增查改删 | INSERT/SELECT/UPDATE |
| `src/repository/task.rs` | 任务 | 同上 |
| `src/repository/session.rs` | 会话 | 同上 |
| `src/repository/skill.rs` | 技能 | 同上 |
| `src/repository/agent_definition.rs` | Agent 定义 | 同上 |
| `src/repository/worktree.rs` | 工作树 | 同上 |
| `src/repository/project_work_context.rs` | 占座（租约） | 同上 |
| `src/repository/cascade.rs` | 聚合删除（多表事务） | 事务 |
| `src/repository/connection.rs` | 连接池管理 | 连接 |
| `tests.rs` / `repository_tests.rs` | 数据库集成测试 | 直接 SQL |

**以后自己验证的方法：**

```bash
grep -rln 'INSERT INTO\|rusqlite' --include='*.rs' crates apps xtask
```

**结论**：想改 SQL → 去 `crates/db`；想改业务 → 去 `ora-application`；想改错误/组合 → 去 `ora-backend`。这就是"各管各的"。

## 七、AppState = 门卫的公共工具箱（状态注入）

```rust
pub struct AppState {
    backend: Backend,                      // 总装车间
    file_system_api: Arc<FileSystemApi>,   // Web 专属
    project_work_context_api: Arc<ProjectWorkContextApi>, // Web 专属
    ready: Arc<AtomicBool>,
}
```

- 谁造的：main.rs → `build_app_state()`（内部调 `Backend::open`）；
- 怎么进 handler：`build_router(app_state.clone())` 塞进 Router → handler 签名写 `State(app_state): State<AppState>`（提取器）→ axum 请求时自动注入；
- **两种 handler 别混**：门卫 handler（web/Tauri，极薄，只转手）vs 业务 handler（ora-application，5 步编排）。

## 八、为什么启动这样设计（动机）

> 启动 = 把线接好再营业。

1. **依赖先于使用者**：配置 → 日志 → 数据库 → 池 → Api → 路由（自底向上构造）；
2. **日志最早**：出事先有东西能记；
3. **接线集中一处**（组合根）：改接线只改一个函数，不会散落各处；
4. **fail fast**：数据库打不开就退出，绝不带病营业（失败在启动期暴露 vs 运行时 500）；
5. **mark_ready 最后**："就绪"= 承诺所有依赖活着；
6. **接线逻辑放共享 crate**（ora-backend）：Web/桌面/测试各调一次 Backend::open。

**万能三问**（以后看任何设计都适用）：
1. 它在解决什么问题？（什么疼）
2. 如果不这么做会怎样？（灾难）
3. 它把代价挪去哪了？（取舍）

## 九、标准答案版：从 main 到 SQL 的完整流程

```
【启动·一次】读配置 → 装日志 → 总装车间（迁移建表、连接池、handler、agent、clock）
            → 打包 AppState → 贴路由表 → 占端口 → 挂"营业中" → 开门

【请求·每次】
前端：JS 对象 →(序列化)→ JSON → HTTP
服务器：
  ① 反序列化：JSON → 契约 DTO        （拆包）
  ② 路由表：找到门卫 handler          （指路）
  ③ 门卫：从工具箱拿 backend → 转交    （转手）
  ④ 业务 handler：契约 → 领域模型      （变身：+ID、+时间）
  ⑤ 接口调用：领域模型 →(repository)→ SQL → 数据库  （真干活）
返回：
  ⑥ 领域模型 ←(map_row)← 数据库
  ⑦ 契约 DTO ←(mapper)← 领域模型      （丢审计字段）
  ⑧ JSON ←(序列化)← 契约 DTO          （打包）
  ⑨ HTTP 响应 → 前端反序列化 → 显示     （拆包）
```

**一句话浓缩**：出门是"JSON→契约→领域→SQL"，进门是"SQL→领域→契约→JSON"，中间永远夹着领域模型。

## 十、同名文件导航 + SQLite 传递链（易混点补充）

**同名文件（必须带完整路径看）：**

| 完整路径 | 是什么 |
|---|---|
| `crates/backend/src/bootstrap.rs` | 总装车间（Backend::open） |
| `apps/web/server/src/bootstrap.rs` | Web 门卫开门流程（build_app_state） |
| `crates/db/src/bootstrap.rs` | 数据库启动器（DatabaseBootstrapper） |
| `crates/domain/src/project.rs` | 领域模型 |
| `crates/contracts/src/project.rs` | 契约 DTO |
| `crates/backend/src/project.rs` | ProjectApi（backend 门面） |
| `crates/db/src/repository/project.rs` | SqliteProjectRepository（真 SQL） |
| `crates/application/src/project/handlers.rs` | CreateProjectHandler（业务编排） |

**SQLite 三层传递：**

```
第1层 传"路径"：Backend::open(BackendPaths{ database_path, ... })
第2层 传"池子"：DatabaseBootstrapper → RepositoryPool → ProjectApi::new(pool, clock)
第3层 传"仓储"：SqliteProjectRepository::new(pool) → CreateProjectHandler::new(repository, ...)
```

**new 的调用规律**：每一层的 `new` 都被**上一层构造函数**调用（接线链 = new 调用链）：

| new / open | 定义处 | 被谁调用 |
|---|---|---|
| `Backend::open` | backend/src/bootstrap.rs | `build_backend`（web/server/src/bootstrap.rs） |
| `ProjectApi::new` | backend/src/project.rs | `Backend::open` |
| `SqliteProjectRepository::new` | db/src/repository/project.rs | `ProjectApi::new` |
| `CreateProjectHandler::new` | application/src/project/handlers.rs | `ProjectApi::new` |
| `UuidProjectIdGenerator::new` | application/src/project/id_generator.rs | `ProjectApi::new` |

自验：`grep -rn 'SqliteProjectRepository::new' crates apps` 就能看到调用点。

## 十一、术语表新增（详见桌面 software technical terms.md）

Generic（泛型）、Port（接口角色/接缝）、Fake/Mock（假实现）、Static Dispatch（静态分发，已深化）、Extractor（提取器）、Fail Fast（快速失败）、Composition Root（已深化）、Monomorphization（单态化）等。

## 十二、下一课预告

> `ora-backend` 的 error 体系：`ApplicationError → BackendError → PublicError` 怎么转换？`ErrorClassification` 如何决定 HTTP 状态码和日志级别？RequestLifecycle（请求生命周期）如何保证只记一次完成事件？
