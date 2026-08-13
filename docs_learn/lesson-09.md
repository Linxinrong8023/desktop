# 第 9 课：⭐ 保存上下文信息——ora-history 会话历史与 transcript（总结）

> 对应对话内容：为什么自己记、文件格式、顺序规则、assembler 合并规则、degraded 失败语义、handoff transcript、binding_needs_handoff。
> 代码地图：`crates/history/`（assembler / writer / record / handoff / path / reader）、`crates/backend/src/session_history.rs`、前端 `session-history-banner.tsx`、`docs/agent-runtime.md` "Session History" / "Degraded History" 节。
> 一句话：**模型上下文是 agent 的，对话记录是 Ora 的。**

## 〇、角色与文件总览

| 模块 | 职责 |
|---|---|
| `record.rs` | HistoryLine / HistoryRecord（六种记录）+ AgentCli 文本序列化 |
| `assembler.rs` | 把实时 update 流组装成 settled 记录（纯内存无 IO） |
| `writer.rs` | append-only JSONL 追加（每次 append 开文件写 flush） |
| `reader.rs` | 读回、按 seq 排序、重复 seq 去重 |
| `path.rs` | 从 session id 推导文件路径（分片） |
| `handoff.rs` | 渲染换 agent 的 transcript + binding_needs_handoff |
| `session_history.rs`（backend） | 删除会话时清理历史文件（级联前收集 id） |

## 一、为什么 Ora 自己记？（Ora owns the transcript; the agent owns the model context）

| 原因 | 说明 |
|---|---|
| 不靠 agent 复述 | agent 可能不能、不愿、不记得 |
| 活得比 provider 久 | 换 agent、provider 挂掉，对话还在 |
| 重放不用问 agent | load 时前端直接读 Ora 的记录重建 |

## 二、文件格式（每会话一个 append-only JSONL）

### 路径：Git 式分片（推导，不存储）

```
<root>/<id[0..2]>/<id[2..4]>/<id>.jsonl
```

- 例：`550e8400-...` → `root/55/0e/550e8400-....jsonl`；两级分片 = 65,536 目录；
- **推导而不存储**：后端知道 session id 就能算出文件在哪；存储 = 第二个答案，路径变化会跟实际打架；
- **id = session id（Ora session id）**：每个会话的历史文件以它的 session id 命名。

### HistoryLine（一行）

```rust
pub struct HistoryLine {
    pub at: String,   // 写入时间（本地 RFC 3339，落盘时间，审计用）
    pub seq: u32,     // 时间线位置（对话里的位置，不是写入顺序）
    pub record: HistoryRecord,  // 内容（serde flatten 平铺）
}
```

### 六种 HistoryRecord

| 记录 | 什么时候 | 为什么 |
|---|---|---|
| Meta | 文件开头 | 钉住 schema 版本 + 初始绑定（session/task/CLI/provider id/cwd） |
| Update | 一条 settled 的消息/想法/工具调用/计划 | 对话内容本身 |
| TurnEnded | 回合结束 | 带 stopReason——provider 从不带，没它"取消"和"完成"分不清 |
| AgentSwitched | 换 agent | from → to + 新 provider session id |
| Gap | 写失败后恢复时 | 标记丢失的洞（洞不能无声） |

Agent 身份存"名字空间文本"（`database_value`）不存 Rust 枚举名——归档不依赖枚举拼写。

## 三、顺序规则（seq 定义时间线，不是文件顺序）

- 记录在 **settled 时**追加，不是出现时——早开的工具调用可能晚写完 → **文件顺序 ≠ 对话顺序**；
- **读者按 seq 排序修复**；
- **重复的 seq = 修正**：同一位置写两次，读者保留最后一个（倒着扫，每 seq 只留第一个遇到的）。

## 四、assembler 合并规则（把流式块拼成完整消息）

### ① 文本合并（messageId 是身份）

- 同一 messageId 的 chunk → 拼成一条；
- **messageId 变了 = 新消息** → 上一条 settle；
- **无 messageId**：连续性当身份——被任何"占自己位置"的条目（工具/计划/图片）打断就 settle；不打断会把前后话拼成一条巨型消息，锚在工具调用**之前**（摘要排到它描述的工具前面——顺序错乱）。

### ② 工具调用只靠"证据" settle，不靠假设

