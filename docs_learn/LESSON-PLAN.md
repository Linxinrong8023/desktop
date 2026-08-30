# 学习路线图（第 6 课起）

> 本文件是 docs_learn 的**章节规划**，用于防止“学了没存档”。
> 每学完一课，把总结写入 `lesson-XX.md` 并在 [README.md](./README.md) 目录里登记。
> 已学：第 1~6 课（lesson-01 ~ lesson-06）。
> 说明：第 1~6 课在“基础学习 session”中完成（本 session 开始时 workflow 尚未出现，lesson-06 的实际内容以本 session 的 gitlancer + Task/Worktree 为准，workflow 相关内容作为新专题按下文规划后续学习）。
> 四个 ⭐ 专题为**必须单独学习**的环节（用户特别指定）。

## 当前优先级调整：插件系统专题

为面试准备，原第 18~21 课暂缓，先完成独立的 HTML 微课程。每课以面试表达和主动回忆为验收，不以“阅读过”作为完成。

> 2026-08-29 起点校正：用户明确表示没有插件系统和软件工程概念基础。以下每个序号都可以继续拆成多次对话；只有主动回忆通过后才前进。

1. **插件是什么**：主程序、内置能力、外部安装能力；为什么插件化能降低扩展成本。
2. **四个必备积木**：文件、程序、进程、通信规则，一次只学习一个。
3. **Package / Contribution / Runtime 三分法**：为什么安装、能力类型和进程状态必须解耦。
4. **发布到安装的供应链**：Registry → Manifest → Installer → Discovery → InstalledPlugin。
5. **Plugin Runtime 协议**：Deno、帧协议、双向 JSON-RPC、不可变 capability registration。
6. **Lifecycle 控制面与数据面**：唯一进程所有者、按需启动、generation、connection、notification sink。
7. **Agent 插件端到端**：Ora → Plugin adapter → Agent CLI 的双进程模型，ACP 透明转发与 Supervisor。
8. **静态贡献对比**：Skill、MCP、Webview 如何不启动通用 runtime；补充 Workbench 与 Hook。
9. **安全与故障语义**：包路径、权限、Host requests、进程树、失败隔离及当前沙箱缺口。
10. **面试白板与扩展演练**：三分钟讲解、深挖问题、设计一种新插件类型并定位改动层。

---

## 第一段：Rust 后端纵深（业务 → 运行时 → 功能模块）

### 第 6 课：Task 与 Git Worktree（gitlancer）—— ✅ 已完成（本 session）
- **衔接**：第 5 课预告即本课。
- **核心问题**：创建任务时怎么建 linked worktree？Task/Worktree 怎么关联？Git 失败时怎么补偿数据库写入？为什么删任务不删 Git？
- **代码地图**：
  - `crates/gitlancer/`（domain / exec / git / parse 四层，GitRunner trait 静态分发）
  - `docs/task-worktrees.md`、`docs/task-workspace-files.md`
  - `crates/application/src/task/`、`crates/application/src/worktree/`
  - `crates/db/src/repository/worktree.rs`
- **本 session 实际覆盖（以 lesson-06.md 为准）**：
  - gitlancer 四层 + 请求/响应对象模式 + newtype 类型安全（BranchName/CommitId 防位置写反）
  - 创建流程 7 步 + 8 位前缀冲突检查（目录/分支两道证据）
  - 先 Git 后 DB + 三层补偿（Force 模式、返回原错误）
  - 删除语义（当前分支：不碰 Git；PR #169 观察项）
  - 路径解析（存 branch_name、问 git 权威）
  - 深入追问 6 点见 lesson-06.md 第十五节
- **⚠️ 观察项（PR #169）**：`fix(backend): clean up Git resources on aggregate deletion` 未合并——将把“删除不碰 Git”改为“删除时清理 worktree 目录 + ora/<prefix> 分支”。当前分支（project_learn）仍是旧行为；合并后需回看本课删除语义章节。

