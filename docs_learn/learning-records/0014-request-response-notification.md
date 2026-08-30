# 需要从零解释 request/response 与 notification

## Learner feedback

用户明确表示不理解“ACP 已有请求 ID、响应、取消和顺序，因此外层使用 notification 避免双重 ID/超时/取消”的解释。

## Interpretation

不能假设用户已经理解 JSON-RPC 的 request/response correlation。需要先建立：request 带 id 并等待同 id response；notification 无 id，只负责投递。然后才能解释 ACP 作为内层协议已经拥有自己的 id。

## Concrete model

- `agent/start`：外层 Plugin request id 7，Backend 等待外层 response id 7，适合短控制调用。
- `session/prompt`：内层 ACP request id 42，可能长时间运行并由 ACP response id 42 结束。
- `agent/acp`：无外层 id 的 Plugin notification，params 原样携带 ACP id 42。
- 若再加外层 id，立即回答只能确认投递；等待内层完成则受外层 timeout 干扰，并引入重复取消语义。

## Mastery status

基础已掌握。用户已能区分 `agent/acp` 使用 notification，而 start/stop 使用 request/response，并理解 Ora 与插件之间存在一层自定义外部通信格式。仍需校正：当前 Agent adapter 通常透明转发 ACP，而不是把 Ora 私有业务协议翻译成 ACP；返回的 ACP 由 Backend 上层 ACP Peer/Session actor 解析，不由通用 Plugin Runtime 解释。
