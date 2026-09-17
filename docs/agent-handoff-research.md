# Agent 上下文交接：协议与一手资料核实

核实日期：2026-09-10（Asia/Shanghai）。状态：方案研究依据，不代表 Ora 已实现。使用 `research` 与 OpenAI Docs 技能；只引用官方协议、官方 API 文档和第一方工程说明。父任务负责总体方案与提交准备。

## 结论

推荐将交接设计为 **Ora 持有的可移植任务状态 + 有预算的近期证据 + 可授权补读的历史**。供应商原生 compaction、ACP session/load、MCP 都没有单独提供“跨 Agent 无损恢复”的保证。任务状态提炼和预算控制需要由 Ora 实现；模型只能协助提炼，不能担任唯一的容量、安全和正确性门禁。

下文“事实”来自已读取的一手资料，“推论”是对 Ora 的设计建议。本文不宣称任意压缩都能保留所有未来可能重要的信息，也不把 Agent 内部状态、用户可见的思考摘要与完整思维链等同。

## 1. ACP 是会话接口，不是通用内部状态迁移协议

**事实。** ACP v1 通过 `session/new` 建立会话；`session/load` 要求 Agent 宣告 `loadSession`，使用该 Agent 认可的已有 Session ID。它没有定义把另一个供应商的内部会话序列导入当前 Agent 的通用格式。创建/恢复会话可传入工作目录和 MCP 服务配置；stdio 必须支持，HTTP/SSE 取决于能力。 [ACP Session Setup](https://agentclientprotocol.com/protocol/v1/session-setup)

**事实。** `session/prompt` 是用户消息及附加上下文的入口；内容格式受接收 Agent 的 `PromptCapabilities` 限制。一次 prompt 可能包含多个模型请求和工具执行；工具在 Agent 侧运行，也可能使用 Client 提供的文件或终端接口。 [ACP Prompt Turn](https://agentclientprotocol.com/protocol/v1/prompt-turn)

**推论。** 跨供应商交接应创建新 Agent 会话并发送可理解的交接材料。相同 Agent 的 native resume 可以作为优化，但需检查源会话身份、版本与能力，不能拿来替代可移植交接。图片、音频等必须能力协商；只传资源 URI 不代表接收方一定有读取权限或支持格式。

## 2. 摘要器不能仅靠提示词保证无工具副作用

**事实。** 当前稳定 ACP v1 的 `PromptRequest` 字段为 `_meta`、`prompt`、`sessionId`，没有标准化的 `tool_choice=none`、输出 token 上限、JSON Schema 约束生成或提交前 token 计数接口。 [ACP v1 Schema：PromptRequest](https://agentclientprotocol.com/protocol/v1/schema#promptrequest)

**事实。** Session modes 和 config options 由 Agent 提供。模式可能改变提示词、工具或审批行为，但不是统一定义的“禁止所有工具”执行合同。 [ACP Session Modes](https://agentclientprotocol.com/protocol/v1/session-modes)、[ACP Session Config Options](https://agentclientprotocol.com/protocol/v1/session-config-options)

**推论。** 复用用户已登录的 Agent 做摘要具有吸引力，但“不要改文件”不构成隔离。空 MCP 列表、拒绝 Client 文件写入也无法覆盖 Agent 自己的 shell/文件工具。摘要后端应采用已验证的无工具生成 API，或具有实际工具禁用和沙箱保证的专用适配器；未知能力的普通 ACP Agent 不能默认启用此路径。无可用后端时采用确定性抽取并明确降级，不暗中执行用户工作、也不假设订阅登录可直接用于独立 API。

## 3. 上下文容量可部分观测，但不是全链路精确预算

**事实。** ACP `usage_update` 于 2026-06-05 稳定，包含当前上下文占用 `used`、总窗口 `size` 与可选累计费用。没有有意义窗口数据的 Agent 可以不发送；动态窗口应更新数值。这里的 `size - used` 是一次观察的剩余量，不是指定下一条请求的完整预检结果。 [稳定公告](https://agentclientprotocol.com/announcements/session-usage-stabilized)、[已完成的 Session Usage RFD](https://agentclientprotocol.com/rfds/session-usage)

**本地核实。** Ora 已使用 `agent-client-protocol-schema = 1.6.0`；`HistoryAssembler` 明确不把 `UsageUpdate` 变成历史记录，测试覆盖这一点。不能由此推导协议没有容量字段。 [Cargo.toml](../Cargo.toml)、[assembler.rs](../crates/history/src/assembler.rs)、[assembler_tests.rs](../crates/history/src/assembler_tests.rs)

**事实。** OpenAI 提供 `POST /responses/input_tokens`，对其请求返回输入 token 数；Claude 提供包含 system、tools、图片/PDF 等结构输入的 token counting，官方明确它是估计值，实际可能稍有差异。计数需要针对目标模型，不能复用源模型数字。 [OpenAI Input Tokens](https://developers.openai.com/api/reference/resources/responses/subresources/input_tokens/methods/count)、[Claude Token Counting](https://platform.claude.com/docs/en/build-with-claude/token-counting)

**推论。** 上述 API 不自动揭示 Codex/Claude Code 等 Agent 最终拼装的隐藏系统提示、Skill、工具 schema 或内部重试。模型窗口、当前占用、最大输出、后续工具结果预留必须分开建模。预算来源至少区分“适配器实测”“模型配置估计”“未知”；模型/工具集合改变后旧数据失效。先计量 Ora 实际渲染的完整交接 payload，再留安全余量；未知环境只能保证 Ora 自己的字节/估算预算，不宣称保证服务端永不超限。超限重试必须确认上一请求未执行副作用，不能盲目重复。

## 4. 供应商 compaction 适合原生续接，不能直接成为通用交接格式

**事实。** OpenAI Responses compaction 产生包含不透明加密条目的窗口，官方要求 standalone compact 结果原样用于后续 Responses 调用；传给 compact 的输入本身仍须适配模型窗口。官方没有提供让其他厂商解释该加密条目的格式。 [OpenAI Compaction](https://developers.openai.com/api/docs/guides/compaction)

**推论。** 不解析、不二次“翻译” opaque item。允许其作为兼容原生续接的可选材料，跨厂商则提炼可见记录里的目标、约束、决策依据和证据。压缩器也有输入上限：长历史需要按完整事件组分块提炼，再按原始引用合并，不能一次把超长全文交给 summarizer。反复 A→B→C 应使用原始记录和可追溯的状态修订，避免只对旧摘要继续摘要。

## 5. 高质量摘要与按需证据检索是互补机制

**事实。** Anthropic 的 context engineering 文章将压缩、结构化笔记与即时检索作为长期上下文方法，明确提醒激进压缩会丢失后来才显得重要的细节；建议先在复杂轨迹上提高相关信息召回，再剔除冗余。其 Claude Code 示例保留架构决定、未解决问题和实现细节。该说明是工程经验，不是保真率保证。 [Anthropic，2025-09-29](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents)

**事实。** Anthropic 的长期 Agent 实验强调新上下文需要结构化进度、Git 历史和真实环境验证；仅有 compaction 仍会发生未完工却被判断完成的问题。 [Anthropic，2025-11-26](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents)

**推论。** 必须直接交接目标、最新明确约束、否定要求、决策变更、未完成事项、未决问题。原文引用服务于核验；仅有引用不能补救 Agent 不知道自己漏了什么。工具成功与业务验证成功分开；计划、已执行、已验证分开。模型提出但未经用户确认的方案不能自动升级为用户要求。语义保真需用标注轨迹评测：约束召回、事实依据、错误状态、连续切换漂移与下一步行动正确率；字数减少和摘要“看起来完整”不是通过标准。

## 6. MCP 历史补读可行，但会话隔离需要 Ora 自己实现

**事实。** MCP tools 支持输入 schema、结构化结果和输出 schema，且为兼容旧客户端建议同时返回文本表示。工具服务需要验证输入、实施访问控制与调用限制；schema 合法不代表内容事实正确。 [MCP Tools，2025-11-25](https://modelcontextprotocol.io/specification/2025-11-25/server/tools)

**事实。** MCP 安全说明明确禁止把协议 Session ID 当认证，授权服务需验证每次请求；应将会话与认证身份关联。授权规范区分 HTTP 和 stdio 的认证方式。 [MCP Security Best Practices](https://modelcontextprotocol.io/docs/2025-11-25/tutorials/security/security_best_practices)、[MCP Authorization，2025-11-25](https://modelcontextprotocol.io/specification/2025-11-25/basic/authorization)

**推论。** `history.search/read` 应由 Ora 绑定当前会话与交接快照；凭证或进程授权决定可读范围，Agent 不提供可任意遍历的 Session ID。证据句柄不能当访问令牌；句柄需逐次检查范围、撤销与快照归属。为 stdio Agent 可提供受限桥接；HTTP 则使用有效认证，不能假定 localhost 端口就是私有访问。

**推论。** 返回 `record_id/revision`、来源角色、事件类型、快照范围、正文片段、截断标记和分页游标；按 UTF-8 字节、条目数量和 token 估计双重限额，服务端有硬上限。敏感历史的发送继续遵守会话/供应商权限。工具历史中的指令属于历史数据，不升级为当前命令；同一限制也适用于摘要器输入。传输授权解决“谁能读取”，不能消除模型被历史文本诱导的风险。

## 7. 实施前必须验证的边界

这些是设计验收点，而非协议自动提供的能力：

1. **缺失或迟到事件**：工具结束状态、正文修订、取消与最后一条用户纠正必须纳入一致快照；未捕获内容明确标记，不能声称完整恢复。
2. **模型/工具变化**：能力不兼容、工具缺失、Skill 变化、窗口缩小、计数服务不可用时重算预算并显示降级。
3. **超大不可分条目**：单次错误输出、长用户要求、图片或二进制不能按字符盲切；需要有边界片段与可访问原件。不可缩减的关键内容超预算时停止自动交接。
4. **摘要失败**：离线、限流、非法结构、虚构引用、超时、取消和摘要器可能调用工具，都不能修改原工作现场或消费成功标记。
5. **注入与重放**：新 Agent 尚未确认收到、传输断线但已可能开始执行、重复切换、重启恢复，要避免丢交接或重复动作。
6. **检索失败**：历史损坏、授权撤销、附件删除、快照过期和引用失效明确返回错误；不能将空搜索结果解释成不存在约束。
7. **事实漂移**：工作树在摘要后被人或另一进程修改时，已缓存代码状态需重新核对；旧工具结果是过去事实。
8. **评测边界**：新 Agent 对摘要的复述只能检查显式误解，不能证明没有遗漏；需要带原始历史的独立标注与后续行为验证。

## 来源与适用范围说明

本次阅读以 ACP v1 稳定协议为基线；官方索引同时列有 ACP v2 草案及尚未稳定的 session compaction、end-turn token usage 等 RFD，不把提案当所有 Agent 已支持的能力。MCP 采用明确的 2025-11-25 版本链接，避免将 draft 的传输变化混入现有实现。供应商文档为访问当日版本，实施时应再次核实目标 Agent/适配器版本与实际握手结果。 [ACP 官方索引](https://agentclientprotocol.com/llms.txt)
