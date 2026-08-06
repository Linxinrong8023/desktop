# 第 4 课：错误体系 + 请求生命周期 + Backend 生命周期（总结）

> 对应对话内容：ApplicationError → BackendError → PublicError 三层错误、ErrorClassification、RequestLifecycle（只记一次）、axum 机制（IntoResponse/中间件/提取器）、Backend 共享与关停、流式响应错误（Error 帧 + DeferredCompletion）。

## 一、为什么错误也要分三层

| 层 | 错误类型 | 说哪种话 | 例子 |
|---|---|---|---|
| 应用层 | `ApplicationError` | 业务话 | "项目不存在" |
| 后端层 | `BackendError` | 翻译话 | 分类 + 公开错误 + 内部链 |
| 契约层 | `PublicError` | 协议话 | `{code:"project_not_found", params:{}}` |

应用层不该关心 HTTP；契约层不该暴露内部诊断；后端层当翻译官。

## 二、BackendError 四字段（各有消费者）

```rust
pub struct BackendError {
    classification: ErrorClassification,  // 分类 → HTTP 状态码 + 日志级别
    public_error: PublicError,            // 公开错误 → 前端 {code, params}
    context: String,                      // 上下文 → 日志一句话
    source: Option<SharedError>,          // 根源链 → 开发者诊断
}
```

## 三、ErrorClassification（传输无关）

```rust
pub enum ErrorClassification { InvalidRequest, NotFound, Conflict, Internal }
```

- Web 适配器映射：InvalidRequest→400、NotFound→404、Conflict→409、Internal→500；
- 日志级别：Internal→error、Conflict→warn、其他→info；
- **backend 不认识 HTTP**——分类是抽象的，状态码由适配器映射。

## 四、总翻译表：From<ApplicationError> for BackendError

- 语义错误精确映射（ProjectNotFound → NotFound + project_not_found）；
- 基础设施错误统一收口（所有 Repository 失败 → Internal + internal_error，不给前端细分）；
- **不检查 source 链**：判断只看枚举变体，绝不靠错误字符串（脆弱）；
- source 链保留在 BackendError 里 → 进日志（完整 chain），不进对外 JSON（contract_error 只装 error + request_id）。

## 五、错误回程（每个转换点就是代码里的 map_err）

```
业务 handler 产生 ApplicationError
  → .map_err(BackendError::from)    [crates/backend/src/bootstrap.rs]  ← 转换点①：查总翻译表
  → .map_err(WebApiError::from)     [apps/web/server/src/handlers/*.rs] ← 转换点②：包一层
  → axum 调 into_response           [apps/web/server/src/error.rs]      ← 转换点③
      → complete_failure（记日志，抢一次）
      → status_for(分类) → 404
      → contract_error(request_id) → {"code","params","requestId"}
  → 前端 decodeRemoteError → i18n 翻译 → 显示
```

**看返回类型 = 看这一层说错误的语言**（handle→ApplicationError，Backend 方法→BackendError，门卫→WebApiError）。

## 六、axum 机制（约定优于配置）

- **axum** = Rust 的 Web 框架（Router/handler/提取器/IntoResponse 都是它提供的）；
- **IntoResponse 规则**："handler 返回的任何东西，只要实现 IntoResponse，框架自动调它的 into_response() 变成 HTTP 响应"——框架不认识 Ora 自定义类型，用 trait 当通用插座；
- **提取器**：`State<AppState>`、`Json<T>`、`Path<T>` = "声明要什么，框架自动给"；
- 与第三课 `Backend::open` 的"组合根"、`State` 注入是一套思想。

## 七、RequestLifecycle（只记一次）

```rust
struct RequestLifecycleInner { request_id, operation, started_at, completed: AtomicBool }
```

- `start(operation, generator)`：生成 request_id（UUID）、记开始时间；
- `claim_completion()`：`compare_exchange(false, true)`——谁先抢到 false→true 谁记录，别人看到 true 就跳过；
- 日志带：request_id + operation + duration_ms + outcome + error.code + error.chain；
- 中间件（`request_context`）在请求进来时 start、塞 x-request-id 头、开 span；响应返回时若无 DeferredCompletion 则 complete_success。

