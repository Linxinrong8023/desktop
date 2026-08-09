# 第 5 课：数据库持久化（ora-db）——迁移、连接池、repository（总结）

> 对应对话内容：迁移机制（图纸/目录/账本/检查员）、连接池（出生/配置/借还/复用/销毁）、repository 读写（SQL 行→领域模型翻译）、路径传递链、SQL 注入与参数化。

## 〇、ora-db 管三件事

| 事 | 代码 | 通俗说 |
|---|---|---|
| 连接 | `repository/connection.rs` | 数据库通道怎么开、怎么管 |
| 结构 | `migration/` | 表怎么建、怎么升级 |
| 数据 | `repository/*.rs` | 增删改查（SQL 全在这） |

**全仓库只有 ora-db 一个 crate 写 SQL**（已用 grep 验证）。

## 一、迁移机制（Migration）

### 1. 三个角色 + 一个执行者

| 角色 | 是什么 | 文件 | 类比 |
|---|---|---|---|
| Migration（图纸） | 版本号 + up（向前）+ down（回滚） | `schema_v0001~0008.rs` | 装修图纸 |
| MigrationCatalog（目录） | 所有图纸 + 目标版本 + 校验 | `catalog.rs` | 档案管理员 |
| migrations 表（账本） | 记录已执行的版本和时刻 | `record.rs` + 表 | 装修记录本 |
| reconcile_database（检查员） | 对账决策 + 执行 | `runner.rs` | 检查员 |

### 2. 一个迁移 = 版本 + up + down

```rust
pub struct Migration {
    version: &'static str,           // 版本号 "0001"
    up_statements: &'static [&'static str],   // 向前：CREATE TABLE / ALTER
    down_statements: &'static [&'static str], // 回滚：DROP TABLE
}
```

### 3. 账本建两次（都是 IF NOT EXISTS，幂等）

- **正式**：0001 图纸的 up 里（`schema_v0001.rs` 第 76 行）；
- **兜底**：runner 读账本前先 ensure（`runner.rs` 第 24 行）——解决"读账本前必须已有账本"的鸡生蛋问题。

### 4. 三道检查（别混）

| # | 检查什么 | 在哪 | 报什么错 |
|---|---|---|---|
| ① | 代码里版本唯一、严格递增、目标前缀 | 构建时（catalog） | Duplicate/Unordered/InvalidTargetPrefix |
| ② | 账本 vs 目标逐位一致 | 运行时（reconcile） | DivergedMigrationHistory |
| ③ | 账本提到的版本目录里存在 | 运行时（回滚/应用时） | UnknownAppliedMigrationVersion |

- ① 检查代码；② 检查历史和代码是否吻合（被篡改就报警）；③ 检查图纸是否还在。
- **删 0008 定义但账本还有 → ③ 拦**（回滚需要 0008 的 down，找不到 → 启动失败）。正确姿势：**保留定义、缩短目标前缀、先退后删**。

### 5. 对账决策（reconcile）

```
① ensure 账本存在
② 读账本（SELECT ... ORDER BY version ASC）
③ 拿目标版本
④ 算：公共前缀长度、缺多少、多多少
⑤ 前缀逐位对比（zip+take+enumerate）→ 不一致 → ❌ 硬错误
⑥ 账本多 → 倒序跑多余 down（.rev()），拆结构 + 删账本行
⑦ 账本少 → 顺序跑缺的 up（.skip(已执行)），建结构 + 插账本行
⑧ 一样 → no-op
```

每个 up/down 步骤：**SQL + 记账在同一事务**（`execute_migration_step`），失败全回滚；语句逐条跑，报错精确到版本和方向。

### 6. 四个核心思想

1. **增量不重跑**：老库只跑新增（skip 已执行的），新库才全跑；迁移跑过一次永不重跑；
2. **不可变历史**：只往后追加（写 0009），不改旧的——像 git 历史；
3. **构建时校验**（fail fast）：图纸有问题启动就炸；
4. **账本即真相**：数据库状态完全由代码管辖，对不上就硬错误。

### 7. 常见场景速查

| 场景 | 结果 |
|---|---|
| 每次启动全跑吗？ | 不，老库只跑新增 |
| 想改表结构？ | 写新迁移，不改旧的 |
| 怎么回滚？ | 目标版本改前缀，倒序跑 down |
| 数据库被手动动过？ | ② 前缀对比失败 → 拒绝启动 |
| 删了 0008 定义？ | ③ 找不到 → 启动失败 |
| 建表失败？ | 事务回滚，账本不记，下次重试 |

## 二、连接池（RepositoryPool）

### 1. 三个角色

| 角色 | 是谁 | 干什么 |
|---|---|---|
| r2d2 | 第三方库 | 池的管理者（几条、借还） |
| rusqlite | 第三方库 | SQLite 驱动 |
| SqliteConnectionManager | Ora 自己 | 告诉 r2d2 "连接怎么开、怎么配" |

### 2. 连接出生时统一打扮（connect）

```rust
fn connect(&self) -> ... {
    let connection = Connection::open(&self.path)?;   // 打开
    configure_repository_connection(&connection)?;    // 统一配置！
}
```

4 条规矩（PRAGMA）：`WAL`（读写可并行）、`busy_timeout=5s`（忙就等）、`synchronous=NORMAL`（折中）、`foreign_keys=ON`（检查引用）。**集中配置 = 一致性 + 改一处全生效**。

### 3. 借-用-还（with_connection）