### 第 7 课：Agent Runtime 总览——ACP 协议与进程监督
- **核心问题**：Backend 启动时怎么拉起 5 个 CLI 子进程（opencode/nga/codeagentcli/claude/codex）？ACP 是什么协议？initialize 握手怎么协商能力？事件怎么路由？背压和超时怎么设计？
- **代码地图**：
  - `crates/backend/src/agent_runtime/`（connection.rs 的 ConnectionSupervisor、manager、actor）
  - `crates/acp/`（reader / peer / pending / trace）
  - `docs/agent-runtime.md` 前半（Process and Session Lifecycle、Flow Control、Timeouts）
- **要点**：每 CLI 一个受监督子进程 + 独立重试（250ms 翻倍到 30s）、连接丢失只影响该 CLI、按 provider session id 路由事件、每会话 256 FIFO、8MiB 帧、30s 非活动超时、`agent_runtime_unavailable`。

### 第 8 课：Session 生命周期与 Warm Session
- **核心问题**：一个会话从打开到对话经历了什么？warm session 为什么存在？attach / load / prompt / stop 的状态机？为什么一个会话同时只允许一个操作？
- **代码地图**：
  - `crates/backend/src/session.rs`、`crates/backend/src/agent_runtime/manager.rs`
  - `docs/agent-runtime.md` "Warm Sessions" 节
- **要点**：warm 键 `(target, agent_cli, client_id)`、attach 按 identifier 命名 vs 切换按 key 命名（为什么）、Running/Stopped 状态、会话标题获取窗口、warm 只活在内存不落库、同会话操作串行。

### 第 9 课：⭐ 保存上下文信息——ora-history 会话历史与 transcript（关键专题 1）
- **核心问题**：Ora 怎么保存一段对话？为什么自己记、不靠 agent 复述？文件格式和顺序规则？写失败（degraded）怎么办？换 agent 时上下文怎么带过去（handoff transcript）？
- **代码地图**：
  - `crates/history/` 全部（assembler / writer / record / handoff / path / reader）
  - `crates/backend/src/session_history.rs`
  - `docs/agent-runtime.md` "Session History" / "Degraded History" 节
  - 前端 `packages/chat/src/store.ts` 的 `HistoryBuilder`
- **要点**：每会话一个 append-only JSONL、`<root>/<id[0..2]>/<id[2..4]>/<id>.jsonl` 分片路径、positions 定义时间线（重复 position = 修正）、只记 settled 内容（不记注入的上下文）、`TurnEnded` 带 stopReason（provider 从不带）、Gap 记录 + `resumeSessionHistory`、"Ora owns the transcript; the agent owns the model context"。

### 第 10 课：⭐ 模型选择与切换（Model Selector）（关键专题 2）
- **核心问题**：前端模型选择器怎么知道有哪些模型？为什么"必须先有 session 才能选模型"？选完模型会话怎么记住？切模型在时间线里怎么体现？
- **代码地图**：
  - 前端：`packages/chat/src/model-option.ts`、`packages/chat/src/store.ts`（`setSessionConfig` / `recordModelChange` / `adoptSwitchedAgent`）、`packages/app-shell/src/features/chat/model-selector.tsx`、`model-catalog.ts`、`state/hooks/use-workflow-agent-models.ts`
  - 后端：config options 机制（`initialize` 握手广告 config-option 能力、`setConfig` 操作、`config_option_update` 更新）
- **要点**：ACP 只在 `session/new` / `session/load` 回复里报告 config options → 打开聊天面就先 warm；model selector = category "model" 的 select 选项（兜底：唯一 select）；agent 的回复是权威（可能调整/拒绝请求值）；前端在 transcript 里记 `modelChanges` 分隔线（首轮前/换绑前不记）；workflow 节点也复用同一模型目录。

### 第 11 课：⭐ 切换 Agent（会话换绑）（关键专题 3）
- **核心问题**：怎么把一段对话换到另一个 CLI？warm pool 怎么认领？为什么 transcript 懒注入而不是重放？为什么不保留旧绑定？
- **代码地图**：
  - 后端：`docs/agent-runtime.md` "Switching Agents" 整节、`crates/backend/src/agent_runtime/` 的 switch 逻辑、`switchSessionAgent` 契约
  - 前端：`packages/app-shell/src/state/stores/pending-agent-store.ts`、`packages/chat/src/store.ts` 的 `adoptSwitchedAgent`、workspace 的 agent picker
