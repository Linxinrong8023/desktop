# 已掌握 Agent 返回消息的分层路径

## Learner response

用户完整复述：main.js 将 Agent 返回包装成 `agent/acp` notification；Plugin Runtime 解析插件与 Ora 之间的 JSON-RPC；ACP Peer 读取真正的 ACP 消息；Session actor 转换成 Ora event；前端读取事件并渲染，最终形成用户可见的 Agent 对话。

## Demonstrated understanding

- 区分外层 Plugin JSON-RPC 与内层 ACP。
- 理解 Plugin Runtime 只解析外层协议。
- 理解 ACP Peer 负责 ACP 业务解析与 correlation。
- 理解 Session actor 将 ACP 更新映射到 Ora 的会话事件。
- 理解前端不直接处理插件协议，而是渲染 Ora events。

## Precision retained

main.js 从 Agent-specific 代码看调用 `notify("agent/acp", frame)`；Plugin SDK 实际编码外层 JSON-RPC 与二进制帧。因此可将 main.js + SDK 合称 Adapter 层，但精确分析时应分开。

## Mastery status

一条 Prompt 的主要返回链已掌握。下一检查点：理解插件进程启动后的 `ora/register` 能力注册以及为什么注册不可变。