```rust
let connection = self.inner.get()?;   // 借
operation(&connection)                // 用
// 闭包结束 → 变量销毁 → 自动回池（r2d2 的 PooledConnection drop = 归还）
```

### 4. 生命周期

- **出生**：启动时 `Backend::open` → `bootstrap_repository_pool`（先迁移后建池），刚建好 0 条连接（懒加载）；
- **第一次借**：r2d2 叫 Manager.connect() 开新连接（打开 + 打扮）；
- **复用**：第二次借直接拿回那条（配置还在），只做 is_valid（SELECT 1）健康检查；
- **销毁**：① 进程退出/池子 drop（最后 owner 消失）② 连接坏了被淘汰重建 ③ 可配 idle_timeout/max_lifetime（Ora 用默认，不主动回收）；
- **上限**：r2d2 默认 10（`Pool::builder()` 没写 max_size），要改在 `RepositoryPool::new` 加 `.max_size(N)`；
- **复用安全**：每个操作是短事务，用完连接干净（panic 也自动回滚）。

### 5. 路径传递链（从环境变量到打开）

```
环境变量 ORA_DATA_DIR（config.rs 读，默认 ".")
  → data_dir.join("ora.sqlite3")          ← 路径诞生（config.rs 第 157 行）
  → build_backend(db_path, ...)           （web bootstrap.rs）
  → Backend::open(BackendPaths{...})      （backend bootstrap.rs）
  → DatabaseLocation::path(&path)         （location.rs：包成枚举）
  → RepositoryPool::new → pooled_path()   （connection.rs）
  → SqliteConnectionManager::new(path)
  → Connection::open(path)                ← 真正使用
```

桌面版：`app_data_dir.join("ora.sqlite3")`（src-tauri/lib.rs）。

## 三、repository 读写（翻译官）

### 1. 每个方法 = 借连接 + SQL + 映射 + 错误包装

```rust
fn find_project(&self, project_id: &ProjectId) -> Result<Option<Project>, RepositoryError> {
    self.pool.with_connection(|connection| {
        let mut statement = connection.prepare(
            "SELECT id, name, root_path, created_at, updated_at, is_deleted
             FROM projects
             WHERE id = ?1 AND is_deleted = 0",
        )?;
        let mut rows = statement.query(params![project_id.as_ref()])?;
        match rows.next()? {
            Some(row) => Ok(Some(map_project_row(row)?)),
            None => Ok(None),
        }
    })
    .map_err(project_repository_error_from_database)
}
```

### 2. 行映射（SQL 行 → 领域模型）

```rust
fn map_project_row(row: &Row<'_>) -> Result<Project, crate::DatabaseError> {
    let is_deleted = row.get::<_, i64>("is_deleted")? != 0;   // 0/1 → bool
    Ok(Project::new(
        ProjectId::new(row.get::<_, String>("id")?),           // TEXT → newtype
        row.get::<_, String>("name")?,
        row.get::<_, String>("root_path")?,
        AuditFields::new(row.get("created_at")?, row.get("updated_at")?, is_deleted),
    ))
}
```

### 3. 软删除的"两道门"（前端永远不知道 is_deleted）

1. **查询过滤**：所有 SELECT 带 `AND is_deleted = 0` → 已删的进不来；
2. **映射丢弃**：application 的 `map_project` 丢掉 audit_fields → 契约/JSON 里没有。

### 4. 其他要点

- `soft_delete` 返回 bool（区分"删成功" vs "本来就不存在"），不报错；
- create/update = 全快照替换；
- 错误包装：DatabaseError → RepositoryError（保留 source 链）；
- **参数化查询**（`?1` + `params!`）防 SQL 注入：值永远是数据，不会被当成代码执行（拼接字符串则可能被注入 `'; DROP TABLE ...; --`）。

## 四、SQLite → handler 传递链（最易忘，背下来）

```
路径（BackendPaths）→ 池子（RepositoryPool）→ 真仓储（SqliteProjectRepository）→ handler 抽屉（repository 字段）
```

```rust
// Backend::open [crates/backend/src/bootstrap.rs]
let pool = DatabaseBootstrapper::system().bootstrap_repository_pool(db_path, catalog)?;
// ProjectApi::new [crates/backend/src/project.rs]
let repository = SqliteProjectRepository::new(pool.clone());
CreateProjectHandler::new(repository.clone(), UuidProjectIdGenerator::new(), clock)
```

自愈命令：`grep -rn 'SqliteProjectRepository::new\|CreateProjectHandler::new' crates apps`

## 五、命名陷阱（这一课踩的坑）

`self.project.get(request)` 里：
- `self.project` = **字段**（Arc<ProjectApi>）；
- `.get(...)` = **方法**（ProjectApi::get）。
ProjectApi 内部 `self.get.handle(request)` 里：
- `self.get` = **字段**（GetProjectHandler）；
- `.handle(...)` = **方法**。

判断方法：后面跟 `(` 的是方法调用，后面跟 `.` 再跟别的的是字段访问。

## 六、术语表新增/深化

Connection Pool（连接池，本课正式条目）、Migration（已深化：对账/三道检查）、PRAGMA（SQLite 设置）、SQL Injection（SQL 注入，已口头讲）、r2d2/rusqlite（第三方库，了解即可）。

## 七、下一课预告

> Task 与 Git Worktree：创建任务时怎么建 linked worktree？Task/Worktree 怎么关联？Git 失败时怎么补偿数据库写入？为什么删任务不删 Git？
