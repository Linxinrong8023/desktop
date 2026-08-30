# Effect idle 边界与 Agent restart

## Learner question

用户提出一个 turn 完成后已经可以修改 Skill surface，为什么修改后仍要 restart。
随后进一步指出此前总结的 Agent 插件四类基础能力里并没有 `restart`，追问实际由谁以及如何执行重启。

## Verified answer

这是两个不同目标：

- `effect/waitForIdle` 确认所有正在进行的 prompt turn 已结束，并建立 barrier 暂存随后到达的新 prompt，保证磁盘变更不会切断一个 turn。
- `effect/restart` 让 Agent 消费者重新加载刚写入的 Skill。当前 OpenCode CLI 只在启动时扫描 `.opencode/skills`，不会热加载；OpenCode 插件因此停止并重新拉起 CLI，然后按顺序重放 barrier 中暂存的 prompt。

该 restart 通常不替换 Deno `main.js` 的 Plugin process generation，而是在同一插件 generation 内替换 Agent CLI 子进程。`effect/restart` 参数中的 `generation` 是 Effect Desired State 的版本，不是 `PluginGenerationKey`。

重启不是所有 consumer 的普遍要求：能够安全热加载的 consumer 可以声明 `uninterrupted`；OpenCode 声明的是 `wait_for_idle_and_restart`，因为其实际加载语义需要重启。

这里需要区分基础 Agent 契约和条件性的 Effect 契约：

- 所有 Agent 插件都必须注册 `agent/start`、`agent/stop`、`agent/listModels` 和双向 `agent/acp`。
- 只有声明了 `coordination = "wait_for_idle_and_restart"` 的 Effect surface，才必须额外注册 `effect/waitForIdle` 与 `effect/restart`。
- Ora 调用 `effect/restart` 只是把重启命令交给插件；具体如何重新加载由插件实现。OpenCode 的实现仍保留 Deno 插件进程，在 handler 中调用 `OpenCodeClient.start(cwd)`；该方法先 `stop()` 旧的 `opencode acp` 子进程，再通过 Host Processes 拉起新的子进程。

## Mastery status

已掌握 OpenCode restart 的直接原因：OpenCode CLI 启动时扫描一次 Skill surface，运行过程中磁盘即使新增、删除或修改 Skill，当前 CLI 仍使用启动时加载的旧视图，必须重启 CLI 才会重新扫描。

后续仍需确认用户能完整区分：`waitForIdle` 负责安全修改边界，`effect/restart` 负责让磁盘上的新 Effect generation 被消费者读取。
