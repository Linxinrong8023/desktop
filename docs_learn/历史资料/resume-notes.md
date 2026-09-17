# 简历素材：Ora 项目工作重点（resume-notes）

> 用途：简历上的"工作重点" + 面试可展开的素材。写作原则：
> ① 卖点导向（写"给用户/产品带来的价值"，不写实现机制）；
> ② 只写真事（不虚构产出/竞品分析/选型报告——面试一追问就穿帮）；
> ③ 实现细节留给面试讲，不进简历。
> 产品定位提醒：Ora 是 **agent 的 IDE（集成商）**——不自造 agent，通过 ACP 协议集成 opencode/claude/codex 等多家供应商。

---

## 四行简历点（每行 ≈60-75 字）

1. **切换 Agent**

> 负责 Agent 切换功能：支持 opencode/claude/codex 等多供应商自由切换，对话历史与工作上下文无缝迁移、不打断进行中的对话，打破单一 agent 锁定，随时试换零成本。

2. **工作流**

> 负责 Workflow 功能：将可视化工作流编排引入 agent IDE，用户可自定义多 agent 协作流程，节点支持 HITL 人工介入（同类 agent 产品少见），多角色协同完成复杂任务。

3. **日志系统**

> 负责日志系统：请求级全链路可观测（追踪、统一完成事件），生产错误链自动脱敏、可安全排障，不泄露敏感信息。

4. **ACP 协议集成**

> 负责基于 ACP 协议的多 agent 供应商集成：通过统一协议接入 opencode/claude/codex 等多家供应商，掌握能力协商、会话生命周期、配置/权限机制。

---

## 1. 切换 Agent（面试素材）

### 价值（简历背后的故事）

- **打破供应商锁定**：一套 IDE 用任意 agent，不被任何一家绑死；
- **对话无缝迁移**：换人聊天记录不丢、干活状态不丢（Ora 管 transcript，agent 管上下文——两条记忆线分离才让"换人"成为可能）；
- **不打断进行中的对话**：点选只记录意图（pick），等当前回合结束、下一条消息才真正切换（commit）——防止拆掉正在回复的 agent；
- **随时试换零成本**：试 A 不满意换 B，随时换回，对话都在（transcript 懒注入：换绑不发任何东西，被放弃的会话零开销）。

### 机制（被追问时讲）

- 会话保留 id/Task/历史，**只有绑定（binding）变**；三个标识：Ora session id（不变）/ provider session id（变）/ warmkey（找预热会话的预订信息）；
- 新绑定从 **warm 池按同 key 认领**（不是现场握手）：选择器用 key 预热目标 CLI（为显示模型），换绑用同一 key 认领——两头对上；
- **先认领、成功后才动原绑定**：认领失败 → 原绑定纹丝不动（用户还在原 agent 上聊天）；WarmReservation 是对象，失败 drop 自动退回池子；
- `session_agent_unchanged` 在 claim 之前检查：换到同一 CLI = 白建一个 provider 会话，纯浪费；
- **不保留旧绑定**：旧上下文停在离开那一刻（残缺），换回 = 重新走一次切换 + 注入完整 transcript（比调和过期上下文简单可预测）；
- transcript 是**有损精简**不是全量：保留用户/助手全文 + 工具标题结果，丢推理/工具输入输出/计划（目标"恰到好处"，太多细节挤掉对话本身）。

## 2. 工作流（面试素材）

### 价值（简历背后的故事）

- **产品定位差异化**：一般 agent 产品是"单 agent 对话"；Ora 把"可视化工作流编排"引进 agent IDE（类似 Dify 的理念）——用户自己画图编排多个 agent 分工协作；
- **节点 HITL（人工介入）**：某个节点停下等人类审批/输入再继续——**同类 agent 产品少见**，是差异化亮点。

### 机制（被追问时讲，⚠️ 只讲已实现的）

