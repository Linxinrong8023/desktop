# Agent 压缩输入超窗：社区实现核查

核查日期：2026-09-11（Asia/Shanghai）。范围限于公开主仓源码、官方文档与原论文；不以 issue 中的建议代表已经实现的行为。

本文回答的是：**尚未总结的历史本身已经超过摘要模型输入窗口时，社区怎样处理，是否完整覆盖历史，是否需要多次 LLM 调用。**
“正常对话提前触发压缩”“主模型请求超窗后再压缩”“摘要请求自己超窗”是三个不同问题。
本文只新增研究记录，不修改 Ora 现有交接设计。

## 结论

本次核查没有发现同时满足“任意长输入、固定摘要窗口、所有历史均被处理、只调用一次生成模型”的通用实现。
OpenCode、Pi 的内建路径主要依靠提前触发和工具结果截断；摘要输入仍超窗时会失败。
Aider、LangMem、LangChain 的部分路径通过裁掉尚未摘要的内容维持输入预算，不能满足“不跳过未总结轮次”的要求。
原文外置加检索、LLMLingua-2 的 token 筛选能够减少生成式摘要工作，但会引入检索覆盖或筛选损失，不能直接等同于完整总结。
下文分别给出源码证据；“对 Ora 的推断”集中放在最后，避免把工程判断写成项目承诺。

## 快速对照

| 实现 | 摘要输入过大时的关键处理 | 生成式调用 | 是否满足全部待摘要历史均被处理 |
| --- | --- | --- | --- |
| OpenCode | 工具结果限长、媒体占位；摘要仍超窗则停止 | 常规摘要 1 次，另有请求重试与恢复后的工作调用 | 工具正文已裁；overflow 恢复还会排除最近一轮的中间过程 |
| Pi | 每个工具结果限 2000 字符；摘要错误向上传递 | 普通 1 次；切开长 turn 时可为 2 次，再直接拼接 | 不按模型窗口递归覆盖积压；工具正文已裁 |
| Aider | 过大的 head 只取最早能装下的部分 | 可递归、多模型尝试 | 否：未装入部分不在摘要，也不在保留 tail |
| LangMem / LangChain | 对待摘要段再做保留最近部分的裁剪 | 通常 1 次，另有重试 | 否：被裁的内容仍可能随整个区间退出上下文 |
| context-mode | 原文留在外部索引，只返回检索片段 | 索引/检索不要求生成式摘要；后续读取另计 | 原文可保留，但不保证接收模型读过全部原文 |
| LLMLingua-2 | 小型 encoder 为 token 打分并删除部分 token | 不要求远端生成摘要；本地分块批量推理 | 原文分块进入分类模型，但输出会删除部分 token，不保证语义完整 |

表中调用次数只描述算法正常路径，不含网络重试；“原文仍在存储”与“交接模型完整获得其语义”也不同。

## 1. OpenCode：先缩小表示，摘要仍超窗就停止