- **要点**：会话保留 id / Task / 历史，只有 binding 变；新绑定从 warm pool 按同 key **认领**（不是现握手），模型选择在换绑前已做 → 换绑后保留；pick 记录 vs commit（下一条消息才真正提交，选中 CLI 时先 warm 对方）；认领失败不动原绑定；懒注入 transcript（换绑不发送任何东西，下一条 prompt 前插一段 leading content block）；不保留旧绑定（上下文停在离开那一刻）；`session_agent_unchanged` 提前拒绝。

### 第 12 课：Skill 体系与 AgentDefinition
- **核心问题**：技能包怎么发现、导入、校验、落库？Skill / AgentDefinition / skill_import 各管什么？
- **代码地图**：
  - `crates/skill-package/`（README 的职责/边界）
  - `crates/application/src/skill/`、`crates/application/src/skill_import/`
  - `crates/backend/src/skill_reconciliation.rs`
  - desktop 的 skill_marketplace、`docs/application-contracts.md` 相关部分
- **要点**：zip-slip 防护、扩展比预算、SKILL.md front matter 校验、最近 manifest 归属规则、导入会话生命周期（prepare / preview / commit / cancel）、200MiB 上限、技能落库与 reconciliation。

### 第 13 课：Spec 管理与租约（ProjectWorkContext）—— ✅ 已完成（project_learn 分支：租约已移除）
- **核心问题**：Spec 目录怎么发现和索引？scheduler 干什么？
- **⚠️ 分支差异（重要）**：PWC 租约**已从当前分支移除**——`schema_v0011`（tail migration）`DROP TABLE project_work_contexts`，无 application/domain 代码，无 lease 逻辑；LESSON-PLAN 原写的“租约 120s 过期、窗口独占、Web 占座 vs Desktop 不支持”是**旧设计**，已删除。scheduler 现在只被 title acquisition/polling 当延迟定时器用（60s/10s/3s），无 cron 任务。
- **代码地图（实际）**：`crates/application/src/spec/`（handlers/ports）、`docs/spec-management.md`、`crates/scheduler/`、`crates/db/src/migration/schema_v0011.rs`（租约移除证据）
- **要点（实际）**：Spec 管理 = 索引并只读展示磁盘上已有的规格 md 文档（不创建/不修改，文档属于用户文件系统工具）；SpecTarget（project/task，task 用 agent 同 cwd）；默认候选目录（OpenSpec: openspec/specs+changes、Superpowers: docs/superpowers/specs+plans+docs/plans、Custom: specs+docs/specs）；有界发现（Git ignore + 最深度归属 + 大小写合并）；安全（catalog/read 不暴露绝对根、canonicalize、只读 .md/.mdx 且仍在 catalog 内、ripgrep 15s/8MiB/10000 限额）；前端（Specs 子视图、HTML 不执行/图片阻止/仅 catalog 链接可导航、项目根不可注册为源）；scheduler 特性（迭代不重叠、错过跳过、DelayHandle 取消语义）。
- **观察项**：LESSON-PLAN 原规划滞后于代码（第三次分支差异）；lesson-13.md 已存档。

### 第 14 课：task_diff 与文件系统层 —— ✅ 已完成（本 session）
- **核心问题**：diff 视图的数据哪来的？`ora-fs` 提供什么？工作区文件浏览怎么限制？
- **代码地图**：
  - `crates/application/src/task_diff/`（README：端口/不变量）、`crates/backend/src/task_diff.rs`
  - `crates/fs/`（path/workspace/search/watch/error）、workspace explorer（web 端）
  - `docs/task-workspace-files.md`