- 版本生命周期：**draft（草稿）可原地改、publish（发布）复制成不可变快照**（published.updated_at = NULL）、rollback（历史快照拷回草稿，不动发布指针）、activate（切换发布指针 + 同步草稿）；
- 版本字符串：用户自定义（如 v1.0.0）或自动 `v{timestamp}`；部分唯一索引防重名（软删后名字可复用）；
- 运行（run CRUD 已实现）：**运行冻结发布快照**（防止运行中改图）+ **专用 run-task 与 Git worktree**（隔离，互不干扰）+ 五值状态（Pending/Running/Succeeded/Failed/Cancelled）；
- 快照保护：在用快照不可删（SnapshotInUse）、有活动运行的 workflow 不可删（ActiveRuns）；
- 删除：拒绝活动运行（Running run / 非终态节点 / Running session），级联软删 + 注册 Git 清理任务异步删 worktree 和 ora/* 分支；
- ⚠️ **执行引擎（start/restart/HITL）尚未实现**（docs 明确 "execution engine builds on top of the same repository later"）——面试别讲"引擎/HITL 已上线"，只讲设计或标注实现中。

## 3. 日志系统（面试素材）

### 价值（简历背后的故事）

- **可观测性**：一个 request_id 贯穿请求/错误/完成事件——排障快，不是"有日志"而是"能追踪"；
- **安全性**：生产日志错误链自动脱敏——**能查日志还不怕泄密**（可以放心开日志排查）。

### 机制（被追问时讲）

- 进程级统一初始化：`ora-logging` 拥有 subscriber 设置、JSON 事件格式、sink 选择、轮转、保留、时区；
- 配置：ORA_LOG_LEVEL（trace~error）、ORA_LOG_MODE（stdout/file/both）、ORA_LOG_MAX_DAYS（默认 3 天）、ORA_TIMEZONE；日志路径由数据根派生（logs/ora.log，不可独立配置）；
- JSON 事件契约：timestamp/level/target/message + 可选 method/span/trace_id/request_id；业务字段按前缀路由（error.* → error 对象，其余 → context）；
- 请求关联：request_id 在 Web/Tauri/流式入口生成（UUID v4，客户端提供的 id 不作数），贯穿 span/错误帧/完成事件；每请求恰好一个完成事件（operation/outcome/duration_ms）；
- 错误分级：内部错误 ERROR、冲突 WARN、客户端错误 INFO、取消 DEBUG；
- ErrorReport：遍历 Rust source() 链；release 版清控制字符/限长/正则滤敏感文本、1024 节点上限防循环链；
- 非阻塞写入 + **不丢行**（写满通道用背压而不是丢日志）；
- Git 命令也进日志（gitlancer logger 桥）：记录子命令与耗时/退出码，但**不含敏感参数**（路径/凭据/提交信息/远程地址全滤掉）；
- 用 `ora_logging::ora_*!` 宏代替裸 tracing 宏（自动带 method 字段）。

## 4. ACP 协议集成（面试素材）

### 价值（简历背后的故事）

- 产品定位：**集成商不自造 agent**——各家私有接口五花八门，ACP 让 Ora 用一套代码接全部；
- 统一协议 = 多供应商即插即用的地基（切换 agent 功能依赖它）。

### 机制（被追问时讲）

- ACP = JSON-RPC 2.0 + stdio 的 agent 客户端协议；
- 能力协商：initialize 握手（15s 超时、验货 + 协商每家支持什么——session/list、config-option、permission 等能力差异被统一管理）；
- 会话生命周期：session/new（报告 config options——所以"先有会话才能选模型"）、load（恢复 provider 上下文）、prompt/stop/delete；StopReason（provider 从不带，Ora 自己记录）；
- 配置/权限：setConfig（agent 回复是权威，可能调整/拒绝请求值）、permission 请求进有序会话队列、空闲时自动拒绝；
- 集成 vs 自研取舍：把 agent 当服务集成，专注做 IDE 层（编排/切换/历史/权限），不重复造模型层。

---

## 诚实红线（面试前必须自查）

- ❌ 不写/不讲"竞品分析报告""选型报告"——没做过；
- ❌ 工作流不讲"执行引擎/HITL 已上线"——引擎尚未实现（有设计或实现中，说清楚）；
- ❌ 不把"研读代码"包装成"开发了 X"——简历写的是理解深度 + 价值视角，被追问时落到具体机制（上面的素材都能支撑）。

## 简历放置建议

- 前三条（切换/工作流/日志）像"功能负责人"的经历，放主体；
- 第 4 条（ACP 集成）放"技术理解/项目背景"类，或作为第一条的背景铺垫（产品核心是什么）；
- 整体面向：agent 工具/IDE/后端方向的岗位最对口。