- ACP 不要求报终态 → 停在 pending/in_progress 与"还在跑"无法区分；
- **证据**：agent 亲口报终态（不重新解释）；**回合正常结束（EndTurn）时还开着的 = 完成**（agent 自己选的停，当然拿到了结果）；
- 其他结束（取消/拒绝/超长）→ **保持未完成**（不改写），TurnEnded 记录原因；
- 有意的、有损耗的推断：文件里的 completed 可能来自 agent 或 Ora，事后无法区分。

### ③ 用户消息从"请求"记录，不记 agent 的 echo

- echo 带的是"实际发送的"（含注入的 handoff transcript 等上下文）；
- 记录的 blocks 才是"Ora 选择保留的真相"（用户真正说的）；用户消息在发请求时已确定 → 立即记录。

### ④ 不记会话 chrome

- AvailableCommands / ConfigOption / SessionInfo / Usage 等 → 每次绑定重新报告 → 存了 = 过期拷贝；要用从当前绑定实时拿。

## 五、失败语义（degraded）

**核心：跳着记比停下来更危险——写失败 = 永远停止记录，绝不悄悄跳过**（缺口不可见 = 最危险；停止 = 缺口可见）。

| 场景 | 处理 | 为什么 |
|---|---|---|
| 写失败在流式期间 | 回合**继续跑完**，然后停止记录 | agent 的工作是真的，掐死 = 骗用户"什么都没发生" |
| 写失败在记 prompt 时 | 调 agent **之前拒绝** | 还没发生，拒绝没损失；发出 = 对话移动到记录跟不上的地方 |
| 会话状态 | `historyState: degraded`（带 OS 原因） | 后续 prompt 拒绝（session_history_degraded） |
| resumeSessionHistory | ① 先写 **Gap** 记录 ② 再恢复可写 | 洞必须显式；失去的补不回，只标记"失去过" |
| 文件读不了 | 同样 degraded | 不知道 seq 用到哪 → 追加会覆盖 |
| load 读不了历史 | load 直接失败 | 空视图 = "从没说过话"，骗用户 |

**流式回合写失败的正确画面**：seq 1-10 正常 → seq 11 写失败 → **回合剩下的所有记录全停**（degraded = 停止记录）→ 前端照常显示完整回复 → resume 时写 Gap(11) → 新记录从 12 继续 → 文件 1-10, Gap(11), 12, 13...。

**信息差问题（用户提问的深挖）**：
- degraded 时 **prompt 和切换都被拒绝**（"用户继续提问"场景被堵死）；
- 前端有红色横幅（`session-history-banner.tsx`）：标题 + 原因原文 + [恢复] 按钮——**用户知道历史坏了**；
- resume 后 Gap 进 transcript → **agent B 知道自己不知道**；
- 剩下的"用户知道内容、agent B 不知道内容"是数据丢失的固有代价（无法消除），靠用户补述或 agent B 追问弥合；
- 对比"跳过继续记"：用户和 agent B 都以为完整实际缺 = 双重信息差（更危险）。

## 六、handoff transcript（换 agent 怎么带过去）

**ACP 装不了会话**：session/new 不带上下文、session/prompt 只有单个用户回合 → **每个记录的回合折叠成一条用户消息**，接收 agent 看到的是"被展示的 transcript"而不是"参与过的"。

### 结构

```
<ora_session_handoff>
preamble（这是完整历史，工作已完成，接着做别重复，用户新消息跟在后面）

## Turn N
**User:** 全文
**Assistant:** 全文
**Tools:** read_file (completed), edit (completed)
_注解（取消/拒绝/超长时才有）_
_笔记（换 agent / Gap 时才有）_

</ora_session_handoff>
[用户的新消息]
```

### 取舍

| 保留 | 丢弃 | 为什么 |
|---|---|---|
| 用户消息全文 | 推理（thought） | 属于产生它的 agent，不转移；不同 agent 想法不同 |
| 助手回复全文 | 工具输入/输出 | 过时/太大，挤掉对话本身 |
| 工具标题+结果 | 计划、session chrome | 同上 |
| —— | 图片等非文本 | 重编码没意义，`[image]` 占位 |

### 细节

