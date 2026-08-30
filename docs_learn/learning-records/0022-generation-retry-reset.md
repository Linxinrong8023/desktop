# Generation 与干净重试

## Learner feedback

用户知道旧 generation 的消息应丢弃，但明确表示仍不理解 generation 本身，也不理解为什么不能在当前 `main.js` 进程里无限调用 `agent/start`。

## Important distinction

“Actor 只消费当前 generation”是 generation 隔离的结果，不是 generation 的定义。Generation 表示一个 Plugin ID 的一次完整插件进程生命，包含该代 Deno `main.js`、不可变注册、子进程树、外层 RPC 连接和通知流。

## Concrete reason for replacement

启动失败可能是部分成功：`main.js` 已拉起 CLI 并建立部分监听，但 `agent/start` 响应丢失或超时。Host 无法判断副作用是否发生；在同一进程重复 start 可能产生第二个 CLI、残留管道或混合通知。当前合同不保证 `agent/start` 幂等或提供完整 reset，因此 Ora 将整个 generation 作为故障隔离边界，回收进程树后从干净状态重建。

## Retry policy verified from code

- `agent_not_installed`、禁用或暂时不存在：不计入 crash circuit，在 Backend 生命周期内持续指数退避重试；初始 250ms，最长 30s。
- 真正启动或连接故障：一分钟内第四次失败打开 circuit，状态为 Failing，并停止本次 Ora 进程内的自动重试。
- 合同不完整：确定性终止错误，立即 Failing，不重试。

## Mastery status

已掌握“部分成功导致原进程状态不可信”这一核心理由。用户能够解释：Ora 只观察到 `agent/start` 失败，无法确定 CLI A 是否已被拉起；如果不回收旧 generation 就启动新的 `main.js`，可能留下两个 Agent CLI，造成消息归属和运行状态不确定。

精度补充：当前实现不会直接让新旧 `main.js` 并存，而是先尽力执行 `agent/stop`，再由 Lifecycle 回收旧 generation 的完整进程树并等待退出，之后才允许启动替代 generation。旧 generation 消息即使迟到也不会交给新连接。