## 八、Backend 生命周期（共享与关停）

- `Backend` 是 `derive(Clone)` + 内部全 `Arc` = **克隆只是复印取物卡，内容共享**（Web/桌面/测试各持一份）；
- **关停机制**（connection.rs 第 290 行）：

```rust
impl Drop for ConnectionSupervisor {
    fn drop(&mut self) {
        if self.shutdown.strong_count() == 1 {   // 最后一份钥匙被丢
            let _ = self.shutdown.send(());       // 按开关 → 运行时线程退出 → 终止回收子进程
        }
    }
}
```

- `strong_count()` 数"还有几份"——**必须等最后一个 owner 才关停**，不会一个窗口关了后厨就断电；
- 关停链：最后一个 Backend drop → AgentRuntimeManager 引用归零 → 每台 CLI 的 supervisor Drop → 按开关 → terminate_and_reap。

## 九、流式响应错误（Error 帧 + DeferredCompletion）

**问题**：流式响应已经开始（HTTP 200 已发出），中途失败没法改状态码 → 错误作为**流里的帧**发出去。

```rust
enum StreamFrame<Event> { Data{data}, Error{error: ContractError}, End }
```

- `Ok(event)` → `{"type":"data","data":{...}}`；
- `Err(error)` → `complete_failure` + `{"type":"error","error":{code,params,requestId}}` → 结束；
- `None`（通道关闭）→ `complete_success` + `{"type":"end"}` → 结束；
- 每行一个帧，`application/x-ndjson`；
- **DeferredCompletion** = 完成权转让证书：`response.extensions_mut().insert(...)` 后，中间件看到标记就不记完成，由流内部自己记——**无论如何结束，只记一次**；
- 前端 fetch.ts 的 `readNdjsonStream` 读帧：data→yield、error→抛、end→结束。

## 十、两种错误路径对比

| | 普通请求（unary） | 流式请求（stream） |
|---|---|---|
| 错误发生在哪 | 响应发出前 | 可能响应已开始 |
| 错误怎么传达 | HTTP 状态码 + JSON body | 流里的 Error 帧 |
| 完成日志谁记 | into_response | 流内部 |
| 中间件 | 自己记 success | 看到 DeferredCompletion 不插手 |

## 十一、导航地图（错误回程关键文件 + 行号）

| 文件 | 看什么 |
|---|---|
| `crates/application/src/project/handlers.rs` | 错误产生处（第 90 行 Err(ProjectNotFound)） |
| `crates/backend/src/project.rs` | ProjectApi::get（原样通过） |
| `crates/backend/src/bootstrap.rs` | Backend::get_project（map_err(BackendError::from)） |
| `crates/backend/src/error.rs` | BackendError 结构体（25）、From 总翻译表（117）、contract_error（96） |
| `apps/web/server/src/handlers/projects.rs` | 门卫 handler（map_err(WebApiError::from)） |
| `apps/web/server/src/error.rs` | From<BackendError>（194）、into_response（213）、status_for（266）、中间件、DeferredCompletion |
| `crates/backend/src/request_lifecycle.rs` | start（38）、complete_failure（85）、claim_completion（147） |
| `crates/backend/src/agent_runtime/connection.rs` | ConnectionSupervisor::start、Drop（290）、run_supervisor |
| `apps/web/server/src/handlers/sessions.rs` | stream_response（StreamFrame、DeferredCompletion） |

## 十二、术语表新增

Middleware（中间件）、IntoResponse（已在课内解释）、Error Chain（错误链/source 链）、Stream Frame（流帧）、AtomicBool/compare_exchange（原子操作）、Arc（共享引用）、strong_count（引用计数）、NDJSON（已在待讲清单，本课补全）。

## 十三、下一课预告

> 数据库持久化（`ora-db`）：迁移（migration）怎么工作？连接池怎么管理？每个实体的 repository 怎么读写？聚合删除（cascade）为什么需要事务？软删除怎么隐藏？
