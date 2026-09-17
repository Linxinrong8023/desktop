# Workspace review 刷新

[English](workspace-review-refresh.md) | 中文

`state/data/workspace-review.ts` 拥有 `refreshWorkspaceReview(queryClient, workspaceId)`。
它统一失效指定 workspace 的全部 diff scope 和 staging status。活跃查询并发重新请求；
非活跃查询标记过期，待再次观察时加载。其他 workspace 的缓存状态不变。

Staging 和 commit mutation 在操作内部等待此接口，确保写入期间切换视图后仍刷新原始
workspace。手动刷新、prompt 结束后的清理，以及经过 debounce 的 Agent 文件修改完成、
回合完成事件复用同一接口。
事件消费不等待网络响应。

每次刷新先显式取消已有 diff/status 请求（包括尚无缓存的首次加载），再启动刷新触发后的请求。
即使 transport 忽略取消，旧响应也不能覆盖新快照。重叠刷新按 QueryClient 和 workspace 共享一个
完成信号：替代轮次的 diff 与 status 请求均结束（包括重试）后，所有先前调用者才结束等待。
由于 `invalidateQueries` 对 paused 查询立即返回，数据所有者同时捕获这一轮活跃请求的 Promise。
离线暂停期间 mutation 保持 pending，直至恢复在线后的实际成功或最终失败。
已被替代的轮次不能结算这些等待者；完成后释放协调状态，下一次刷新重新开始。

刷新失败保留为查询错误，
不会将已经成功的 Git 写入改判为 mutation 失败。视图呈现 diff/status 错误，允许手动重试；
staging 写入失败直接展示错误，不触发成功刷新。

行为测试通过生产 contracts client 和显式类型化 transport handler，以可控 Promise 验证：
`task-diff-view.review.test.tsx` 覆盖操作、pending、错误和 workspace 切换；
`use-workspace-diff-live-sync.test.ts` 覆盖 Agent 触发；`workspace-review.test.ts` 覆盖查询范围与隔离。
`task-diff-view.review-races.test.tsx` 覆盖三轮重叠、最新和已过时请求失败、B 隔离，以及无缓存时
写入前启动的 status 响应。它还使用生产 QueryClient 配置验证离线暂停、恢复在线、
刷新最终失败和暂停期间的重叠刷新。
