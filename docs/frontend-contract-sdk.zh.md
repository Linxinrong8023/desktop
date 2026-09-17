# 前端 Contract SDK

[English](frontend-contract-sdk.md) | 中文

`@ora/contracts` 是前端访问 Desktop IPC API 的唯一契约入口。Rust 拥有 DTO 结构、操作目录和生成的客户端连接代码；TypeScript 拥有共享客户端执行逻辑和 Tauri transport。

## 仅用于生成的端点元数据

`xtask` crate 在按 namespace 划分的目录模块中，以 `FrontendEndpoint` 声明每个面向前端的 IPC 操作。内部的 `frontend_endpoints()` 仅在 `cargo xtask export-contracts` 生成清单时汇总这些模块。每条声明包含：

- `operation_name`：协议层的扁平标识符，例如 `createTask`。
- `namespace` 和 `member_name`：操作在生成客户端中的位置，例如 `client.task.create`。
- `request_type` 和 `response_type`：TypeScript DTO 名称。
- `response_mode`：显式声明的 `Unary` 或 `Stream`，不从操作名称推断。

清单不包含特定 transport 的路由、查询或序列化元数据。前端类型由 `ora-contracts` DTO 生成，不来自领域实体或 adapter 内部结构。

自动市场刷新保留原生订阅。插件契约所有者（`crates/contracts/src/plugin/marketplace_sync.rs`）声明 `MarketplaceAutoSyncEvent`；app-shell 重新导出生成的 DTO，不再复制 payload。Desktop 在 `apps/desktop/src-tauri/bindings/marketplace_sync.rs` 中拥有原生路由，由 Rust 发送端与导出器共享。导出流程派生 Desktop 监听函数和 serde 序列化样本，`check:contracts` 覆盖两者。适配器测试消费这些序列化样本，宿主测试验证 Tauri 发送，shell 测试覆盖刷新状态、列表失效与订阅释放。路由与授权继续归 Desktop；此次不新增通用事件框架或 SDK stream。

## 生成流程

`cargo xtask export-contracts` 是规范生成入口，在 `packages/contracts/src` 中写入：

- `dto/`：`ts-rs` 为每类 Ora 契约生成一个 `.ts` DTO 文件；嵌入 ACP payload 的字段引用官方 `@agentclientprotocol/sdk` 类型，不再生成一份 ACP schema。
- `dto/error.ts`：生成的 request-id 和可辨识公共错误契约。
- `endpoints.ts`：生成的端点清单。
- `dto.generated.ts`：从 ts-rs 拥有的 DTO 文件推导出的统一导出入口，不维护第二份类型到文件目录。
- `client.generated.ts`：由操作 namespace 和响应模式生成的静态强类型工厂。

生成文件带有所有权头注释。`client.ts` 重新导出工厂及其接口，`client-runtime.ts` 实现共享执行逻辑；它们与 `transport.ts`、`index.ts` 均为手写代码。导出先在新的临时目录中生成，检查路径冲突，再替换发生变化的自有产物并移除过期产物。非生成器拥有的文件会保留，不能被新生成的同名文件覆盖。

`task export-contracts` 先执行 Rust 导出，再通过 `ts-to-zod` 从 `dto/error.ts` 派生 `error.schema.ts`。修改契约后需显式运行。`task check:contracts` 在临时目录中生成两层产物，对比内容和文件清单，不依赖 Git，也不修改工作区。对比时将 Windows CRLF 统一为 LF，避免 Git 换行策略造成误报；其他字符和文件清单必须完全一致。缺失、变化、过期的产物（包括未跟踪文件）都会使验证失败。`task test:frontend` 在 lint 和测试前运行该检查，不自动修复过期产物。

`cargo xtask check-contracts` 失败时使用 `failed to check contracts` 前缀；`cargo xtask export-contracts` 使用 `failed to export contracts`。两者都保留底层错误详情。

contracts、chat 和 editor 包测试先运行 `tsc --noEmit`，再通过 `deno test --no-check` 直接执行 TypeScript 测试。共享的 `run-with-clean-stderr.ts` 包装器拒绝意外 stderr。相对引用显式写出源文件扩展名和目录入口文件，无需宽松的 import 解析或中间 JavaScript 产物。契约生成保留 `.ts` 扩展名。

