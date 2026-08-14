# 第 11 课：⭐ 切换 Agent（会话换绑）（总结）

> 对应对话内容：后端 switch_agent 全流程（前置检查/claim/先认领后放旧/响应带配置）、pick vs commit、懒注入 transcript、不保留旧绑定、pending 概念、Q2/Q6 深挖重讲。
> 代码地图：后端 `crates/backend/src/agent_runtime/mod.rs` 的 `switch_agent` + `warm.rs` 的 `claim`；前端 `packages/app-shell/src/state/stores/pending-agent-store.ts`、`packages/chat/src/store.ts` 的 `adoptSwitchedAgent`；`docs/agent-runtime.md` "Switching Agents" 节。
> 一句话：**会话保留 id/Task/历史，只有 binding 变；换绑从 warm pool 按同 key 认领；transcript 懒注入；不保留旧绑定。**

## 〇、三课关系（9/10/11）

```
第 9 课（基础）：Ora 自己记 transcript → 换 agent 时带过去的东西
第 10 课（前奏）：模型选择器 → 换绑前先选模型/预热目标 CLI
第 11 课（动作）：切换 Agent → 认领新会话 + 懒注入第 9 课的 transcript
```

## 一、切换是什么？（会话不变，只有绑定变）

回顾三标识（第 8 课）：

```
Ora session id：不变（对话还是那段对话，数据库记录）
provider session id：变！（opencode 的 → claude 的）
warmkey：用来找 claude 的预热会话（任务+新 CLI+聊天面）
```

**会话保留 id、Task、历史——只有 binding（绑定）变**。模型选择在换绑前已做（第 10 课）→ **换绑后保留**（用户选的模型跟着走）。

## 二、后端 switch_agent 流程（mod.rs）

### ① 前置检查（在 claim 之前！）

```rust
if target == session.agent_cli {
    return Err(SessionAgentUnchanged);  // 换到同一个 CLI
}
if let HistoryState::Degraded { .. } = session.history_state {
    return Err(history_degraded());     // 历史坏了（第 9 课）
}
```

**为什么 unchanged 必须在 claim 之前**（完整链条）：
- 没有检查 → 继续 claim → claim 会 warm/新建 provider 会话（池子没有就现场建）→ 认领到新会话 B → 换绑（拆 A 绑 B）→ **同一个 CLI、同一个会话、什么都没变，但白白建了一个 provider 会话 B + 拆了会话 A**；
- 检查在 claim 之前 = **连新建都不让发生**；
- 注释原话："Warming the CLI a session already runs on would build a second provider session only to replace the current binding with an indistinguishable one."

**为什么 degraded 拒绝**：历史坏了，transcript 无法可靠带过去（第 9 课）。

### ② claim（认领 claude 的预热会话）

```rust
let reservation = self.inner.warm.claim(
    WarmKey { target: Task{task_id}, agent_cli: target, client_id },
    &cwd,   // 任务 worktree 目录
).await?;
```

- **同一个 key**：选择器预热用这个 key、switch 认领用同一个（第 8 课 claim）；
- claim 内部：**解析 + 预订在一个临界区**（加锁，没人能插队抢走）；
- **池子里有 → 直接拿；没有（被淘汰/过期/进程重启）→ 现场新建**——switch 永远不会因为"预热没了"而失败（永远能换，只是可能多等握手）；
- 类比：订好的房间没了（超时释放）就现场开一间——永远有房住。

### ③ 认领成功，才动原绑定（关键顺序！）

```rust
// Only now is the move certain, so the old binding can be released.
let response = async {
    let _lifecycle = self.inner.lifecycle.lock().await;
    let channel = supervisor.open_session_channel(...)?;      // 新路由
    let (session, recorder) = self.rebind_to_provider(...).await?;  // 换绑定 + 记 AgentSwitched
    self.insert_actor(session.clone(), ActorSetup {
        ...
        handoff_pending: true,                    // 新 agent 啥都不知道
        title_acquisition: TitleAcquisition::locked(),  // 标题窗口锁死
    })?;
    ...
}.await?;
reservation.commit();   // 全部成功 → 预订生效
```

**为什么顺序必须是"先认领、成功后才动"**：
- ❌ 反序（先动旧、再认领）：旧绑定没了（actor 停/路由摘）→ 认领失败 → **两头空**（旧会话废了、新会话没有）= 用户卡死；
- ✅ 正序：认领成功（移动确定）→ 才动旧 → 认领失败 → **旧绑定纹丝不动**，用户还在原 CLI 上聊天，零损失；
- **认领失败的兜底**：claim 返回 WarmReservation 对象（第 8 课）——后面任何一步失败，对象 drop 自动退回池子，claude 会话也不浪费（留给重试）。
- 类比：**先租到新房子再退旧房子**——新房没租到还能住旧的；先退旧的、新房又黄了就睡大街。

### ④ 响应必须带 available_commands + config_options

> "ACP reports both only while a session is being created or loaded, so anything not returned here can never be asked for again."

