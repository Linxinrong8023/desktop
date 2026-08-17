# 第 6 课：Task 与 Git Worktree（gitlancer 与任务工作树）（总结）

> 本课从"Git 概念都不懂"开始，互动式学习到能横向对比 subagent / Dify。
> 内容覆盖：Git 基础概念补课、gitlancer 四层架构、任务创建流程与补偿、删除语义、路径解析、workflow 的 git 三大用途。

## 〇、本课两个心智模型（先背下来）

1. **先 Git 后 DB + 补偿**：坏状态永远留在用户看不见的一侧（宁可留目录残留，绝不留"假任务"）。
2. **存身份、问权威**：数据库存 branch_name（身份），路径永远问 git（权威）；git 管"现状"，DB 管"档案"。

## 一、Git 基础概念补课（本课要用到的地基）

| 概念 | 一句话 |
|---|---|
| 工作区 / 暂存区 / `.git` | 你改文件的目录 / `git add` 的中间地带 / 仓库的"数据库"（对象、refs、配置） |
| 分支 | 本质是 `refs/heads/` 下的一个**指针文件**，内容是 commit id；commit 不可变，分支会移动 |
| refs | 给 commit 起名字的引用：`refs/heads/`（本地分支）、`refs/remotes/`（远程）、`refs/tags/`，HEAD 也是 ref |
| porcelain / plumbing | 给人看的漂亮输出（随时会变）vs 给机器的稳定格式；`--porcelain` 标志把前者变成后者，gitlancer 只解析稳定格式 |
| `git rev-parse <ref>^{commit}` | 把 ref 剥成不可变 commit id（`^{commit}` = 剥 tag 到 commit） |
| `git for-each-ref refs/heads` | 只读列出本地分支名，无网络副作用 |
| detached HEAD | HEAD 直接指向 commit、不指向分支；porcelain 里没有 `branch` 行；Ora 用 `branch_name: None` 表示（合法状态，不拒绝） |
| linked worktree 的 `.git` | 是一个**文件**（内容 `gitdir: <主gitdir>/worktrees/<name>`），不是目录；linked 共享主仓库对象库 |

**判断主/从 worktree**：主 checkout 的路径 == 仓库根（repo root）；否则是 linked。判断不靠"第几个"。

**为什么需要 linked worktree**：一个 checkout 目录同一时刻只能有一个分支；多个任务并发时要各自的分支 + 各自目录。linked worktree 复制的是 checkout 目录（工作区），**共享 `.git` 对象库**（历史共享、不重复占存储），且每个任务天生在自己的分支上——**永远不需要 checkout 切换**（切换是把工作区内容搬来搬去的高危操作）。

## 二、gitlancer 是什么：为什么有它

gitlancer = Ora 的**类型化 Git CLI 运行时**（不用 libgit2，纯包一层系统 git 命令）。

为什么不能"到处直接调 git 命令"（三个病 + 治法）：

| 病 | 治法 |
|---|---|
| 类型不安全：命令名/参数是字符串，拼错运行期才炸 | 类型化 API，拼错编译期报错 |
| 没法测试：不能模拟 git 失败 | `Git<R: GitRunner>` 可注入执行器，测试用 Fake/Recording runner |
| 没法观察：不知道命令慢/失败/读还是写 | `GitIntent`（ReadOnly/Mutating/Network）分类 + 命令日志 |

## 三、gitlancer 四层（本课核心）

```
domain ── 概念 + 规则（不 spawn 进程、不解析文本）
exec   ── 命令对象 + 执行器（怎么跑）
git    ── 类型化用例（要什么操作）
parse  ── 文本 → 类型（翻译官）
```

### 每层回答什么问题、包含什么

