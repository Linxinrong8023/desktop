# Agent CLI 进程由谁启动

## Learner response

用户理解为：Agent 插件要启动的真实 Agent 进程，最终由 Ora 的 `ora-process` crate 创建。

## Verified architecture

理解正确。插件 `main.js` 决定要运行哪个命令和参数，并通过 `ora/childprocess/spawn` 向 Host 发出请求；Plugin Lifecycle 中的 `PluginProcessHost` 处理请求，生产环境使用 `ora_process::TokioProcessSpawner` 创建 OS 进程并持有 stdio。该进程被绑定到当前插件 generation，generation 结束时由 Host 执行进程树级回收。

## Mastery status

已掌握 Agent CLI 进程启动中的职责拆分：插件决定启动意图，Ora Host/`ora-process` 执行并拥有真实 OS 进程。