- **本 session 实际覆盖（以 lesson-14.md 为准）**：
  - 两个功能：Task Diff（Git 变更审查 + 评论：diff_id 稳定计算、anchor 必须仍匹配当前 patch 防 stale、端口静态分发、超大 patch 截断）
  - Workspace Files（只读浏览/搜索/查看/行选择/watcher）：四层分工（fs → 映射 → HTTP/Tauri → UI）、客户端从不提供 root
  - ora-fs 5 能力详解：path 跨平台统一、workspace canonical containment（canonicalize 后检查防 symlink 逃逸、TOCTOU 诚实标注）、search 15s/8MiB/10000 + 固定文本 + 截断上报、watch 100ms 合并 + rename 双路径 + 歧义 rescan、error 类型化 + adapter 映射
  - 安全边界汇总表 + 共享层动机（AGENTS.md 铁律）
- **要点**：每轮 agent 的增量文件变化（additions/deletions）、turn diff 与工具调用关联、ripgrep 注入、15s / 8MiB / 10000 结果限制、截断上报。

---

## 第二段：Workflow 专线

### 第 15 课：Workflow 定义与版本管理
- **核心问题**：workflow 怎么存？draft / publish / version 生命周期？为什么草稿可改、发布快照不可变？
- **代码地图**：
  - `docs/workflow.md`、`crates/application/src/workflow/`
  - `crates/contracts/src/workflow.rs`、相关 db 迁移
- **要点**：Workflow + WorkflowSnapshot 两实体、draft 保留字符串、publish 复制不可变（published.updated_at 为 NULL）、rollback / activate、版本命名规则（用户自定义 vs `v{timestamp}`、URL 安全、部分唯一索引）、graph 作为不透明 React Flow JSON、读取模型不带 graph。

### 第 16 课：Workflow 运行引擎
- **核心问题**：一个 workflow 怎么跑起来？run CRUD、node run、HITL、串行状态机怎么设计？
- **代码地图**：
  - `crates/application/src/workflow_run/engine/`（engine.rs / ports.rs / graph.rs / node_type.rs / README.md）
  - `crates/backend/src/workflow_run_engine.rs`、`workflow_run_executor.rs`、`workflow_run_prerequisites.rs`
  - `docs/workflow.md` "Workflow runs" 节
- **要点**：快照钉住（run 冻结发布版本）、专用 run-task + Git worktree、5 值状态枚举（Pending/Running/Succeeded/Failed/Cancelled）、petgraph DAG 解析与校验、`NodeExecutor` 委托 agent 会话、完成通过 `complete_node`/`fail_node` 回调、**每 run 一个串行 executor 保证状态转换串行**、HITL（start/restart/cancel）、`current_nodes` 锚点、快照保护（SnapshotInUse / ActiveRuns）。

### 第 17 课：⭐ Workflow 前端设计模式（关键专题 4）
- **核心问题**：前端怎么画 workflow 图？设计模式是什么：原生 React Flow 形状、ports 抽象、内存适配器、UI-free runtime？
- **代码地图**：
  - `packages/workflow-mock/`（node-factory、node-data、validation、capabilities、demo、version-history、README）
  - `packages/workflow-runtime/`（ports、graph-codec、workflow-path-order、memory、run-projection、README）
  - `packages/app-shell/src/features/workflow-run/`（theater、run-act-*、HITL composer）
- **要点**：**React Flow 原生形状 = 单一数据源**（不复制 DTO、无 adapter 层）、可执行字段放 `data` 扩展点、agentConfig 版本化（CLI/模型/Role/Skill/自定义 prompt）、`createMockWorkflowNode` 拥有默认值、导入校验（唯一 id/合法端点/单 Start）；`@ora/workflow-runtime` 定义 Host/Run ports + 共享 run 类型、graph-codec 归一化为 `WorkflowDefinition`、`workflowPathOrder`（拓扑优先 + 画布位置决胜）、`createMemoryWorkflowRuntime` 内存适配器（生产代码只允许在组合根用 memory 子路径）、事件带序列/游标（未来 NDJSON 重连不丢）；Theater 舞台 UI（act / stage / HITL composer）。

---

## 第三段：前端 / Web / 桌面