| 层 | 回答 | 包含 | 变化频率 |
|---|---|---|---|
| domain | Git 世界有哪些概念、什么合法 | `RepoRoot`/`WorktreeRoot`/`GitDir`/`BranchName`/`CommitId`/`WorktreeKind`/`RepoRelativePath`/`WorktreeHandle` | 几乎不变（最稳） |
| exec | 命令怎么执行 | `GitCommand{cwd,args,env,intent}`、`GitEnv`（禁交互/禁分页/LANG=C）、`GitRunner`、`CliGitRunner`（spawn 系统 git、并发读两管道、`run_bounded` 超限杀进程）、`RecordingGitRunner` | 环境相关 |
| git | Ora 需要哪些 Git 操作 | worktree / branch / commit / diff / status / push / base_branch 各一个模块，每个用例组装 `GitCommand` | 随需求增 |
| parse | git 文本怎么变程序数据 | 只解析稳定机器格式（porcelain/plumbing）；格式变了只动这一层 | **最容易变** |

### domain 层的价值（"让非法状态不可表示"）

- `RepoRelativePath` 只能通过 `WorktreeHandle::resolve_repo_relative_path` 获得（词法规范化、拒绝绝对路径、拒绝 `..` 逃出工作树 → `PathOutsideWorktree`）——**构造不出来的非法路径**，用的人不用再防；
- `branch_name: Option<BranchName>`——detached 是合法状态（None），不是错误；
- `WorktreeKind::Main | Linked`——任何 match 必须处理两个分支，不会"忘了区分主从"；
- 仓库根 / 工作树根 / gitdir 是三个不同类型——类型系统防传错。

### 为什么最后要"转成 domain"（纸条 → 卡片类比）

git 进程和程序之间**只能传文字**（纸条）。程序直接在纸条上做"找哪个分支对应哪个目录"的操作：容易错、别扭。所以 parse 把纸条翻译成"卡片"（类型化对象）：

```
纸条：worktree /Users/silon/ora
      branch refs/heads/ora/7f3a9c21
        ↓ parse（翻译官）
卡片：{ 路径 = …, 分支 = ora/7f3a9c21 }（WorktreeHandle）
```

- 卡片给**程序内部接下来要处理这些信息的代码**用（git 层查找、上层取路径），**不给 DB 用**——DB 只收从卡片里抽出的字符串（branch_name、commit id）；
- 类比 serde：JSON → DTO，DTO 是给处理逻辑用的，不是给数据库用的；
- **parse 就是 gitlancer 的 serde**（外部格式 → 内部类型）。

### 一次命令的完整旅程（resolve_worktree_by_branch）

```
git 层：组装 GitCommand{ "worktree list --porcelain", ReadOnly }
  → exec 层：spawn 系统 git → 拿回原始文本（GitOutput）
  → parse 层：文本 → Vec<WorktreeHandle>（逐行解析 worktree/branch/detached）
  → git 层：find(branch == ora/xxx) → 返回 WorktreeHandle
  → 上层：.worktree_root() 取真实路径
```

### 三条解析路径（易混）

| 路径 | 按什么找 | 谁用 |
|---|---|---|
| `resolve_worktree` | worktree 名（Linked 名 = 目录名；main = "main"） | 一般寻址 |
| `resolve_worktree_by_branch` | 检出的分支名 | **agent runtime 恢复任务目录**、删除补偿 |
| `find_worktree` | 任意嵌套路径（最深前缀匹配） | 判断某路径属于哪个工作树 |

### 错误模型

```
GitlancerError
  ├─ Domain: NotARepository / PathOutsideWorktree / WorktreeMismatch / MainWorktreeDeletionUnsupported / BranchNotFound / BranchAlreadyExists
  ├─ Exec:    GitNotFound / SpawnFailed / NonZeroExit / OutputTooLarge（diff 映射为 DiffTooLarge）
  └─ Parse:   InvalidWorktreeList / InvalidStatus / …
```

## 四、创建任务完整流程（worktree 模式）

入口按 `workspace_mode` 分叉（默认 Worktree）：

