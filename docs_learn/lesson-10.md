# 第 10 课：⭐ 模型选择与切换（Model Selector）（总结）

> 对应对话内容：模型列表来源（config options）、findModelOption、model-catalog、setSessionConfig（含抢占标题轮询深挖）、recordModelChange 分隔线、workflow 复用模型发现。
> 代码地图：`packages/chat/src/model-option.ts`、`packages/chat/src/store.ts`（setSessionConfig/recordModelChange/withConfigOptions）、`packages/app-shell/src/features/chat/model-selector.tsx`、`model-catalog.ts`、`state/hooks/use-workflow-agent-models.ts`、后端 `crates/backend/src/agent_runtime/mod.rs` 的 `set_session_config`。
> 一句话：**模型列表只在会话建立时报告——必须先有 session 才能选模型；换模型是同一会话内改配置，不涉及 transcript。**

## 〇、核心认知

- **换模型（setConfig）= 同一台 CLI、同一个会话里改配置**——会话不变、上下文不变、**没有 transcript 问题**；
- **换 agent（switch）= 换成另一台 CLI = 新会话**——才有 transcript 注入（第 9、11 课）；
- **改模型不走 prompt**：prompt 是对话内容，改模型只能走 setConfig 操作（agent 自主用工具改模型是另一回事）。

## 一、模型列表从哪来？（config options 机制）

- ACP 规定：agent 在 **session/new 和 session/load 的回复**里报告配置选项（`SessionConfigOption[]`）；
- 其中 **`category: "model"` 的 select 类型选项 = 模型选择器**；
- **为什么必须先有 session 才能选模型**：模型列表只出现在 session/new、session/load 的回复里 → 没有会话就没有列表 → **打开聊天面先 warm**（这就是第 8 课"选择器预热"的根因，不是随便预热，是协议规定）。

### findModelOption 的两级规则（model-option.ts）

```typescript
selects.find(o => o.category === "model")   // ① 优先 category=model
    ?? (selects.length === 1 ? selects[0]! : null);  // ② 兜底：唯一 select
```

- ① 优先 `category === "model"`；② **兜底：只有唯一一个 select 就当它是模型选择器**——category 是 UX 提示，协议要求客户端必须容忍缺失；③ 都没有 → null（无可选模型）。

## 二、模型列表怎么显示？

### model-catalog.ts（CLI 目录）

- `AGENT_CLI_LABELS`：CLI 的人类可读名（**硬编码**，稳定产品名）；
- `AGENT_CLI_ORDER`：CLI 展示顺序（独立于哪个活跃）；
- **能硬编码的是"CLI 名单"，不能硬编码的是"模型列表"**——"哪些 CLI 存在 = 编译期已知；每个 CLI 提供哪些模型 = 运行时才知道"（来自 agent 自己的会话配置）。

### 缓存

模型列表几乎不变 → `useAgentModelStore.known[agentCli]` 缓存——"等待 session/new 再说一次，正是打开聊天感觉慢的原因"。

## 三、选完模型会话怎么记住？（setSessionConfig）

### 前端 → 后端

```
用户选模型 → setSessionConfig(oraSessionId, configId, value)
  → client.setConfig({ sessionId, configId, value })
```

### 后端 set_session_config（mod.rs）——先分叉

```rust
// ① 先试 warm：会话还在池子里 → 直接在池子里改
if let Some(result) = self.inner.warm.set_config(...).await { return result...; }

// ② 持久会话（有 actor）：
//    发 PreemptTitlePolling（抢占标题轮询）并等 ack
//    → 直接调 provider 的 session_set_config_option（不走 actor 串行流）
```

### 为什么"直接调 provider、不经过 actor 串行流"？

- actor 是串行管家（一次一个操作）；如果 setConfig 排队，可能被一个很长的 prompt 卡住几分钟；
- setConfig 独立于对话流 → 直连，快；
- 代码注释："The provider request remains **direct** because it is **independent of the actor's serialized prompt/load stream**; only the title-polling attempt needs preemption."

### 为什么抢占标题轮询？（深挖）

**标题轮询是什么**：会话标题 = agent 起的会话名（只有 agent 知道，ACP 的 session/list 返回）；push = agent 主动报，poll = Ora 主动问（session/list）。轮询只在**标题获取窗口**里（新 attach 的会话，First + Final 两个尝试，60s 手柄，之后 Locked 关闭）。

**机制**：轮询跑在 actor 里（占用 actor、`channel.take()` 拿走通道、发 session/list 请求等回复——**5 秒是超时不是间隔**，整个窗口最多发 2 次）。

**为什么抢占**：设计约定——**同一会话上运行时发起的请求一次只发一个**（用户操作优先于后台杂活 + 兼容不支持并发的 agent CLI）。setConfig 先发 PreemptTitlePolling 并**等 ack**（轮询那边还 channel + 记账 preempted + 回确认），确认停了才发自己的请求。

**诚实标注（深挖结论）**：技术上 ACP 客户端其实**支持并发**（pending 表是 HashMap、写入器有 AsyncMutex、响应按 ID 配对）——**没有硬冲突**；抢占是**设计约定**（串行化 + 可预测），不是修补会崩的 bug。冲突场景罕见（要新 attach + 轮询在途的 5 秒内 + 恰好改模型），防御性处理。

### agent 的回复是**权威**

请求值可能被调整（请求 X 给最近的 Y）或拒绝（保持原样）；之后 agent 发 `config_option_update` 回来 → 前端更新显示的是"agent 最终确认的值"。

## 四、切模型在时间线里怎么体现？（recordModelChange 分隔线）

