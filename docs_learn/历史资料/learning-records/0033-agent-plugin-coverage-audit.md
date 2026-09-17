# Agent 插件学习覆盖审计

## Learner request

用户在完成共享 Agent connection 的 Session 路由课程后，请求排查 Agent 插件是否还有遗漏点。

## Audit boundary

- Agent 插件本体：包、Manifest、`main.js`、运行时合同、CLI 启停与 ACP 透明转发。
- 紧邻的 Agent Runtime：Supervisor、ACP initialize、连接恢复、模型发现与多 Session 路由。
- 更上层 Session Runtime：warm/attach/load/prompt/cancel/stop/delete、history 与切换 Agent。此层可作为面试延伸，但不把全部细节算作 Agent 插件必修缺口。
- 审计以主动复述为掌握标准；课程中只出现过的名词不自动标记为掌握。

## Strong coverage

- 安装期两阶段验证、`InstalledPlugin` 与类型化 Contribution。
- Deno `main.js` 与真实 Agent CLI 的双进程适配模型。
- Manifest kind 与 `ora/register` 运行时能力合同的区别。
- Plugin RPC 控制面与 ACP notification 数据面的分层。
- Lifecycle、Connection Supervisor、Session actor 三层所有权。
- Plugin generation 作为干净重试与旧消息隔离边界。
- 一个 Agent connection 服务多个 Session 时的 pending correlation、RouteRegistry、独立队列、generation 与 route token。

## Mentioned but not yet retrieval-checked

- `Unavailable` / `Failing`、指数退避、restart circuit 与确定性合同失败的完整决策语义。
- 从插件 Running 到 Agent Ready 的完整步骤中，ACP initialize 的能力协商含义。
- `agent/stop`、`ora/shutdown`、强杀与等待进程树退出的完整 teardown 顺序。
- `PluginId` 与 `AgentRef` 两种身份的稳定区分。
- Plugin process、Agent connection、Effect desired state 三种 generation 的统一对照。

## Missing topics

1. Agent 插件当前近似宿主权限的安全边界，以及 Host child-process capability 解决了什么、没有解决什么。
2. Agent 控制合同与 ACP 版本兼容；不完整或不支持时为何属于 terminal failure。
3. `agent/listModels` 与 ACP session config options 两条模型发现路径。
4. 安装、禁用、卸载时 Supervisor 集合和进程 generation 的运行期协调。
5. 插件作者使用 SDK 实现一个 Agent adapter 的最小闭环与错误约定。
6. 流控、超时、坏帧、权限请求等长期连接可靠性细节。

## Priority decision

面试优先补：故障决策、安全边界、对称 teardown、合同/能力协商。开发扩展检查表随后补。Effect/Skill 热加载继续作为旁支，不抢占 Agent 主线。

## Mastery status

这是覆盖审计，不新增掌握结论。下一课从“失败不是一种失败”开始，通过具体场景让用户判断状态、重试策略和影响范围。