```
① 校验 base_branch 非空                    → TaskBaseBranchRequired
② validate_repository（discover_repository）→ 不是 git 仓库报 NotARepository
③ select_available_task_id：生成 id + 两处冲突检查（最多 3 次）
④ 推导 branch_name = "ora/<id 前 8 位>"，目录 = <worktree_root>/<完整 id>
⑤ create_task_worktree（适配器）：
     resolve_worktree_base_commit（base 分支 → 不可变 commit id）→ base_branch_not_found
     create_parent_directory
     git worktree add -b <branch> <path> <commit>        ← 进 gitlancer 四层
⑥ 落 worktree 行（branch_name + baseline commit）
⑦ 落 task 行（worktree_id 指向 ⑥）
⑧ 返回
```

**project_root 模式**：不建 worktree、不落 worktree 行、`task.worktree_id = None`，直接用项目根当 cwd。

### 命名规则与冲突检查（为什么是 8 位）

- 分支名用 **8 位前缀**（可读性），目录用**完整 task id**（唯一性）；
- 前缀可能被占两处：① work_dir 下已有同前缀目录；② 已有 `ora/<前缀>` 分支（**孤立分支也算**——目录删了分支还在，前缀仍被保留）；
- 3 次冲突 → `TaskWorktreeIdExhausted`，绝不死循环。

### 为什么 base 先生成不可变 commit id

`git worktree add -b <branch> <path> <base>` 的 base 必须是 commit id：任务从"此刻的 main"出发，之后 main 怎么动不影响任务起点。该 commit 同时记录为 **WorktreeBaseline**（`base_commit_id` 列），供 turn-diff 计算基线。

## 五、先 Git 后 DB + 三层失败补偿（核心哲学）

**为什么先 Git 后 DB**（用户初判是反的，被纠正）：

| 顺序 | DB 成功、Git 失败 | 用户看到什么 |
|---|---|---|
| 先 DB 后 Git | 假任务：记录在、目录不在 | agent 一进去就崩，**坏状态可见** |
| 先 Git 后 DB | 目录残留（用户不可见），补偿删除即可 | 创建失败报错，列表干净 |

原则：**DB 失败容易补偿（软删一行），Git 失败/残留难补偿；所以先做难补偿的 Git，DB 失败就删掉 Git 留下的东西。坏状态永远留在用户看不见的一侧。**

三层补偿（按失败点）：

| 失败发生在 | 补偿动作 | 结果 |
|---|---|---|
| `git worktree add` 本身失败 | 什么都不写 | 无任何残留 |
| add 成功但 `find_worktree` 发现失败 | gitlancer 内部：`worktree remove --force` + `branch -D`（两个独立目标，一个失败也试另一个） | 报原错误 |
| worktree 行落库失败 | Force 删 git worktree | 报原错误 |
| task 行落库失败 | ① 软删 worktree 行 → ② Force 删 git worktree | 报原错误 |

- **Force 模式**：补偿时脏检出不能阻止回滚；
- **优先级**：DB 清理失败 > 文件系统清理失败 > 原错误——永远返回原始业务错误。

## 六、删除语义：谁的东西删谁的

**删用户任务：只删数据库记录，一行 git 都不调**（目录和 `ora/<前缀>` 分支留着）。

理由：
1. **所有权边界**：Git 资产归 Git/用户，Ora 无权抹掉用户的工作历史（commit、push 的东西）；
2. **可救回**：目录和分支还在，用户随时手动捡回来；删了是不可逆事故；
3. **判断标准**："用户没见过的东西才能删"——创建失败补偿是唯一删 Git 的场合（刚建的、用户没见过的残留，删了无损失）。

**删 workflow run：软删记录 + Force 删物理 worktree**（对比！）。

| | 删用户任务 | 删 workflow run |
|---|---|---|
| worktree | 保留 | **连根删** |
| 为什么 | 用户资产（产出、历史） | Ora 的一次性执行环境（"车间"），跑完/废弃就该拆 |