### 第 18 课：前端契约 SDK（@ora/contracts）
- **核心问题**：前端怎么调用后端？哪些生成、哪些手写？三种 transport？错误怎么解码和本地化？
- **代码地图**：`docs/frontend-contract-sdk.md`、`packages/contracts/src/`（client.ts / transport.ts / fetch.ts / endpoints.ts）、`xtask` 的 export-contracts
- **要点**：endpoints manifest（operation_name / namespace / method / path / request/response 类型）、ts-rs 生成 DTO、ts-to-zod 派生 error.schema、客户端与 Rust 编译期锁步（漏改 client.ts 会 tsc 报错）、fetch vs Tauri transport、decodeRemoteError（RemoteContractError / UnknownRemoteError / LocalTransportError）、i18n 本地化、`stream_already_consumed`。

### 第 19 课：app-shell 与聊天状态管理
- **核心问题**：Web/桌面共享的壳怎么组装？聊天状态怎么管？流式响应怎么变成 UI？
- **代码地图**：`packages/app-shell/src/`（app-shell.tsx、chat-store-context、contracts-client-context、features/chat、features/workspace）、`packages/chat/`
- **要点**：app-shell 装配（分店接线）、chat store（zustand vanilla）、乐观回合（先上屏再发 prompt）、流式文本批处理（4KiB 上限 + 16ms 刷屏）、工具调用时间线（pending→in_progress→终态）、turn 结束兜底结算、load 重放重建 transcript（HistoryBuilder）。

### 第 20 课：Web 服务器运行时
- **核心问题**：HTTP 服务器怎么工作？路由、中间件、NDJSON 流、健康检查？
- **代码地图**：`docs/web-server-runtime.md`、`apps/web/server/src/`（routes.rs / app_state.rs / error.rs / handlers/sessions.rs / bootstrap.rs）
- **要点**：axum 路由（共享路径常量）、AppState 注入、request context 中间件（request_id + span）、NDJSON data/error/end 帧、DeferredCompletion、watchAppEvents 事件流、ORA_DATA_DIR 派生全部路径、时区处理。

### 第 21 课：桌面运行时（Tauri）
- **核心问题**：桌面版和 Web 版差在哪？IPC、Channel 流、配置、平台专属命令？
- **代码地图**：`docs/desktop-runtime.md`、`apps/desktop/src-tauri/`（commands.rs / lib.rs / config.rs / error.rs / state.rs）
- **要点**：独立 Cargo workspace（有自己的 Cargo.lock）、Tauri transport 映射命令 + Channel 帧、`unsupported_operation`（PWC 三操作 + listDirectory）、平台专属命令（get_desktop_config / set_worktree_root / resolve_task_cwd / open_location）、app_data_dir 全部路径（sqlite/config/logs/worktrees/sessions/skills）、config.json 版本化原子写、时区固定一次。

---

## 待确认/后续可展开（非必修）

- `crates/plugin-manager`（插件发现，较小）
- `docs/application-contracts.md`、`docs/domain-models.md`（可作第 2、3 课的补充回看）
- `docs/gitlancer-architecture.md` 全文精读（可并入第 6 课）
- `docs/runtime-logging.md`（可并入第 20/21 课）
- `packages/plugin-sdk`、`packages/ui`（工具组件库）
- `apps/web/client` 路由与页面装配（可并入第 19 课）

---

## 四个关键专题速查（为什么必须单独一课）

| 专题 | 学完能回答 | 核心代码 |
|---|---|---|
| 保存上下文信息 | 对话存在哪、怎么记、写坏了怎么办、换 agent 怎么带过去 | `crates/history/`、`session_history.rs` |
| 模型选择与切换 | 模型列表哪来的、为什么先 warm 才能选、切换怎么留痕 | `model-option.ts`、`store.ts`、`model-selector.tsx` |
| 切换 Agent | 换绑走什么流程、warm pool 认领、为什么懒注入 | `agent-runtime.md` Switching Agents、`pending-agent-store.ts` |
| Workflow 设计模式 | 前端为何不复制 DTO、ports/内存适配器/UI-free 怎么用、引擎怎么保证串行 | `workflow-mock/`、`workflow-runtime/`、`workflow_run/engine/` |