- ACP 协议规定：命令和配置**只在 session/new、session/load 的报告里出现**——**换绑之后没有任何 API 能再问一次**；
- 不带上 → 前端永远拿不到 claude 的命令/模型 → **模型选择器继续显示旧 CLI（opencode）的模型**（错得离谱）；
- 为什么 claim 能带上：预热时（session/new）config_options 记录在 WarmAttachment 里，claim 从 entry 上读出来带进响应；
- 类比：入职体检报告只在入职时发一次——换部门必须带过去，不然新部门永远不知道你的身体状况。

## 三、pick vs commit（前端）

### pending-agent-store：两个形状的"记下来"

```typescript
selections: Record<targetKey, AgentCli>;  // ① 未开始聊天表面的默认 agent
switches:   Record<sessionId, AgentCli>;  // ② 持久会话要换绑的 agent
```

**pending = 悬而未决的（还没兑现的）选择**：point 选 claude（还没发消息）→ switches 里记一条；发消息 → commit → 清掉。**pending = 意图；session = 事实**。

### 点选 ≠ 立即切换

> "The rebind itself waits for the next message, because performing it at click time would tear down the agent that is mid-reply."
>
> （换绑本身等**下一条消息**——点击时执行会**拆掉正在回复的 agent**。）

```
点选 claude → 记 switch + warm claude → 菜单不关让你选模型
发下一条消息 → commit（真正调用 switch_agent）
```

### 又选回原 CLI = 撤回记录

> "picks accumulate freely while nothing is committed, so a client that arrives back at the CLI its session is still bound to withdraws the record instead of committing a move onto it"

A→B→A 反复横跳：记录不断覆盖，没 commit 什么都不发生；选回原 CLI = 撤掉记录（不会 commit 一个 `session_agent_unchanged` 会拒绝的请求）。

### 故意不持久化（pending 深挖）

> "Deliberately unpersisted: once a chat starts or a move commits, the agent lives on the session itself"

- ① 聊天已开始 → agent 活在 session（数据库）→ 不用恢复；
- ② 换绑已 commit → agent 活在 session → 不用恢复；
- ③ 什么都没发生（只选了没发消息）→ 本来就没生效 → 丢了无妨（刷新后 session 还是旧 agent，重新选一下）；
- 类比：**pending = 购物车（想买没下单）；session = 已下订单**——刷新购物车清空无妨，订单还在。

## 四、懒注入 transcript（换绑时不发任何东西）

```rust
handoff_pending: true,   // insert_actor 时标记
```

- **换绑那一刻：什么都不发送**（只认领会话 + 换绑定）；
- **下一条 prompt 时**：transcript 作为 leading content block 插在用户消息前面（第 9 课 handoff 渲染）；
- **"被换绑又放弃的会话成本为零"**——换绑后用户再也不说话，就没注入过，白省；
- 判断要不要注入：binding_needs_handoff（第 9 课，从记录推导）。

## 五、为什么不保留旧绑定（Q6 深挖）

**"丢掉"丢的是什么**：**agent 自己记录的上下文（model context）**——不同 agent 的上下文不能迁移（第 9 课核心：Ora owns the transcript, agent owns the model context）。

**保留旧绑定方案（否决）**：
```
opencode 聊 → 换 claude（保留 A）→ claude 聊 5 回合 → 换回 opencode
  → 重用 A？→ A 的上下文停在离开那一刻 → 缺 claude 的 5 个回合
  → 调和过期上下文（复杂、不可预测、可能漏回合）❌
```

**不保留方案（采用）**：
```
换回 opencode → 新建全新会话 C → 下一条 prompt 注入【完整 transcript】
  → C 知道全部对话（含 claude 期间 5 个回合）→ 简单、可预测 ✅
```

注释原话："its context stops at the moment it was left, so returning to it would need the intervening turns anyway — and injecting a full transcript into a fresh session is simpler and more predictable than reconciling a stale one."

**修正"换回 = 没有变化"的误解**：换回原 CLI **不是没有变化**——是**再走一次完整的 switch**（warm opencode → claim → 换绑 → 下一条 prompt 注入 transcript）。旧绑定**每次切换都被丢**，换回 = 从零再换一次。

**transcript 是"精简取舍"不是"压缩"**：保留用户消息/助手回复全文 + 工具标题结果；丢推理（属于旧 agent，带过来误导）、工具输入输出（巨大，挤掉对话本身）、计划（到达即过期）。**目标不是"尽可能多"，是"恰到好处"**——太多细节会挤掉对话本身（crowd out the conversation itself）。

## 六、整条链路（三课收官全景）

```
【第 8 课】用户点开选择器 → 用 key warm 目标 CLI（看模型）
【第 10 课】用户选模型（setConfig，换绑前已定；不走 prompt）
【第 8 课】点选 CLI → 前端记 switch（pending）+ 预热（不立即切换）
【第 11 课】发下一条消息 → commit：
    ① 前置检查（unchanged/degraded，在 claim 之前）
    ② claim（同 key 认领/现场新建，加锁防插队）
    ③ 认领成功才释放旧绑定（失败不动原绑定，reservation drop 退回）
    ④ 换绑定 + 记 AgentSwitched（第 9 课历史）+ handoff_pending: true
    ⑤ 响应带 available_commands + config_options
【第 9 课】下一条 prompt 前：懒注入 transcript（binding_needs_handoff 判断）
```