- **modelChanges = 纯前端 UI 标记**（对话视图里画"模型切换分隔线"，第 N 回合后 + 新模型名）；
- **触发**：每次收到新 config options（setConfig 响应 / config_option_update / 握手）→ `withConfigOptions` → `recordModelChange` 比较前后模型；
- **三种静默情况**：① 无基线（previous/next 为 null——第一次报告的 options 是建立基线不是变化）② 模型没变 ③ 首轮前（afterTurnCount === 0，空对话没有东西要分）；
- **同位置折叠**：新切换位置和最后一条相同 → **替换**而不是追加（反复切换菜单不堆分隔线）；
- **afterTurnCount 语义**："当前回合报告的 options 描述的是在这回合**之前**生效的变化"——分隔线放在变化生效回合的**之前**；
- **三个"不是"**：不是历史文件记录（config 是会话 chrome，第 9 课规则④不存历史）、不是 handoff transcript（换 agent 才要）、不是后端逻辑（画线是前端自己的事）。

## 五、workflow 复用同一模型目录

- **为什么**：workflow 节点执行时委托给 agent 会话（第 16 课 NodeExecutor）→ "给节点选模型 = 给会话选模型" = 同一个问题 → 直接复用发现机制；
- **机制**：对每个 CLI warm → config options → findModelOption → 展平 `{ agentCli, modelId, label: "OpenCode · claude-sonnet-4" }`——发现机制零重复；
- **与 chat 的区别**：chat 只 warm 当前目标（按需）；workflow **全量 warm 所有 CLI**（`AGENT_CLI_ORDER.map`，要列出全部选项）；
- **单列表派生**：都从 `AGENT_CLI_ORDER` 派生——加一个新 CLI 只改 model-catalog.ts 一处，chat 和 workflow 自动同步（不维护第二个列表）；
- **target 回退**：task → projectRoot → 第一个项目（fallback，让 Settings 无选区也能发现）→ null；
- **每 CLI 独立状态**：cliStatus（loading/error per CLI），inspector 每行单独 spinner/重试；
- **范围澄清**：第五步只讲"模型面板列表哪来"；run=worktree、节点=session 的架构是第 15-17 课。

## 六、整条链路

```
选择器打开 → useWarmSession(target, agentCli) → warm → session/new
  → config options 回来 → findModelOption → 显示模型列表

用户选模型 → setSessionConfig → client.setConfig
  → 后端：warm 走池子 / 持久走"抢占标题轮询 + 直接调 provider"
  → agent 回 config_option_update（权威）
  → 前端 withConfigOptions → recordModelChange（有变化才记分隔线）
```

## 七、检查题答案（6 题）

**Q1. 模型列表从哪来？为什么"必须先有 session 才能选模型"？**
模型列表来自 ACP 的 config options——只在 session/new、session/load 的回复里报告。没有会话就没有 session/new → 没有列表。所以打开聊天面先 warm（这就是"选择器预热"的根因）。

**Q2. findModelOption 的两级规则？为什么需要"唯一 select 兜底"？**
① 优先 category === "model" 的 select；② 只有唯一一个 select 时当它是模型选择器。因为 category 是 UX 提示（协议要求客户端必须容忍缺失）——agent 不标 category 时，唯一 select 就只能是模型选项。

**Q3. model-catalog 里什么能硬编码、什么不能？为什么？**
CLI 名单（哪些 CLI 存在）能硬编码——编译期已知、稳定产品名；模型列表不能硬编码——运行时才知道（每个 CLI 提供哪些模型来自 agent 自己的会话配置）。

**Q4. setSessionConfig 怎么区分 warm/持久？为什么直接调 provider？为什么抢占标题轮询？**
warm 会话先试池子（set_config）；持久会话找 actor → 发 PreemptTitlePolling 等 ack → 直接调 provider。直接调：setConfig 独立于 actor 串行 prompt/load 流，排队会被长 prompt 卡住（快是目的）。抢占标题轮询：轮询占着 actor + 通道 + 有在途请求；设计约定"同一会话运行时请求一次一个"（用户操作优先于后台杂活 + 兼容不支持并发的 agent CLI）——技术上无硬冲突，是防御性设计约定。

**Q5. recordModelChange 哪三种情况静默？"同位置折叠"？和 transcript 的关系？**
三种静默：① 无基线（第一次 options 建立基线）② 模型没变 ③ 首轮前（没有东西要分）。同位置折叠：新切换位置与最后一条相同 → 替换而非追加（反复切换不堆分隔线）。和 transcript 没关系——modelChanges 是纯前端 UI 标记（不落盘、不是 handoff transcript、不是后端逻辑）；config 是会话 chrome，第 9 课规则④明确不存历史文件。

**Q6. workflow 模型发现和 chat 有什么不同？"单列表派生"指什么？**
chat 只 warm 当前目标（按需）；workflow 全量 warm 所有 CLI（要列出全部选项）。单列表派生：chat 和 workflow 都从 AGENT_CLI_ORDER 这一个列表派生——加新 CLI 改一处，两处自动同步，不用维护第二个列表。

## 八、术语表新增

Config Options（配置选项）、Model Selector（模型选择器）、category（类别提示）、setConfig / session_set_config_option（配置变更操作）、config_option_update（配置更新）、PreemptTitlePolling（抢占标题轮询）、Title Poll（标题轮询，已深化）、modelChanges（模型切换分隔线）、afterTurnCount（回合计数）、Workflow Agent Models（workflow 模型目录）。详见桌面 software technical terms.md。

## 九、下一课预告

> 第 11 课（⭐ 切换 Agent，关键专题 3）：怎么把一段对话换到另一个 CLI？warm pool 认领、pick vs commit、懒注入 transcript、为什么不保留旧绑定——三课收官。
