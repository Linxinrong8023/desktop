# 插件 Running 与 Agent Ready

## Learner hypothesis

用户推测插件虽然已经启动并处于 Running，但插件内容本身可能存在错误，因此 Agent 仍然不可用。

## Assessment

方向正确。需要补充：失败不只来自插件代码错误，也可能来自 Agent CLI 未安装、配置或环境错误、Agent 运行时合同缺失，以及 ACP initialize 失败。

## Teaching target

建立必要条件与充分条件的区别：插件 Deno 进程 Running 是 Agent Ready 的必要条件，不是充分条件。Lifecycle 管插件进程层；Connection Supervisor 管 Agent CLI/ACP 连接层。

## Mastery status

已掌握。用户能根据“`main.js` 正在运行但 Agent CLI 路径不存在”的场景，正确判断插件进程层可以是 Running，而 Agent 连接层是 Unavailable。

后续精度：这是连接建立过程中的状态快照。启动或 ACP 握手确认失败后，Supervisor 会请求 Lifecycle 停止失败的插件 generation，再按重试策略重新尝试；状态不会永久固定在该组合。