**能力已预留**：`TaskWorktreeProvisioner::delete_task_worktree` 已存在（创建补偿在用），产品将来做"删除时提醒并清理"直接复用。设计原则：破坏性、不可逆操作必须**显式**（用户确认），不隐式连带。

## 七、路径解析：存身份、问权威（git 账本类比）

- 配置的 worktree root 只是**创建目标**（"新东西往哪放"），**不是记录**；
- 已存在目录**绝不靠配置 root 拼路径**，而是：`task → worktree 行（branch_name）→ git worktree list --porcelain（权威）→ 真实路径`；
- **git 的账本**：`git worktree add` 时 git 在仓库 `.git` 里记下"分支 X 的 checkout 在 /A/T1"，跟着仓库走。用户改 Ora 的 root 配置只改"默认值"，**git 的账本一个字没动**；
- 为什么 A（拼路径）会错：root 被改（旧任务还在旧位置）、目录被重建（分支不变、位置变）——Ora 的记忆会过期，git 的账本不会；
- **为什么存 branch_name 不存路径**：路径可变（可移动/重建），身份稳定（分支名不变）——身份与位置分离；
- 代码链（`resolve_task_cwd`）：task 行 → worktree_id → worktree 行（校验 task_id 匹配 + Activity::Active）→ branch_name → `resolve_worktree_by_branch` → 目录（再校验 is_dir，否则 `task_worktree_unavailable`）；
- agent 会话启动时就用这个 cwd 干活（第 7 课的接口）。

## 八、workflow 用 git 的三件事

| 时机 | 干什么 | 谁在做 |
|---|---|---|
| 创建 run | 建专属 worktree + `ora/<8位>` 分支（**复用** `TaskWorktreeProvisioner`、`branch_name_for_task`、`worktree_path_for_task`）+ skills 物化到 `<worktree>/.agents/skills/` + 校验角色/技能 | `CreateWorkflowRunHandler` |
| 执行节点 | agent 节点在 run worktree 里干活，跑完算"这个节点改了什么" | `workflow_run_executor.rs` |
| 删除 run | **连物理 worktree 一起删（Force）** | `DeleteWorkflowRunHandler` |

细节：
- run + task（TaskType::Workflow）+ worktree **一个事务**落库；创建失败同样补偿删物理 worktree；
- skills 物化 = "run 出生即完整"，`start` 不再校验；
- 节点执行时 `resolve_task_cwd` 拿 run worktree 目录；`capture_worktree_snapshot`（`git ls-files -co`：tracked + untracked 全部文件+内容）节点启动前/后各打一次 → diff → `FileChange { path, additions, deletions }`；
- 为什么不用 gitlancer 的 diff：要包含 untracked 文件（`git status --porcelain` 会把 untracked 目录折叠成单个 `?? dir/`）。

## 九、节点间怎么传递（两条通道 + 功劳清单）

| 通道 | 传什么 | 给谁 |
|---|---|---|
| 文本通道 | 上游节点的**结论**（output）：`assemble_upstream` 收集所有传递前驱的成功输出，按拓扑序拼进下游 prompt；加上 run 的 kickoff input | 下游 agent |
| 实物通道 | 共享 worktree：上游改的文件**真实留在目录里**，下游直接看到合并后的现状 | 下游 agent |
| 功劳清单 | 每个节点的 `file_changes` 存在**自己那行**的 `workflow_node_runs.payload`（JSON） | **人/前端**（run 详情页展示"每个节点改了什么"） |

关键结论（用户自己推出来的）：
1. **执行链路不关心"谁改的"**：下游 agent 只看"现状 + 结论"，不需要账本；
2. **"哪个文件是哪个节点的"**：文件上无标记，归属靠**时间快照窗口**（"在谁的时间段里它变了"）——摄像头按时间段录像的类比；
3. **为什么记账**：结果可解释性（多 agent 接力要分得清功劳/责任）+ 客观性（output 会吹牛，git diff 不会）+ 复盘定位（节点级 git blame）；
4. **为什么预先算好存库、不展示时现查 git diff**：① 删 run 时 worktree 连根删，账要活得过 worktree（脱离环境独立存在）；② 现查 diff 分不出"哪个节点的增量"（归属窗口已过去）；
5. run 终态后 worktree **保留**（产物还能看），**用户删 run 时才删** worktree；账本无论何时都在 DB（软删只是不可见）。

