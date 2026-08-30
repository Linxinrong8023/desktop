# Agent 插件主线收束与下一课

## Learner request

用户指出连续追问导致课程主线混乱，要求先总结并回归正式课程，然后继续下一课。

## Course action

新增 Agent 插件阶段总结，将已掌握内容压缩为四段：安装与可信贡献、Agent 双进程运行、Plugin RPC/ACP 消息链、Lifecycle/Supervisor/Actor 所有权与 generation 隔离。

下一课只讲 Lifecycle 的两个连接入口：`connection()` 无副作用地查询当前运行代；`ensure_running()` 在消费者明确需要插件时执行 get-or-start 并等待，成功后返回 generation-bound lease。

## Mastery status

等待用户回答第十五课主动回忆题；不把浏览总结页视为掌握新知识。
