# 正在建立一条 prompt 的端到端路径

## Learner response

用户解释 `agent/acp` 通过 notification 传输，而 start/stop 使用 request/response；用户把外层协议理解为 Ora 与插件人为约定的通信格式，并推测 adapter 在专属格式与 ACP 之间转换，Ora 最后解析内容返回前端。

## Demonstrated understanding

- 已区分外层 notification 与 request/response。
- 已理解 `agent/acp` 把 ACP payload 包在 Ora Plugin Runtime 的外层协议中。
- 已意识到 adapter 位于 Ora 与真实 Agent CLI 中间。
- 已将 Agent 返回内容与最终前端展示链路联系起来。

## Precision corrections

- Backend 的 Session actor/ACP Peer 首先产生 ACP；Plugin transport 只把完整 ACP JSON 放入 `agent/acp` notification。
- `main.js` 当前通常只处理 CLI stdio framing 并透明转发 ACP；如果未来某 CLI 使用私有协议，插件可以翻译，但对 Ora 一侧仍必须表现为 ACP。
- Plugin Runtime 只解析外层 framed JSON-RPC；`plugin_agent` bridge 基本把 ACP payload 当不透明对象转发。
- 返回进入 Backend 后，由 ACP Peer 解析、按 ACP id 关联，再由 Session actor 将更新映射成前端契约和事件。

## Mastery status

用户已主动修正“转换”为“读取、包装和转发”，理解当前 main.js 不需要改变 ACP 业务协议。仍需保留精度：Plugin SDK 负责外层二进制 framing 与 Plugin JSON-RPC 的解封装/封装；Agent-specific main.js handler 接收已经解出的 ACP JSON，处理 CLI stdio framing 并转发；ACP Peer 才解析业务语义和关联请求。