## 十、workflow 定位（与 subagent / Dify 的对比）

**subagent 模式 vs Ora workflow**：

| | subagent（主从委派） | Ora workflow（静态 DAG） |
|---|---|---|
| 谁决定分工 | 主 agent 运行时动态决定 | 用户提前画好图，引擎按图执行 |
| 结构 | 动态树，不可预期 | 静态图，可预期、可审计 |
| 汇总 | 子 agent 汇报给主 agent | 没有主 agent——节点接力，账本记录，用户直接看 |

**Dify 的 LLM 节点 vs Ora 的 agent 节点**：

| | Dify | Ora |
|---|---|---|
| 执行单元 | **一次模型调用**（prompt → completion），无状态 | **完整 agent 会话**：CLI/模型/角色/技能，自主决策、改文件、跑命令 |
| 环境 | 平台内抽象（API/工具） | 真实 Git worktree（真实代码） |
| 产物 | 数据/文本 | 真实代码改动 + 账本 |

一句话：Dify 是"编排**模型调用**造应用"（输出答案），Ora 是"编排**多个 agent** 在代码库里干活"（输出 commit）。Ora 的 workflow 是用户自定义、可版本化（draft/publish）、可审计的一等公民产物。

## 十一、总装图

```
 用户 UI
   │
   ▼
 ora-application（业务编排：CreateTaskHandler / WorkflowRunEngine / resolve_task_cwd）
   │  ▲
   │  │ TaskWorktreeProvisioner 接口（唯一接缝，handler 看不见 Git 类型）
   ▼  │
 ┌──────────────────── gitlancer 四层 ────────────────────┐
 │  git 层（用例）→ exec 层（spawn）→ [系统 git] → parse 层 │
 │            ↓ 全部产出 domain 类型（概念+规则）            │
 └─────────────────────────────────────────────────────────┘
   │
   ▼
 ora-db（worktrees / tasks / workflow_node_runs：行 + 账本）
```

**总纲**：上层只会通过接口说"我要什么"；gitlancer 负责"怎么跑 git"；DB 负责"记什么账"。git 管现状、DB 管档案，两者靠"先 Git 后 DB + 补偿"对齐，靠"存身份问权威"保持同步。

## 十二、检查题（详细答案版）

**1. gitlancer 四层各管什么？哪层最容易变、哪层最稳？**

domain（概念+规则，不 spawn 进程不解析文本）、exec（命令对象+执行器，怎么跑）、git（类型化用例，要什么操作）、parse（文本→类型，翻译官）。**最容易变的是 parse**（git 各版本的机器输出格式有差异，status 还有 v1/v2 之分，所以只解析稳定格式，格式变了只动这一层）；**最稳定的是 domain**（仓库根/工作树/分支这些概念 git 一百年不会变）。git 层随需求增用例，exec 层兜环境问题（git 没装、输出超限、平台差异）。

**2. 为什么最后要转成 domain 类型？给谁用？**

git 进程和程序之间只能传文字（纸条）；程序直接在纸条上做“找哪个分支对应哪个目录”的操作容易错。parse 把纸条翻译成“卡片”（类型化对象，如 WorktreeHandle）——卡片好查、不会错（能 find、能取字段、编译期防传错）。**给程序内部接下来要处理这些信息的代码用**（git 层查找、上层取路径），**不给 DB 用**（DB 只收从卡片里抽出的字符串：branch_name、commit id）。类比 serde：JSON → DTO，DTO 是给处理逻辑用的，不是给数据库用的。

**3. 为什么先 Git 后 DB？**