- **注解只在回合被砍断时**（取消/拒绝/超长；EndTurn 正常结束零注解）——"只有接班人可能误读的结局才值得说明"；
- **笔记只在那个回合发生了换 agent 或 Gap**；
- **标记中和化**：transcript 文本里的 `</ora_session_handoff>` 被替换成不可见字符版本——防提前关闭包裹块、剩余变成指令（prompt injection 防护）；
- **没有大小预算**：超长 = 接收模型上下文窗口错误（provider 浮现）；
- **懒注入**：换绑那一刻什么都不发，下一条 prompt 作为 leading content block 插入——被换绑又放弃的会话成本为零。

## 七、binding_needs_handoff（要不要注入？从记录推导）

- 回答：**"换绑后的新 agent 有没有被说过话？"**；
- 推导逻辑：**倒着扫**——先碰到**用户消息** = 说过了（false）；先碰到 **AgentSwitched** = 还欠着（true）；**Gap/TurnEnded/Meta 跳过**（中性记录，不回答该问题）；
- **不存 flag 的三个原因**：① 重启幸存（内存 flag 重启就没了）；② 无第二真相源（flag 可能跟记录打架）；③ **记录 prompt 会撤销欠债**——注入失败时直接放弃 = "问题永远不会再被问起"（transcript 悄悄丢）→ 所以注入时读不了历史要**推迟到下一条 prompt**；
- 写 Gap 不影响判断（Gap 在跳过组）。

## 八、检查题答案（7 题）

**Q1. 为什么 Ora 不靠 agent 复述？**
agent 不一定能通过 ACP 发回对话历史，就算能也不一定愿意——通过 agent 没保障；自己记还能让对话活得比 provider 久（换 agent/provider 挂掉对话还在）。

**Q2. 路径为什么推导而不存储？**
session id 固定 → 推导就能找到文件；存储 = 第二个答案，路径变化会跟实际打架——只有一个权威，不会不一致。

**Q3. 文件顺序 ≠ 对话顺序靠什么修复？重复 seq 代表什么？**
靠 seq 排序修复；重复 seq = 修正（同一位置写两次，读者保留最后一个——倒着扫，每 seq 只留第一个遇到的）。注意：in_progress **从不写盘**（assembler 攥到终态证据才第一次写）；重复只发生在"写盘后又变了"。

**Q4. 工具调用停在 pending/in_progress 永远不报终态怎么处理？**
agent 亲口报终态 → 写（不重新解释）；回合正常结束（EndTurn）时还开着 → 记成 completed（证据：agent 自己选的停，当然拿到了结果）；其他结束 → **保持未完成**（不改写，不是记失败——failed 只能 agent 亲口报），TurnEnded 记录原因。

**Q5. 为什么"跳着记比停下来更危险"？degraded 怎么恢复？**
跳着记的缺口不可见——agent 误以为完整实际缺（消息差）；停下 = 缺口可见。恢复两步：① 先写一条 **Gap 记录**（命名中断原因，让洞显式化）② 再恢复可写（失去的内容补不回，只标记"失去过"）。

**Q6. handoff transcript 保留什么、丢弃什么？**
保留用户消息全文 + 助手回复全文；丢弃工具输入/输出（只留标题+结果）、推理 thought（不同 agent 想法不同）、计划、会话 chrome；图片等非文本 → `[image]` 占位。

**Q7. binding_needs_handoff 为什么不存 flag、从记录推导？推导逻辑？**
不存 flag：① 重启幸存（内存 flag 重启就没了）；② 无第二真相源（flag 可能跟记录打架）；③ 记录 prompt 会撤销欠债（注入失败时直接放弃 = 问题永远不会再被问起，所以要推迟）。推导逻辑：倒着扫——先碰到用户消息 = 说过了（false）；先碰到 AgentSwitched = 还欠着（true）；Gap/TurnEnded/Meta 跳过。

## 九、术语表新增

Transcript（对话记录）、append-only（只追加）、JSONL、settle（定稿）、seq（位置）、HistoryLine / HistoryRecord、Gap（缺口记录）、degraded（降级）、resumeSessionHistory（恢复历史）、handoff（移交）、binding_needs_handoff（是否欠注入）、chunk（分块）、messageId、echo（回显）。详见桌面 software technical terms.md。

## 十、下一课预告

> 第 10 课（⭐ 模型选择与切换 Model Selector）：前端怎么知道有哪些模型？为什么"必须先有 session 才能选模型"？config options 机制 + model selector + modelChanges 时间线记录。
