# Supervisor 与 Lifecycle 的职责边界

## Learner hypothesis

用户提出可以让 Supervisor 负责启动插件，并猜测抽出 Lifecycle 的主要原因是避免每个 Agent Supervisor 重复编写启动代码。

## Correction

多个 Supervisor 是同一份 `ConnectionSupervisor` 实现的多个实例，因此即使启动逻辑位于 Supervisor 实现中，也不会要求为每个 Agent 插件重写代码。代码复用不是单一所有权的主要理由。

Lifecycle 被单独设为进程所有者，是为了让 Agent 连接监督、设置页启停、插件更新与卸载等所有入口共享同一个真实运行状态，并确保同一插件只有一个受控进程代。Supervisor 负责 Agent 连接、ACP 路由和重试；Lifecycle 负责插件 Deno 进程的启动、停止和代际替换。

## Mastery status

已掌握 Lifecycle 单一所有权的核心动机：防止多个模块分别维护插件进程状态而产生状态冲突，让其他模块通过统一入口查询或请求操作，从而共享同一个可信事实。

需要继续保持的术语精度：Lifecycle 统一管理的是 Agent **插件进程**的运行状态，而不是整个 Agent 系统的所有状态。Agent ACP 连接状态由 Connection Supervisor 管理，具体聊天状态由 Session actor 管理。