## 七、检查题答案（10 题，详细版）

**Q1. 切换 agent 时哪些不变、哪些变？**
不变：Ora session id（数据库身份证）、Task、对话历史（数据库记录）——注意历史文件本身不变，但 transcript 是要**注入给新 agent** 的（第 9 课）。变：provider session id（新 agent 分配的新房间号）、actor（新的）、路由（新代新 token）、绑定（CLI 换掉）。

**Q2. session_agent_unchanged 为什么要在 claim 之前检查？**
没有检查 → claim 会 warm/新建一个 provider 会话（池子没有就现场建）→ 认领到新会话 B → 换绑（拆 A 绑 B）→ 同一个 CLI、同一个会话、什么都没变，但**白白建了一个 provider 会话 B + 拆了会话 A**（纯折腾）。检查在 claim 之前 = 连新建都不让发生。浪费的是 **provider 会话（warm 新建的）**，不是 Ora 会话（它本来就在数据库）。

**Q3. 认领失败时原绑定会怎样？为什么？**
原绑定纹丝不动。因为顺序是"先认领、成功才动旧"——认领失败时旧绑定还没被碰。如果反序（先动旧再认领），失败 = 两头空（旧会话废了、新会话没有）= 用户卡死。另外 WarmReservation 是对象，失败时 drop 自动退回池子，claude 会话也不浪费。

**Q4. 为什么"点选 CLI"不立即换绑、要等下一条消息？**
因为点击时执行换绑会拆掉正在回复的 agent（mid-reply）——旧 actor 被停、路由被摘、回复被拦腰砍断。所以点选只做"记录 switch + 预热"，发下一条消息时（没有回复在跑）才真正换绑。

**Q5. 为什么换绑响应必须带 available_commands 和 config_options？**
因为 ACP 只在 session/new 和 session/load 时报告命令和配置——换绑之后没有任何 API 能再问一次。不带上，前端永远拿不到新 CLI 的命令/模型，模型选择器会一直显示旧 CLI 的模型。claim 能带上是因为预热时 config_options 记录在 WarmAttachment 里。

**Q6. 为什么不保留旧绑定？换回原 CLI 时靠什么？**
旧绑定的上下文（agent 的 model context）停在离开那一刻——换回来时中间的回合约已经发生，旧上下文是残缺的；调和残缺上下文复杂不可预测，丢掉重来 + 注入完整 transcript 简单可靠。换回原 CLI = 再走一次完整 switch（warm → claim → 换绑 → 下一条 prompt 注入 transcript），不是"没有变化"。

**Q7. pending 是什么？为什么故意不持久化？**
pending = 还没兑现的选择（选了 claude 但还没发消息），两种形状：selections（未开始聊天表面的默认 agent）、switches（持久会话要换绑的 agent）。故意不持久化：开始聊天/换绑成功后 agent 活在 session 上（不用恢复）；没兑现的丢了无妨（刷新后重新选一下）。pending = 意图，session = 事实。

**Q8. "丢掉旧绑定"丢的是什么？为什么 transcript 是"精简取舍"不是"压缩"？**
丢的是 agent 自己记录的上下文（model context）——不同 agent 的上下文不能迁移（Ora owns transcript, agent owns model context）。transcript 保留用户消息/助手回复全文 + 工具标题结果；丢推理（属于旧 agent，带过来误导）、工具输入输出（巨大挤掉对话）、计划（到达即过期）。目标不是"尽可能多"而是"恰到好处"。

**Q9. 懒注入 vs 立即注入/重放的区别？为什么懒？**
换绑时不发任何东西，下一条 prompt 前才把 transcript 作为 leading content block 插入。"被换绑又放弃的会话成本为零"——换绑后用户不再说话就没注入过，白省。判断要不要注入靠 binding_needs_handoff（从记录推导，第 9 课）。

**Q10. 把第 8/9/10/11 课串起来的完整链路？**
见第六节：warm（8）→ 选模型（10）→ 点选记 pending（8）→ 发消息 commit：前置检查→claim→先认领后放旧→换绑定记 AgentSwitched→响应带配置（11）→ 下一条 prompt 懒注入 transcript（9）。

## 八、术语表新增

Switch Agent（切换 Agent）、SwitchSessionAgent（换绑契约）、session_agent_unchanged（未变更错误）、handoff_pending（待注入标记）、pick vs commit（选择 vs 提交）、PendingAgentStore（待定选择仓库）、selections / switches（两种 pending 形状）、懒注入（lazy injection）、旧绑定释放。详见桌面 software technical terms.md。

## 九、下一课预告

> 第 12 课：Skill 体系与 AgentDefinition——技能包怎么发现、导入、校验、落库（zip-slip 防护、SKILL.md front matter 校验、导入会话生命周期）。