比较两种顺序的失败后果：先 DB 后 Git → DB 成功、Git 失败 → 用户看到一个“假任务”（记录在、目录不在），agent 一进去就崩（**坏状态可见**）；先 Git 后 DB → Git 成功、DB 失败 → 目录残留（用户不可见），补偿删除即可。原则：**DB 失败容易补偿（软删一行），Git 失败/残留难补偿；所以先做难补偿的 Git，DB 失败就删掉 Git 留下的东西——坏状态永远留在用户看不见的一侧**。

**4. 创建流程各失败点的补偿分别是什么？**

按失败点分四层：① `git worktree add` 本身失败 → 什么都不写（无残留）；② add 成功但 `find_worktree` 发现失败 → gitlancer 内部清理（`worktree remove --force` + `branch -D`，两个独立目标，一个失败也试另一个）；③ worktree 行落库失败 → Force 删 git worktree；④ task 行落库失败 → 先软删 worktree 行，再 Force 删 git worktree。所有情况**永远返回原始业务错误**，补偿失败不吞掉主错误。Force 模式保证脏检出不能阻止回滚。

**5. 为什么删任务不删 Git、删 workflow run 却删 worktree？**

判断标准是“**谁的东西**”：用户手动创建的任务 = 用户的资产（目录里有产出、commit、push 的历史），Ora 无权替用户抹掉 → 只删自己的数据库记录，一行 git 都不调（目录和 `ora/<前缀>` 分支留着，用户随时能捡回）；workflow run = **Ora 自己的一次性执行环境**（“车间”，用户没在上面做自己的事）→ 删 run 时连根删（Force）。印证规则：“用户没见过的东西才能删”——创建失败补偿是唯一删 Git 的场合。`delete_task_worktree` 能力已预留，产品将来做“删除时提醒并清理”直接复用；但破坏性操作必须显式（用户确认），不隐式连带。

**6. 路径解析为什么存 branch_name 不存路径？**

存身份、问权威：路径**可变**（用户能移动目录、root 配置能改、目录可能被重建），身份**稳定**（`ora/<8位>` 分支名不会变）。配置的 worktree root 只是“新东西往哪放”的创建目标；已存在目录永远问 git（`git worktree list --porcelain` 是权威，git 从创建时刻就在自己 .git 的账本里记着“分支 X 的 checkout 在哪”，Ora 改配置影响不到那本账）。代码链：task → worktree 行（branch_name）→ `resolve_worktree_by_branch` → 真实目录。若用“当前配置 root + task_id”拼路径：root 被改后旧任务路径就拼错了。

**7. file_changes 记在哪个节点名下？给谁看？为什么不展示时现查 git diff？**

记在**各节点自己那行**的 `workflow_node_runs.payload`（JSON：stop_reason + file_changes）。**给前端/用户看**（run 详情页展示“每个节点改了什么”）——执行链路不读它（agent 只看 worktree 现状 + 上游结论文本）。**为什么不展示时现查 git diff**：① 删 run 时 worktree 连根删，账要活得过 worktree——必须预先算好存库，脱离环境独立存在；② 归属靠**时间快照窗口**（节点启动前/后的两次快照），展示时现查 diff 分不出“哪个节点的增量”——窗口过去了就没了，必须当场算。类比：摄像头按时间段录像，文件无主，归属 = “在谁的时间段里它变了”。

**8. 节点间怎么协作？**

两条通道 + 一份账：① **文本通道**——上游节点的结论（output）拼进下游 prompt 的 upstream 块（transitive lineage + run input）；② **实物通道**——共享 worktree，上游改的文件真实留在目录里，下游直接看合并后的现状；③ **功劳清单**（file_changes）不进 prompt，只存 DB 给用户看。核心：下游 agent 不需要知道“谁改的”，只需要“现状 + 结论”；账本是给“人”看的（结果可解释性：output 会吹牛，git diff 不会——节点级 git blame）。

## 十三、术语表新增