## 强类型客户端

`createContractsClient(transport)` 返回按 namespace 组织的客户端，其结构由生成清单推导：`ContractsClient` 将每个端点的 `namespace`/`memberName` 映射为嵌套对象类型，生成的静态工厂接受该类型的检查。新增操作后重新生成即可更新工厂，无需手工编辑转发方法。重复操作或 namespace 成员会使生成失败。

namespace 包括 `project`、`task`、`session`、`appEvents`、`agentRuntime`、`skill`、`skillImport`、`agent`、`fileSystem`、`gitIdentity`、`workflow` 和 `workflowRun`。`appEvents` 提供 `client.appEvents.watch({})`；流以 `Ready` 开始，随后尽力投递 `SessionTitleUpdated` 等失效通知。

每次调用都将操作名称和完整请求 DTO 交给注入的 transport，不为 transport 拆分、重命名或序列化字段。每个 `ContractTransportRequest` 仅携带操作名称和原始完整请求 DTO，Tauri transport 将其原样转发给对应 command。

## Transport

`ContractTransport` 提供 `send` 和 `stream`，均通过 `ContractCallOptions` 接收可选的 `AbortSignal`。流采用冷启动且只能消费一次；第二次迭代同一个 `AsyncIterable` 会抛出 `stream_already_consumed`。

**Desktop**：`apps/desktop/web` 导出 `createTauriTransport`，通过 `createContractsClient(createTauriTransport())` 注入。它将操作名称映射为 snake-case Tauri command，并通过 Tauri Channel 使用统一的私有帧结构驱动流。单次响应失败和流错误帧使用共享解码器。

Desktop 拥有的 `src-tauri/bindings/` 目录提供 handler 路径和显式授权，导出时与逻辑操作目录组合，生成 `tauri-bindings.generated.ts`、Rust command 注册、强类型流路由及独立权限清单。公共清单不暴露这些 adapter 元数据。绑定缺失、重复、未知或响应模式不匹配都会使生成失败。新增操作无需手工编辑 transport 的 command map、流名称联合类型或权限。

`watchAppEvents` 是尽力投递的失效广播，不是可重放事件日志；不携带浏览器所有权元数据，允许多个 transport 并发订阅。Web App Shell 挂载普通查询或打开此流之前，平台 adapter 必须取得同源 `ora:app-window` Web Lock。第二个标签页等待锁，在活动文档关闭或重载后自动进入。Desktop adapter 立即授予所有权，因为原生 host 拥有单个主应用窗口。

## 公共错误与本地化

Rust 直接导出 `{ code, params, requestId }` payload。`decodeRemoteError` 先验证基本结构和 UUID，再检查生成的可辨识联合类型：

- 已知 code 且参数有效：转为 `RemoteContractError`。
- 未知 code 且 request id 有效：转为 `UnknownRemoteError`，保留原始 code 供诊断。
- request id 缺失或无效，或已知 code 的参数无效：转为 kind 为 `malformed_response` 的 `LocalTransportError`。
- 网络、Tauri 调用、帧处理、流消费、不支持的操作和取消失败：使用有限的本地 kind，不伪造 request id。

App shell 使用统一的 `localizeContractError` 路径和对应的中英文 `errors.<code>`、`errors.unknown`、`errors.transport.<kind>` 资源。内部错误与未知错误包含有效 request id，便于支持排查；本地 transport 错误不显示伪造 id。UI 不展示技术性的 `Error.message`，现有的受限语法 lint 规则负责约束这一边界。

## 排除的操作

生成 SDK 仅暴露受支持的公共操作。Task worktree 是后端拥有的内部状态，因此清单没有 `/api/worktrees` 端点，也没有独立的 worktree 客户端方法。Task 暴露其 Workspace 标识，worktree 保持为内部一对一扩展。参见 [Task Worktrees](task-worktrees.md)。

另见 [Application and Contracts Boundary](application-contracts-boundary.md) 和 [Desktop Runtime](desktop-runtime.md)。
