# connection 与 ensure_running 的实际使用者

## Learner response

用户正确回答 Agent Supervisor 应调用 `ensure_running()`：已经运行就取得当前 generation 的 lease，没有运行则请求 Plugin Lifecycle 启动。随后追问 `connection()` 是否用于设置页读取插件状态。

## Verified answer

- 设置页不调用 `connection()`；它通过 `listInstalledPlugins` 获取 Lifecycle 的状态投影。
- 当前生产代码中，Effect worker 直接调用 Lifecycle `connection()`，只协调此刻存在的 live Agent generation。未运行时跳过，不把一次 Effect reconcile 变成 Agent 启动或可用性等待。
- Backend 的 `PluginGateway` 暴露了 `connection()`，但当前 Surface gateway trait 不暴露它；Workbench 方法调用使用 `ensure_running()`。
- Agent picker 的 ready/starting/unavailable/failing 来自 Connection Supervisor 的独立 `getAgentRuntimeStatus`，也不是 Lifecycle `connection()`。

## Mastery status

已掌握 `ensure_running()` 的 get-or-start 语义。用户进一步指出 Agent Supervisor 会主动建立连接，质疑“Effect 更新会意外启动闲置 Agent”的例子。

该质疑成立，原例子对当前 Agent runtime 过度简化。修正后的边界是：Effect worker 是全局 worker，每次 reconcile 针对一个 Workspace surface；Agent 连接是 application-scoped，并非 Workspace 专属。Effect 使用 `connection()` 是为了只协调此刻存在的 live generation，不把 reconcile 变成 Agent 启动或可用性等待。Backend 启动、失败、重启换代或显式停止期间仍可能没有 live generation；此时没有活跃消费者需要 barrier，下一代启动后读取已物化 surface。