Linked Worktree、Porcelain/Plumbing、Detached HEAD、Ref、GitDir、Baseline（基线 commit）、Cascade Delete（级联软删）、Compensation（补偿清理）、Intent（ReadOnly/Mutating/Network）、NodeType（Start/Agent/Output，v1 可执行）、FileChange、Transitive Predecessor（传递前驱）、Static Dispatch（Git<R: GitRunner>，已深化）。

## 十四、下一课预告

> Agent Runtime：Backend 启动时怎么拉起 5 个 CLI 子进程（opencode/nga/codeagentcli/claude/codex）？ACP 是什么协议？initialize 握手怎么协商能力？事件怎么路由？一个会话从打开到对话经历了什么（warm session / attach / load / prompt）？——注意：第 7 课的 cwd 就是本课 `resolve_task_cwd` 给的。

---

## 十五、本课补充：会话中深入追问的六个理解（已有内容不重复）

以下 6 点是学习中反复追问后补上的理解，正文各节未展开，单独记录。

### 1. 为什么用“请求/响应对象”而不是散传参数

- Rust 函数参数**按位置匹配**，编译器只查类型、不查“本意”：两个 `String` 参数写反，编译器不报错，运行时数据才错；
- 请求对象字段**有名有姓**：写反容易发现；少写字段/写错字段名编译器直接报错；
- 加字段（可选项）不改已有调用方；整个请求对象可整体校验/记录/序列化；
- 类比：口头点餐（传错没人知道）vs 点餐单（字段清楚、能留底、可加备注栏）。

### 2. “位置写反”具体怎么发生

```rust
fn set_user(id: String, email: String) { ... }
set_user("user@example.com".into(), "u123".into());  // 编译器无感，数据错
```

请求对象版 `UserRequest { id: ..., email: ... }`：字段名在明处，错误从“编译器看不见”变成“编译器拦一半 + 人眼容易发现另一半”。

### 3. 前缀冲突检查的真相：目录检查不是防“目录撞”，是查“前缀被占”

- 目录用完整 UUID **永远不会撞**（学习中正确质疑过）；真正会撞的是**分支**（前 8 位相同 → 分支同名 → `git worktree add -b` 失败）；
- 目录检查 = 找“名字以 8 位前缀开头的目录”作为“前缀已被占”的**便宜证据**（starts_with）；
- 两道证据：目录（孤儿目录）+ 分支（孤儿分支）——覆盖任何方向的残留；
- “分支被删、目录还在”在正常流程**不会发生**（git 拒绝删被 checkout 的分支），只有脚本绕过（`git update-ref -d`）、元数据失联、强删工具才会 → 目录检查是**防御性兜底**；
- 成本极低（一次目录遍历）→ 便宜的保护宁可多做。

### 4. 分支名为什么用 8 位不用 32 位

- 技术上 32 位也合法；但 8 位可读（`git branch` 输出 / UI / 日志），32 位不可读；
- 冲突检查成本趋近于零（毫秒级目录遍历 + 任务创建非高频）→ “短名字 + 便宜检查” 比 “长名字 + 无检查” 划算；
- 一句话：可读性是用户天天看到的，检查成本是机器毫秒级的事。

### 5. TaskApi::create 为什么每次动态构造 handler

- 对比：ProjectApi 的 handler 启动时构造一次（SQLite 对全项目是同一个）；
- Task 的 handler 装着 `GitTaskWorktreeProvisioner`（**绑定特定 Git 仓库** = 项目的 root_path）；
- 不同项目 = 不同仓库 → 必须先查“任务属于哪个项目” → 每次现搭；
- 类比：固定工位（Project）vs 移动工位（Task，得先知道去哪个工地）。

### 6. 删除任务前先收集 session_ids 的顺序细节

- `session_ids` 必须在级联软删**之前**收集：软删后 `WHERE is_deleted = 0` 就查不到这些会话了；
- 收集的 ids 用于清理存盘上的会话历史文件（`remove_session_histories`）——先记下“要清理的东西”再动手删。