固定版本：[anomalyco/opencode@193de13a88d62a6409c6d385831180f1def527dc](https://github.com/anomalyco/opencode/tree/193de13a88d62a6409c6d385831180f1def527dc)。

- **提前触发。** `isOverflow()` 将累计 token 与 `usable()` 比较；后者按模型输入或上下文上限扣除预留输出预算。这是运行请求的预算控制，不是对序列化后的摘要请求做任意长度保障。[overflow.ts](https://github.com/anomalyco/opencode/blob/193de13a88d62a6409c6d385831180f1def527dc/packages/opencode/src/session/overflow.ts#L8-L34)
- **摘要前已有损。** `serialize()` 保留用户/助手文本和调用参数，但每个完成工具结果最多取 2000 字符；已经 prune 的结果变为清理标记，附件只剩类型和文件名占位。[序列化源码](https://github.com/anomalyco/opencode/blob/193de13a88d62a6409c6d385831180f1def527dc/packages/opencode/src/session/compaction.ts#L28-L85)
- **近期尾段另留。** `select()` 依据近期 token 预算选择保留 tail，必要时在一个 turn 内切分；其余 head 进入摘要。这是在选择近期保留范围，并未把过大 head 切成多个模型窗口。[选择源码](https://github.com/anomalyco/opencode/blob/193de13a88d62a6409c6d385831180f1def527dc/packages/opencode/src/session/compaction.ts#L223-L269)
- **旧工具输出清理是独立机制。** 开启 prune 时，它跳过最近部分、保护 `skill` 工具，累计超过保护预算后标记更旧输出；只有可清理量超过阈值才写入标记。这个操作不调用摘要模型。[prune 源码](https://github.com/anomalyco/opencode/blob/193de13a88d62a6409c6d385831180f1def527dc/packages/opencode/src/session/compaction.ts#L271-L317)
- **真正的摘要超窗。** 选出的 head 序列化后进入一次 `processor.process()`。返回 `compact` 时写入 `ContextOverflowError` 并返回 `stop`，没有在这里缩小分块后继续摘要的循环。[请求与失败分支](https://github.com/anomalyco/opencode/blob/193de13a88d62a6409c6d385831180f1def527dc/packages/opencode/src/session/compaction.ts#L377-L458)

主请求已经 overflow 时还有一个区别：如果存在更早用户历史，代码将摘要历史截到最近一个普通 user 之前，成功后重放该 user 的 parts；媒体附件替换为占位文本。
**由控制流可见，该 user 之后的 assistant/tool 中间过程没有随 user 一起重放，也未进入这次摘要。** 因此不能将此恢复描述为完整保留所有未摘要轮次。
证据：[截取历史](https://github.com/anomalyco/opencode/blob/193de13a88d62a6409c6d385831180f1def527dc/packages/opencode/src/session/compaction.ts#L340-L359)、[重放用户消息](https://github.com/anomalyco/opencode/blob/193de13a88d62a6409c6d385831180f1def527dc/packages/opencode/src/session/compaction.ts#L468-L495)。

该判断限于内建路径；项目提供插件钩子，插件可以替换摘要行为，不能据此推定所有插件都具有相同限制。

## 2. Pi：普通一次摘要，长 turn 可分成两次，但不是超窗递归

固定版本：[earendil-works/pi@62129190d81067ec86ae5a5fc907c96bfe435a78](https://github.com/earendil-works/pi/tree/62129190d81067ec86ae5a5fc907c96bfe435a78)。

- **提前触发。** 默认预留 16384 token，保留最近约 20000 token；判断为 `contextTokens > contextWindow - reserveTokens`。官方文档同时说明工具批次完成后、下一响应前以及新用户请求前后的检查位置。[默认值和判断](https://github.com/earendil-works/pi/blob/62129190d81067ec86ae5a5fc907c96bfe435a78/packages/coding-agent/src/core/compaction/compaction.ts#L132-L237)、[触发时机](https://github.com/earendil-works/pi/blob/62129190d81067ec86ae5a5fc907c96bfe435a78/packages/coding-agent/docs/compaction.md#L25-L46)
- **摘要前仍截工具结果。** `serializeConversation()` 将每个工具结果保留前 2000 字符并加截断标记；用户/助手文字、thinking、工具参数并不因此获得全局输入上限。[utils.ts](https://github.com/earendil-works/pi/blob/62129190d81067ec86ae5a5fc907c96bfe435a78/packages/coding-agent/src/core/compaction/utils.ts#L88-L150)
- **正常是一次。** `generateSummaryWithUsage()` 把当前待摘要消息与旧摘要序列化，交给一次 `completeSummarization()`；设置 `maxTokens` 主要约束摘要输出，并没有将输入按窗口循环切片。[构造及调用](https://github.com/earendil-works/pi/blob/62129190d81067ec86ae5a5fc907c96bfe435a78/packages/coding-agent/src/core/compaction/compaction.ts#L654-L726)
- **长 turn 可以两次。** 若保留边界切在一个 turn 内，且之前还有历史，先总结历史，再总结该 turn 的 prefix；两个结果直接字符串拼接，不再增加第三次合并模型。若没有更早历史，只调用 prefix 摘要。这是语义边界拆分，不是依据摘要模型窗口反复分块。[compact()](https://github.com/earendil-works/pi/blob/62129190d81067ec86ae5a5fc907c96bfe435a78/packages/coding-agent/src/core/compaction/compaction.ts#L858-L947)
- **失败不推进摘要。** 摘要响应报错或输出因长度截断会抛错；自动压缩捕获异常后报告失败并设置 `willRetry: false`。没有在该失败分支继续切小输入的代码。[摘要校验](https://github.com/earendil-works/pi/blob/62129190d81067ec86ae5a5fc907c96bfe435a78/packages/coding-agent/src/core/compaction/compaction.ts#L546-L555)、[自动压缩失败](https://github.com/earendil-works/pi/blob/62129190d81067ec86ae5a5fc907c96bfe435a78/packages/coding-agent/src/core/agent-session.ts#L2426-L2448)

主工作请求 overflow 的恢复则是另一层逻辑：从运行上下文移除失败/截断的最后 assistant，执行压缩，再尝试继续一次；重复失败时停止恢复。
源码注释明确失败消息仍留在 session history；这与删除数据库记录不同。[恢复边界](https://github.com/earendil-works/pi/blob/62129190d81067ec86ae5a5fc907c96bfe435a78/packages/coding-agent/src/core/agent-session.ts#L2180-L2223)

瞬时网络失败还可以触发 `retryAssistantCall`，因此“普通一次/长 turn 两次”不是实际 HTTP 请求数上限。[重试包装](https://github.com/earendil-works/pi/blob/62129190d81067ec86ae5a5fc907c96bfe435a78/packages/coding-agent/src/core/compaction/compaction.ts#L572-L599)

## 3. Aider：窗口内只总结部分 head，其余未覆盖内容可能丢失

核查固定文件版本：[history.py@19a7864](https://github.com/Aider-AI/aider/blob/19a786416846d3ce446973f6e9dabb859642a9d2/aider/history.py#L31-L113)。

`summarize()` 先保留最近一段 tail；待摘要 head 超过摘要模型预算时，只从最早消息向后收集能放下的 `keep`，然后产生 `summary(keep) + tail`。
**由返回值可见，head 中未进入 keep 的消息没有被分块补摘要，也没有包含在 tail 中。** 递归针对已经得到的 `summary + tail`，不能补回此前遗漏的 head。
少量消息、递归过深或切点过早时还可能直接进入 `summarize_all()`；这些路径也不构成任意长输入保护。[切分、裁剪及递归](https://github.com/Aider-AI/aider/blob/19a786416846d3ce446973f6e9dabb859642a9d2/aider/history.py#L31-L113)

Aider 的弱模型优先、后台摘要线程属于成本和等待优化：阈值到达后可在后台开始摘要，真正使用前等待线程结束。
它们不会减少需要覆盖的信息量；递归、模型回退和重试仍可产生多次请求。[后台启动和等待](https://github.com/Aider-AI/aider/blob/4e77720c6f96d4960b61ef19f32a2ee12218bf96/aider/coders/base_coder.py#L1002-L1038)、[官方阈值说明](https://aider.chat/docs/config/options.html#--max-chat-history-tokens-value)

## 4. LangMem / LangChain：单次调用依赖摘要前裁剪

LangMem 的 `_adjust_messages_before_summarization()` 使用 `trim_messages(strategy="last", allow_partial=True)`，只保留待摘要段的最近部分，也允许消息部分裁切。
裁空时它警告并退回原始输入，因此该分支仍可能超窗。[输入调整](https://github.com/langchain-ai/langmem/blob/a463b1f86520230dcaff7822f8a73a3bacf350dd/src/langmem/short_term/summarization.py#L203-L233)

真正送给模型的是调整后的列表，但更新 `summarized_message_ids` 和决定最终保留切片用的是原始待摘要区间。
**由此推导：被裁掉而未送入模型的内容，仍可能被记为已经 summarized 并离开上下文。** 这不是输入覆盖保证。[模型调用与 ID 更新](https://github.com/langchain-ai/langmem/blob/a463b1f86520230dcaff7822f8a73a3bacf350dd/src/langmem/short_term/summarization.py#L431-L449)、[返回上下文](https://github.com/langchain-ai/langmem/blob/a463b1f86520230dcaff7822f8a73a3bacf350dd/src/langmem/short_term/summarization.py#L263-L297)

当前核查的 LangChain `SummarizationMiddleware` 同样默认将待摘要内容裁到 4000 token，通常调用一次带重试的模型；裁剪异常退为最后 15 条，裁空则返回固定提示文本。
`trim_tokens_to_summarize=None` 可禁用裁剪，但其效果是全量发送，未增加后续的超窗分块恢复。
证据：[默认值](https://github.com/langchain-ai/langchain/blob/bc16168d710d8b594d0bc622c70361876bd9cbda/libs/langchain_v1/langchain/agents/middleware/summarization.py#L98-L100)、[摘要与裁剪实现](https://github.com/langchain-ai/langchain/blob/bc16168d710d8b594d0bc622c70361876bd9cbda/libs/langchain_v1/langchain/agents/middleware/summarization.py#L832-L910)、[替换整个区间](https://github.com/langchain-ai/langchain/blob/bc16168d710d8b594d0bc622c70361876bd9cbda/libs/langchain_v1/langchain/agents/middleware/summarization.py#L425-L435)。

## 5. context-mode：外置原文，检索代替全量进入窗口

官方 README 描述 `ctx_index` 将内容放入 SQLite FTS5 索引，`ctx_search` 用全文检索/BM25 返回匹配片段。
索引和搜索本身不需要生成式摘要模型，特别适合在工具输出进入主上下文之前控制其体积。[官方仓库与工具说明](https://github.com/mksglu/context-mode)

这里保存的是可检索原文，进入模型的是查询选中的子集。
**对覆盖的判断：** 原文未被删除，并不保证某次查询会取回所有重要约束；如果需要进一步读原文，仍会消耗 agent 轮次、输入 token 和等待时间。
因此这是改变“交接资料如何被读取”的方案，不能直接作为“一份固定长度摘要完整代表超大历史”的实现证据。

## 6. LLMLingua-2：不用生成摘要，改做 token 分类筛选

微软官方说明将任务建模为 token 分类：由小型双向 encoder 判断应保留哪些 token，删除低重要度部分；它不是通过一次生成式调用理解任意长历史再写摘要。[官方 README](https://github.com/microsoft/LLMLingua)、[ACL 原论文](https://aclanthology.org/2024.findings-acl.57/)

源码的 `init_llmlingua2` 设置局部序列长度，`__chunk_context` 分块后通过 DataLoader 批量推理。
这能把远端生成调用换成本地模型计算，但仍有模型部署、多个批次推理和筛选损失。[prompt_compressor.py](https://github.com/microsoft/LLMLingua/blob/main/llmlingua/prompt_compressor.py)

**对 Ora 的判断：** 若保留用户要求、代码标识符、数字和否定条件至关重要，不能从论文总体压缩指标直接推定本项目中文/代码交接安全。
它适合作为需专项验证的预处理候选。它与直接跳过旧轮次不同：文本可以全部进入分类过程，但“都经过分类”仍不保证关键约束完整保留。

## 补充：lossless-claw 的摘要树把调用成本显式化

官方架构的 leaf pass 选择最旧的连续原始消息块，受 `leafChunkTokens` 限制，结合 `previous_context` 调用 `runtime.llm.complete`。
`compactUntilUnder()` 还可运行多轮 leaf / condensed pass，直到满足预算或无法继续。这是分块摘要与层级合并，并不是避免多次摘要的办法。
证据：[架构文档](https://github.com/Martian-Engineering/lossless-claw/blob/main/docs/architecture.md)、[压缩实现](https://github.com/Martian-Engineering/lossless-claw/blob/main/src/compaction.ts)。

README 直接讨论增大 `leafChunkTokens` 以减少频繁摘要、用 `incrementalMaxDepth` 限制深度。
它的价值在于原文可追溯和可控的增量处理；“lossless”不意味着一份摘要在语义上包含所有原文细节。[项目说明](https://github.com/Martian-Engineering/lossless-claw)

以上三个补充方向引用核查时的 `main` 文档/源码，链接会随上游变化；OpenCode、Pi、Aider、LangMem、LangChain 的关键判断则固定到具体 commit。

## 对 Ora 的推断：可借鉴什么，哪些条件不能混淆

以下为基于上述源码的工程判断，尚未实现，也未在 Ora 数据上验证。

1. **必须分别计算正常工作请求与摘要请求的预算。** 更早触发能减少超窗概率，但不能覆盖历史导入、切换到较小摘要模型、单次巨大输入等情况。工具结果限长也不能约束任意长的用户文本或调用参数。
2. **不能把裁掉的消息写入“已摘要”水位。** 如果要求所有新增历史 H 都进入摘要过程，应以实际送入模型的消息区间推进水位；Aider/LangMem 的上述裁剪行为不符合这个要求。
3. **保留原始历史是恢复条件，不等于摘要保真。** 即使全部 H 都曾进入摘要模型，也只能保证输入覆盖，不能保证所有事实、限制条件和跨块关联都被摘要正确保留。
4. **若 H 已经超窗而又禁止跳过内容，分块是可核算的兜底。** 可对 H 的连续块分别生成增量摘要，块间不反复附带旧摘要 S；随后只在需要时合并新增摘要，最后进行一次 S 与新增摘要的整合。这样减少 S 的重复处理，但并未消除多次模型调用及摘要损失。
5. **若不愿反复压缩 S，应改变交接产物。** 保存不可变旧摘要、增量记录与原文索引，让接收方按需读取；这是检索式交接，需要接收端支持，也不保证一次窗口读完所有历史。

因此实际可选项是明确取舍：提前增量处理来摊开等待，降低摘要模型单价，原文外置后按需读取，或者承担覆盖全部新增历史所需的分块调用。
本次源码核查不能支持“社区已经有一个既不丢未总结轮次、又对任意长积压只调用一次模型”的结论。
