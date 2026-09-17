# Stream 取消完成确认

[English](stream-cancellation.md) | 中文

取消分为三个不同阶段：

1. **已发送请求：** Desktop 取消注册项的 token，或 backend stream 向 actor 发送带操作编号的取消命令。此时不能确认清理完成。
2. **已触发清理：** 创建结算或转发退出后，资源所有者开始清理。`Drop` 仍然只是尽力清理，不提供完成确认。
3. **已确认完成：** 操作所有者确认停止、事件发布者释放资源、附加的 workflow 状态恢复成功返回。只有此后 `cancel_contract_stream` 才返回成功。

Backend 调用方可使用 `SessionEventStream::cancel_and_wait(&mut self)`。取消只触发一次，重复调用返回同一结果。丢弃等待 future 不会中止其清理任务。Backend 调用方可用 `tokio::time::timeout` 设置自己的等待期限。

Desktop 的每次 `cancel_contract_stream` 最多等待 30 秒。超时只结束本次等待，不中止创建、清理或其他等待者。清理期间注册项仍被占用。并发等待者观察同一个注册项，后续调用可以读取最终回执。注册表以 256 项为清退阈值缓存已完成回执，进行中的注册项不会被清退。未知或已清退的编号返回错误，不推定成功。新操作应使用新的 stream call ID。

创建阶段，如果 source 尚未被轮询就取消，不会创建资源。一旦开始创建，Desktop 等待创建结束，再等待所得 stream 的清理。创建或清理错误会同时保留给启动命令和取消等待者。运行中的 stream 在清理结算后才发送终止帧并记录生命周期结束。原生文件监听器在创建取消和运行转发两条路径均等待 `ora_fs::WorkspaceWatcher::close()`。公开 watcher 的 Drop 和宿主 worker 退出均不代表原生资源释放。close 等待原生事件处理器的销毁回执，确认原生句柄和回调捕获的资源已经释放；回执缺失或 panic 均返回错误。

Actor 确认属于具体操作代次。Prompt 必须得到 provider 的终止响应；既有取消宽限期到期或 provider 失败，即使已完成本地隔离，仍然无法确认。后续 turn 的成功不能替旧操作证明成功。Actor 最多保留 256 个操作回执，证据缺失时保守返回错误。取消 load 会解除订阅并等待 relay 退出，不取消它跟随的 prompt。历史回放和应用事件 worker 也会确认任务终止。Workflow 清理等待 `end_human_turn`，包括其持久化结果。可复用的共享 actor、provider 连接、其他 stream 和用户文件仍由原有模块负责。

终止帧和请求生命周期保留原业务失败。例如，清理同时返回 `agent_runtime_unavailable` 时，原有的 `session_history_degraded` 不会被覆盖；取消回执独立报告清理错误。只有不存在原业务失败时，清理错误才成为终止错误。

原生回执依赖精确锁定的 notify 8.2.0 的已核对析构顺序；平台核对记录位于 `crates/fs/src/watch/retirement.rs`，升级时须重新核对。Linux 测试在 Desktop 创建和转发两条真实路径中阻塞 notify 回调，并通过独立子进程核查：阻塞时 inotify FD 仍存在，close 返回前 FD 已释放。macOS 和 Windows 路径已静态检查，但未在本 Linux 环境实际运行。

清理失败、所有者回执缺失、清理 worker panic、等待超时均返回错误。Desktop 复用现有契约错误和 request ID：基础设施或确认失败使用 `internal_error`，actor 不可用使用 `agent_runtime_unavailable`，领域恢复错误保留原有错误码。诊断会区分恢复失败、确认缺失或已过期，以及 `stream cleanup wait timed out; cleanup remains active`。错误不证明清理完成；仅发送 abort 或放弃 SDK 迭代器也不等于取得清理回执。

`todo-08b8a11f` 验证（2026-09-14）：通过可控阻塞点分别验证创建、所有者确认、发布者释放和恢复过程；另覆盖恢复失败、确认缺失、放弃等待、并发／重复取消，以及暂停时钟下超时后再取得成功确认。已核对基线 `4a5c77f` 和本地 `d781bf5e`。实际命令结果随任务交付报告记录。

评审修复后的实际验证结果（2026-09-14，Linux）：

| 命令／阶段  | 结果                                                                                                  |
| ----------- | ----------------------------------------------------------------------------------------------------- |
| `task test` | 全部阶段通过，退出码 0                                                                                |
| Rust 工作区 | 格式、模块体积、Clippy 和 1,373 项测试通过；1 项既有测试忽略。独立 inotify 检查另在子进程中执行一次。 |
| Desktop     | Clippy 和 87 项测试通过                                                                               |
| E2E         | Clippy 和 12 项测试通过，含 workflow 持久化恢复及 load／prompt 隔离                                   |

回归证据：`cargo test -p ora-desktop native_release` 在原生释放回执修复前，两条创建／运行测试均失败；修复后均通过。`cargo test -p ora-desktop business_failure_survives` 先复现 `agent_runtime_unavailable` 覆盖 `session_history_degraded`，修复后通过，原终止帧和独立清理失败回执均得到保留。`cargo test -p ora-fs watch::tests` 的四项测试全部通过，包含独立 inotify FD 检查、回执缺失和原生 worker panic。

初版的模块体积检查失败已通过拆分清理／load 职责并降低体积基线解决。最终全量执行包含两项评审修复。未执行提交、切换分支、合并或删除工作树。
