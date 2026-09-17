# Stream cancellation confirmation

English | [中文](stream-cancellation.zh.md)

Cancellation has three distinct stages:

1. **Requested:** Desktop cancels the registration token, or a backend stream sends an operation-specific actor command. This does not confirm cleanup.
2. **Triggered:** startup settles or forwarding exits, and the resource owner begins cleanup. `Drop` remains best effort and provides no confirmation.
3. **Confirmed:** the operation owner acknowledges stopping, event publishers have released their resources, and attached workflow restoration has returned successfully. Only then does `cancel_contract_stream` return success.

Backend consumers can call `SessionEventStream::cancel_and_wait(&mut self)`. Cancellation is initiated once; repeated calls return the same result. Dropping the waiting future does not abort its cleanup task. Backend callers can apply their own deadline with `tokio::time::timeout`.

Desktop waits up to 30 seconds per `cancel_contract_stream` call. A timeout ends that waiter, not startup, cleanup, or another waiter. The registration stays owned while cleanup runs. Concurrent waiters observe the same registration, and a later call can retrieve the eventual receipt. Completed receipts are cached within the registry's 256-entry pruning threshold; active registrations are never pruned. Unknown or evicted identifiers produce an error rather than inferred success. Use a fresh stream call ID for a new operation.

During startup, cancellation before the source is polled creates no resource. Once creation begins, Desktop waits for it to settle, then awaits the resulting stream's cleanup. Startup and cleanup errors are preserved for both the startup command and cancellation waiters. Running streams defer terminal frames and lifecycle completion until cleanup settles. Native filesystem watchers in both startup and forwarding await `ora_fs::WorkspaceWatcher::close()`. Public watcher Drop and host-worker exit do not confirm native resource release. The close operation waits for the native event-handler retirement receipt after the native handles and callback captures have been released; missing receipts and panics return errors.

Actor confirmation belongs to an operation generation. A prompt needs a terminal provider response; expiration of the existing cancellation grace period or provider failure remains unconfirmed even after local isolation. A later successful turn cannot validate an earlier failed operation. The actor retains at most 256 operation receipts; missing evidence fails conservatively. Cancelling a load detaches and joins its relay without cancelling the prompt it follows. Replay and application-event workers also confirm task termination. Workflow cleanup awaits `end_human_turn`, including its persistence result. Shared reusable actors, provider connections, other streams, and user files remain under their existing owners.

The original business failure takes precedence in the terminal frame and request lifecycle. For example, `session_history_degraded` stays intact if cleanup also returns `agent_runtime_unavailable`; the cancellation receipt independently reports the cleanup error. Cleanup errors supply the terminal error only when there was no original business failure.

The native receipt depends on the audited destructor ordering in exactly pinned notify 8.2.0; the platform audit is recorded in `crates/fs/src/watch/retirement.rs` and must be repeated on upgrade. Linux tests block the real notify callback through both Desktop startup and forwarding, and an isolated subprocess checks that the inotify FD stays allocated while blocked and disappears before close returns. macOS and Windows destruction paths were statically inspected; they were not executed on this Linux host.

Cleanup failure, a missing owner receipt, a panicked cleanup worker, and waiting timeout all return errors. Desktop uses the existing contract error and request ID: infrastructure/confirmation failures use `internal_error`, an unavailable actor uses `agent_runtime_unavailable`, and domain restoration errors preserve their existing code. Diagnostics identify failed restoration, missing or expired evidence, or `stream cleanup wait timed out; cleanup remains active`. An error never certifies that cleanup completed. An abort signal or an SDK iterator being abandoned alone is not a cleanup receipt.

Validation for `todo-08b8a11f` (2026-09-14): controlled gates test creation, owner acknowledgment, publisher release and restoration independently; additional tests cover failed restoration, missing confirmation, abandoned waiters, concurrent/repeated cancellation, and a paused-clock timeout followed by successful confirmation. Baselines `4a5c77f` and local `d781bf5e` were inspected. Actual command results are recorded with the task delivery report.

Actual validation results after review fixes (2026-09-14, Linux):

| Command / stage | Result                                                                                                                                              |
| --------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| `task test`     | Passed all stages, exit code 0                                                                                                                      |
| Rust workspace  | Passed formatting, module size, Clippy, and 1,373 tests; 1 existing ignored test. The isolated inotify check also executes once in a child process. |
| Desktop         | Passed Clippy and 87 tests                                                                                                                          |
| E2E             | Passed Clippy and 12 tests, including durable workflow restoration and load/prompt isolation                                                        |

Regression evidence: `cargo test -p ora-desktop native_release` failed both startup and running tests before the native receipt fix, then passed both. `cargo test -p ora-desktop business_failure_survives` first reproduced `agent_runtime_unavailable` replacing `session_history_degraded`, then passed while preserving the original terminal frame and the independent cleanup failure receipt. `cargo test -p ora-fs watch::tests` passed all four tests, including the isolated inotify descriptor check, lost receipt, and native-worker panic.

The initial implementation's module-size failure was resolved by extracting cleanup/load responsibilities and lowering the size baseline. The final full run includes both review fixes. No commit, branch switch, merge, or worktree deletion was performed.
