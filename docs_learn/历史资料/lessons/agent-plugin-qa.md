# Agent 插件学习问答：从安装到对话

> 整理日期：2026-09-09。根据本目录第 0001～0026 课整理，并结合当前 `project_learn` 分支源码校正。
> 用途：日常复习、口述练习、面试追问。先尝试自己回答问题，再看答案；不必一次背完。
> 第 37～40 题补充本次讨论中的 agent 切换和历史保存，不属于这 26 节插件课的全部原始内容。

## 一、先认识插件：Q1～Q5

课程：第 1 课（旧 HTML 页面已删除）、第 2 课（旧 HTML 页面已删除）、第 6 课（旧 HTML 页面已删除）、第 7 课（旧 HTML 页面已删除）。

### Q1：Agent 插件到底是什么？为什么要插件化？

**答：它是 Ora 与具体 agent 之间的接入适配器。** Ora 定义统一接口，插件处理某个 CLI 的启动、停止、模型查询和通信差异。

这样新增 agent 时，通常可以通过新增插件完成接入，减少修改 Ora 核心代码的需要。插件不是大模型本身，真正调用模型和执行任务的是背后的 agent。

### Q2：插件文件、插件进程、插件数据有什么区别？

**答：分别是“安装了什么”“此刻在运行什么”“需要长期保存什么”。**

| 对象 | 内容 | 关闭 Ora 后 |
| --- | --- | --- |
| 插件文件 | 插件代码、说明、资源、可能随包携带的 CLI | 保留在安装目录 |
| 插件进程 | 执行 `main.js` 的 Deno 进程 | 由 Ora 关闭和回收 |
| 插件数据 | 配置值及插件保存的数据 | 保留在独立数据目录 |

插件运行状态由内存中的 Lifecycle 管理，`store.json` 保存的是插件配置，不是 `running/stopped` 状态。代码和数据分开，也让更新程序时可以保留数据。

### Q3：所有插件都必须启动一个进程吗？

**答：不是。** 当前 Agent 和 Workbench 需要通用 Deno 插件进程；Skill、Webview、MCP、Hook 不运行这种 `main.js` 进程。

这里说的是“通用插件进程”。MCP 或 Hook 的后续使用可能涉及其他可执行程序。因此静态插件显示 `stopped`，不一定表示不可用。

### Q4：PluginContribution 是什么？为什么不只保留一个 kind 字符串？

**答：它是宿主验证之后得到的“能力类型＋该类型必需的数据”。** 比如 Agent 类型同时携带经过验证的入口信息。

如果使用 `kind = Agent` 加一个可空的入口字段，就可能出现“声称是 Agent，却没有入口”的矛盾状态。枚举把类型和必需数据绑在一起，让后续代码更难构造出这种错误组合。

### Q5：Skill 插件和 Agent 插件是什么关系？

**答：Skill 提供资料，Agent 提供执行能力。** Skill 插件交付指令和相关文件，Agent 插件把真正会工作的 agent 接进 Ora。

Agent 可以声明自己从哪些工作区目录读取 Skill。Ora 的 Effect 系统负责把选中的内容交付到相应资源，并协调更新；最终由 agent 读取和使用。MCP 配置则通过会话建立或恢复时的 `mcpServers` 交给 agent，不是同一套文件投影。

## 二、安装和身份：Q6～Q11

课程：第 3 课（旧 HTML 页面已删除）、第 4 课（旧 HTML 页面已删除）、第 5 课（旧 HTML 页面已删除）、第 22 课（旧 HTML 页面已删除）、第 25 课（旧 HTML 页面已删除）。

### Q6：Manifest 和实际插件代码是什么关系？

**答：`orax.toml` 是插件说明书，代码是执行说明中能力的程序。** Manifest 声明名称、版本、类型等，不能代替程序运行。

看到 `main.js` 也不能直接认定它是一个合法 Agent 插件，还需要检查说明、身份和包结构。当前插件的 namespace 来自安装来源，由宿主确定，不由 Manifest 自报。

### Q7：为什么安装时要分两阶段验证？

**答：一阶段检查说明是否成立，另一阶段检查磁盘上的文件是否符合说明。**

`ora-plugin-manifest` 检查文本语法、字段和组合规则，不访问文件系统。`ora-plugin-manager` 再检查入口、资源、路径和目录版本等宿主事实。

例如 `kind = "agent"` 语法合法，但包里没有 `main.js`：说明解析可以通过，实际包验证必须失败。

### Q8：InstalledPlugin 的“可信”是什么意思？

**答：它表示插件的结构、身份和类型要求已经过宿主验证。** 后面的模块可以直接使用这份结果，不用反复解析 Manifest、检查同一批文件。

“结构可信”不表示插件代码没有恶意，也不表示运行永远成功。安全权限、运行时合同和故障处理仍然需要其他层负责。

### Q9：点击安装后，文件是怎样正式落地的？

**答：先选包，再下载校验，最后验证并提交目录。**

```text
按插件 ID 找到所属市场来源
→ 选择匹配本机的发布包或通用包
→ 下载并校验 SHA-256
→ 解压到临时目录
→ 验证包结构和适用平台
→ 把完整目录移动到正式版本位置
```

临时目录的意义是避免把半个安装包当成完整插件。哈希校验只能说明下载内容与发布信息一致，不代表发布者的代码一定安全。

### Q10：PluginId 和 AgentRef 现在还需要拼接或转换前缀吗？

**答：两者属于不同领域类型，但当前 agent 身份使用完整的插件 ID，包含 namespace。** 例如 `official/ora-space.claude`。

这样来自不同市场、名称相同的两个包仍是不同 agent。不能把旧课中的“AgentRef 只保存短 identifier”当作当前事实，也不应在业务代码里手工补 `official/`。

### Q11：安装文件完成后，为什么还要 scan 和 sync_plugin_agents？

**答：磁盘变化还需要同步到当前正在运行的 Ora。**

`scan` 刷新 Lifecycle 的已安装插件快照；`sync_plugin_agents()` 再把已安装的 Agent 插件集合同步为 Supervisor 集合，开始管理新 agent 的连接。

只完成下载解压，运行中的 agent 列表未必知道它存在。这两步让安装和卸载可以在不重启 Ora 的情况下生效。安装完成返回，也不等于 agent 已经 Ready。

## 三、启动和职责划分：Q12～Q18

课程：第 8 课（旧 HTML 页面已删除）、第 11 课（旧 HTML 页面已删除）、第 13 课（旧 HTML 页面已删除）、第 14 课（旧 HTML 页面已删除）、第 15 课（旧 HTML 页面已删除）、第 16 课（旧 HTML 页面已删除）、第 21 课（旧 HTML 页面已删除）。

### Q12：为什么说 Agent 插件是双进程模型？

**答：运行插件适配器的 Deno 进程，和真正干活的 Agent CLI 进程，是两个对象。**

`main.js` 处理 Ora 的统一合同和具体 agent 差异；Agent CLI 管理自己的模型上下文、模型调用和工具行为。这里的“双进程”不包含 Ora 自己，也不意味着 agent 后续只能有一个子进程。

### Q13：到底是谁启动和停止 Agent CLI？

**答：插件决定策略，Ora Host 执行并管理操作系统进程。**

插件知道运行哪个命令、传什么参数、如何优雅退出，通过 `ora/childprocess/*` 请求 Host 操作。Host 持有真实进程句柄和进程树，负责最终回收。

所以“插件启动 agent”指插件发起和安排启动，不表示插件绕开宿主私自管理一套进程。

### Q14：Lifecycle、Plugin Runtime、Supervisor 分别做什么？

**答：分别管理插件的一生、插件通信机制和 agent 连接的可用性。**

| 模块 | 主要职责 |
| --- | --- |
| Plugin Lifecycle | 统一拥有插件进程，协调启动、停止、扫描、卸载及运行状态 |
| Plugin Runtime | 执行插件入口，处理注册、外层 JSON-RPC 和通信帧 |
| Connection Supervisor | 编排 agent 启动、ACP 初始化、连接状态、重试和路由失效 |
| Session actor | 顺序处理一段对话的消息、取消、权限交互和历史记录 |

Supervisor 通过 Lifecycle 请求使用插件，不再私自启动第二个 Deno 进程。否则设置页可能显示已停止，另一个模块启动的进程却还在工作。

### Q15：connection() 和 ensure_running() 有什么区别？

**答：前者查询当前连接，后者表达“现在确实需要它运行”。**

`connection()` 不会为了查询而启动插件；`ensure_running()` 必要时请求启动，并等待运行结果。两者拿到的连接都属于某次具体启动，不转移进程所有权。

例如只想协调一个已经在线的消费者时，可以查询连接；确实需要建立 agent 服务时，则使用确保运行的路径。

### Q16：插件 Running 为什么不等于 Agent Ready？

**答：Running 表示插件进程已运行并完成注册；Ready 还要求真实 agent 启动和 ACP 初始化成功。**

典型情况是 `main.js` 已经运行，但本机没装需要的 CLI，或者 CLI 启动后握手超时。前一层成功，不能证明后一层已经能服务对话。

### Q17：Ready 前的三次确认分别确认什么？

**答：分别确认适配器合同、启动结果与协议版本、真实 agent 的通信能力。**

| 阶段 | 通信对象 | 确认的问题 |
| --- | --- | --- |
| `ora/register` | 插件 → Ora | 插件实现了哪些方法，能发哪些通知？ |
| `agent/start` | Ora → 插件 | 底层 agent 是否准备好，使用什么协议和版本？ |
| ACP `initialize` | Ora ↔ 真正的 agent | ACP 是否能实际往返，支持哪些可选能力？ |

`session/load`、`session/list` 等能力由 ACP 初始化协商。模型目录查询是独立的按需操作，当前不是发布 Ready 的必要步骤。

### Q18：为什么 ora/register 只能发一次？

**答：同一代插件进程的能力合同必须固定。** 如果并发请求进行时还可以随意增删方法和通知，宿主就无法稳定判断哪些操作合法。

需要改变合同，就启动新一代插件并重新注册。Manifest 描述安装包，`ora/register` 描述这次运行的实际接口，两者不能互相替代。

## 四、协议和多会话：Q19～Q23

课程：第 9 课（旧 HTML 页面已删除）、第 10 课（旧 HTML 页面已删除）、第 17 课（旧 HTML 页面已删除）。

### Q19：Plugin JSON-RPC 和 ACP 为什么是两层？

**答：外层管理插件通信，内层表达 agent 对话业务。** 可以把外层理解成信封，ACP 理解成信里的内容。

`ora/register`、`agent/start`、`agent/stop` 属于插件层；`session/new`、`session/prompt` 等属于 ACP。`agent/acp` 是外层承载一条完整 ACP 消息的通道。

插件桥接层主要转发 payload；到了 ACP 层，Ora 才理解会话、请求响应和权限交互等含义。

### Q20：为什么 agent/acp 使用 notification，而不是外层 request？

**答：ACP 内部已经有请求 ID、响应、取消和完成规则，不需要再套一套。**

一条 prompt 可以持续几分钟。如果外层也等它完成，就会出现两套 ID、两套取消方式和重复超时。外层 notification 负责承载消息，真正的请求完成由内层 ACP 判断。

外层没有 response，不代表内层 ACP 没有 response，也不代表消息可以随便乱序。

### Q21：一条 prompt 从前端到 agent，再返回，经过哪些对象？

**答：前端使用 Ora 契约，后端负责转换和路由，插件负责接到 CLI。**

```text
前端发送消息
→ Session actor / ACP 层创建 session/prompt
→ Plugin transport 包装为 agent/acp notification
→ Plugin SDK 分发给 main.js 的 onAcp
→ 插件通过 Host 提供的代理流写入 Agent CLI stdin

Agent CLI 输出 ACP 消息
→ 插件读取代理 stdout，调用 send(frame)
→ agent/acp notification 回到 Ora
→ ACP 层关联请求、解析事件
→ 路由到对应 Session actor
→ 记录历史并更新前端
```

前端不用理解插件进程和 CLI stdio。代理流背后的真实进程和管道仍由 Host 管理。

### Q22：一个 agent 连接服务多个对话，为什么不会串线？

**答：共用连接，但每段会话有独立地址和事件队列。**

Agent 的 `sessionId` 决定消息属于哪个会话，路由表把它对应到一个 SessionChannel。只有请求 ID 的最终响应，则先通过待处理请求表找回会话，再投递。

因此连接像公共大门，会话 ID 像房间号。每个 Session actor 只处理自己的消息，同一段对话同时只允许一个 prompt 执行，不同对话可以并行。

### Q23：Ora Session ID、agent Session ID、request ID 有什么区别？

**答：分别标识用户的一段对话、agent 内部的一段上下文、一次协议请求。**

| ID | 主要用途 | 切换 agent 时 |
| --- | --- | --- |
| Ora Session ID | 数据库、历史、任务归属、前端定位 | 保持不变 |
| agent Session ID | agent 内部会话地址、事件路由 | 换成新 agent 返回的 ID |
| request ID | 匹配一次请求及其响应 | 新请求使用自己的 ID |

不能用 request ID 当长期会话身份，也不能要求不同 agent 理解同一个内部会话 ID。

## 五、模型选择与插件作者：Q24～Q27

课程：第 24 课（旧 HTML 页面已删除）、第 26 课（旧 HTML 页面已删除）。

### Q24：为什么有模型目录和 Session configOptions 两种结果？

**答：一个回答“开始前可以选什么”，另一个回答“这段会话现在实际支持和使用什么”。**

`agent/list_models` 不需要先创建 Session，按需查询并携带工作区 `cwd`。`session/new` 或 `session/load` 返回的 `configOptions` 属于具体会话，包括当前选择和可用选项。

前者像事先查看菜单，后者像实际下单时的确认。会话建立后，应以该会话返回的配置为准。

### Q25：选好模型后，Ora 会强制使用它吗？

**答：会话建立前的选择是意图，必须用新会话报告的配置核实。** 只有会话确实提供该模型选项，Ora 才尝试应用；不支持或设置被拒绝时，以 agent 实际返回的配置为准。

当前打开空聊天页不会预先创建 Warm Session。第一条消息触发 `startSession`，建立 agent 会话、应用模型意图并保存 Ora 会话，再正式发 prompt。已有会话则通过 `setSessionConfig` 修改配置。

### Q26：defineAgent 替插件作者完成什么？

**答：它把业务回调装配成标准的 Agent 插件合同。** 作者提供 `start`、`stop`、`listModels` 和 `onAcp`，SDK 负责接口注册、ACP 通道接线和部分参数、结果结构处理。

时间顺序要分清：调用 `defineAgent(...)` 只在本地装配合同；调用 `plugin.run()` 才发送 `ora/register`；之后收到 `agent/start` 才真正启动 CLI。

SDK 不会替作者决定具体 CLI 的命令、退出方式或输出格式。

### Q27：插件作者怎样完成最小的双向通信闭环？

**答：接好进程生命周期，再接好 ACP 的两个方向。**

| 回调或能力 | 作者负责的内容 |
| --- | --- |
| `start(context, send)` | 选择 CLI，通过 Host 启动，读取输出并调用 `send(frame)` |
| `onAcp(frame)` | 把 Ora 发来的帧写入 CLI 输入 |
| `stop()` | 执行专属退出策略，必要时请求 Host 终止，并清理本地状态 |
| `listModels({ cwd })` | 查询无需创建对话就能展示的模型目录 |

程序来源也有边界：包内没有携带 CLI 时可以尝试本机 PATH；包里有但损坏或不可运行时，应报告包错误，不能悄悄换用本机另一个版本。

## 六、重启、故障和退出：Q28～Q34

课程：第 12 课（旧 HTML 页面已删除）、第 18 课（旧 HTML 页面已删除）、第 20 课（旧 HTML 页面已删除）、第 23 课（旧 HTML 页面已删除）、第 25 课（旧 HTML 页面已删除）。

### Q28：插件 ID 没变，重启后为什么旧连接不能继续使用？

**答：插件身份相同，不代表仍是同一次运行。** 每次启动尝试都有自己的 Plugin generation，连接、注册信息和通知都属于那一代。

旧连接不能自动跳到新进程，旧消息也不能混进新连接。generation 就像房卡批次：房间号没变，上一批房卡也不能继续开门。

### Q29：为什么不同地方的 generation 不能直接比较？

**答：它们描述不同对象的版本，不是一个全局计数器。**

Plugin generation 描述插件启动代次；connection generation 描述成功建立的 agent 连接代次；Effect 协调中的版本和身份用来确认消费者看到的是正确的资源状态。

一次插件启动失败，可能已产生新的 Plugin generation，却没有建立新的 Ready connection。更新 Skill 也不必然重启插件。旧课中具体 Effect 接口已有变化，不应把它的参数当作进程编号使用。

### Q30：为什么启动失败后不在原 main.js 上无限重试 agent/start？

**答：失败可能已经产生了一部分效果，宿主无法证明原状态干净。** 比如 CLI 已经启动，只是成功响应丢失。

直接再启动可能留下两个 CLI、旧监听器和旧管道。Ora 以整个运行代次为清理边界，回收旧状态后重新建立连接，减少对第三方插件复杂恢复逻辑的依赖。

### Q31：哪些错误可以重试，哪些应该停止？

**答：看再次尝试是否可能得到不同结果。**

| 情况 | 处理思路 |
| --- | --- |
| 本机 CLI 尚未安装 | 作为预期缺失，退避重试，不计入崩溃次数 |
| 普通启动失败、ACP 初始化超时、连接断开 | 清理旧代次，计入故障窗口并重试 |
| 合同缺失、协议版本不支持、随包程序不可用 | 确定性问题，进入 Failing 并停止自动重试 |

当前普通故障在一分钟内超过三次会触发熔断，本次 Ora 进程中停止该 agent 的自动重试。模型目录查询现在独立于连接启动，查询失败不应再按旧课说成共享连接启动失败。

### Q32：一个会话或 agent 出问题，会影响多少范围？

**答：尽量只影响真正依赖故障对象的部分。**

单个会话取消、超时或队列溢出，通常只结束该会话的操作。共享 agent 连接断开，会影响挂在这条连接上的会话。其他 agent 有各自独立的 Supervisor，可以继续工作。

恢复连接不会自动重发上一条 prompt，因为那条请求可能已经修改文件或执行工具，重复执行会造成新的副作用。

### Q33：为什么发出 agent/stop 后，还要继续清理进程树？

**答：请插件停止，只是给它优雅收尾的机会，不能保证它一定成功。**

```text
结束旧连接并使相关路由失效
→ agent/stop：给适配器机会停止 CLI，最多等待 2 秒
→ Lifecycle 停止插件，发送 ora/shutdown
→ Host 必要时终止仍存活的受管进程
→ 等待退出、回收进程记录
→ 才允许替代代次启动
```

`kill` 是请求终止仍在运行的进程，`wait/reap` 是确认退出并回收。已经正常退出的进程不需要再强杀，但仍需要确认和收尾。

### Q34：停止、更新、卸载分别意味着什么？

**答：停止改变运行状态，更新改变代码版本，卸载移除安装身份。**

| 操作 | 结果 |
| --- | --- |
| stop | 结束当前插件进程，包仍安装；Supervisor 可能再次请求启动 |
| update | 停止旧进程，安装验证后的新版本，重新扫描并建立连接 |
| uninstall | 关闭相关界面，停止受管进程，移除包；数据按选择保留或删除 |

当前 stop 不是持久禁用。版本目录也不表示已有一键回滚：更新成功后会清理其他旧版本目录。卸载 agent 插件不等于删除 Ora 对话。

## 七、安全和排查：Q35～Q36

课程：第 18 课（旧 HTML 页面已删除）、第 19 课（旧 HTML 页面已删除）。

### Q35：经过校验和统一进程管理，就能称为严格沙箱吗？

**答：不能。** 校验说明文件、限制协议合同、清理进程树，各自解决不同的问题，不等于限制住插件能做的一切。

当前 Agent 插件获得 Deno 的运行程序、读取文件、读取环境变量和联网权限，权限大致接近宿主。没有直接授予 `--allow-write` 也不能证明无法写文件，因为它能启动子进程。

Host child-process API 的主要价值是统一管理和可靠回收受管进程；它不能单独证明第三方代码已经受到最小权限隔离。

### Q36：用户说“装好了但不能聊天”，应该按什么顺序排查？

**答：沿着从安装到对话的链条，找到最后一个已经成功的阶段。**

1. 包是否通过验证，出现在已安装快照中？
2. Agent Supervisor 是否已通过同步被创建？
3. Deno 插件是否启动并注册了完整合同？
4. `agent/start` 是否成功，实际 CLI 是否可运行？
5. ACP `initialize` 是否成功，连接是否 Ready？
6. `session/new` 或恢复过程是否成功，模型和 MCP 是否可应用？
7. 用户消息是否成功记录并发送，返回消息是否进入正确会话？

例如 Running 但不是 Ready，优先看合同、CLI 和 ACP 握手；Ready 但某个会话不能发送，继续看该会话建立、配置和历史写入，而不是把全部问题都归为安装失败。

## 八、把插件与切换、历史连起来：Q37～Q40

补充依据：[Agent Runtime](../../../docs/agent-runtime.md)、[ora-history](../../../crates/history/README.md)。

### Q37：切换 agent 时，是不是另开一段 Ora 对话？

**答：不是。** Ora 对话 ID、任务、工作目录和历史保留，改变的是它绑定的 agent 和 agent 内部会话。

界面选择先保留在本地，发送下一条消息时才提交切换。Ora 先建立新 agent 会话，成功后才释放旧绑定。切回原 agent 也使用新会话接收最新历史，不直接复用已经缺少中间对话的旧上下文。

### Q38：新 agent 怎样理解之前发生过什么？

**答：Ora 把自己的历史整理成交接文本，放到新 agent 的下一条 prompt 前面。**

交接保留用户消息和 agent 回复，工具调用压缩为标题和结果概况，不传旧 agent 的完整思考过程、工具输入输出和计划。它是在阅读一份历史记录，不是直接继承另一个 agent 的内部记忆。

Ora 会记录切换和交接投递状态。只有请求被发送通道接受后，才确认这一轮交接已投递，避免发送前失败却误以为新 agent 已经拿到历史。当前交接文本没有自动长度预算，长对话仍可能超出接收模型容量。

### Q39：对话历史是谁保存的，打开旧对话需要启动插件吗？

**答：Ora 自己保存，一段对话对应一个追加写入的 JSONL 历史文件。** 数据库保存会话信息，文件保存消息、工具过程、回合结束原因及切换等记录。

打开旧对话读取 Ora 的记录，不需要 agent 启动，也不依赖插件重新讲一遍历史。继续发送时才需要可用的 agent 会话。插件卸载后仍能看历史，但不能继续向已卸载的 agent 发消息。

记录会刷新写入但不逐条强制同步磁盘，因此不能承诺断电时最后内容绝不丢失。

### Q40：历史写入失败，为什么要限制继续发送？

**答：否则对话已经继续前进，Ora 保存的记录却悄悄停在过去，用户和接手 agent 都可能误以为历史完整。**

如果用户消息本身尚未成功记录，就在调用 agent 前拒绝发送。已经开始输出的回合可以继续，但会话进入历史降级状态，后续发送和切换被阻止，直到显式恢复记录。

恢复会写入缺口标记，说明曾有内容没有保存；它不会凭空补回丢失内容。

## 九、旧课与当前实现的差异

旧 HTML 保留了学习时的实现背景。遇到下面这些说法，复习时按右侧理解，不要混用两个版本。

| 旧课中的说法或写法 | 当前应采用的理解 | 核对依据 |
| --- | --- | --- |
| Manifest 自己声明 namespace | namespace 来自市场来源或本地导入规则，由宿主确定 | [Manifest](../../../crates/plugin-manifest/README.md) |
| AgentRef 是短 identifier，与 PluginId 需要映射 | 当前 agent 身份就是包含 namespace 的完整插件 ID | [连接管理](../../../crates/backend/src/agent_runtime/connection.rs) |
| `agent/listModels`、`ora/childprocess/closeStdin` 是线上方法名 | 当前线上名称为 `agent/list_models`、`ora/childprocess/close_stdin`；SDK 回调仍可使用 camelCase | [Agent 协议常量](../../../crates/plugin-protocol/src/agent.rs)、[子进程协议常量](../../../crates/plugin-protocol/src/child_process.rs) |
| 模型目录在每代连接启动时查询并缓存，是 Ready 前提 | 现在带工作区 cwd 按需查询；查询失败不破坏共享连接 | [控制接口](../../../crates/backend/src/agent_runtime/plugin_agent/control.rs)、[运行说明](../../../docs/agent-runtime.md) |
| 打开聊天先创建 Warm Session 发现模型 | 当前空聊天不预建会话，首次发送走 startSession | [运行说明](../../../docs/agent-runtime.md) |
| `effectSurfaces`、`effect/restart` 及旧 Workspace generation 模型 | 当前使用 `effectResources` 和 coordinate / reactivate / verify_ready，按资源与消费者的具体身份和版本核验 | [Agent adapter](../../../crates/backend/src/agent_runtime/plugin_agent/README.md)、[SDK](../../../packages/plugin-sdk/src/agent.ts) |
| 被禁用的插件会被 Supervisor 持续重试 | 当前没有正式的持久 disable 状态；stop 不能当成永久禁用 | [Lifecycle](../../../crates/plugin-lifecycle/README.md)、[插件操作](../../../crates/backend/src/plugin/operations.rs) |

## 十、最后用一分钟口述整条主线

> Ora 先下载并验证插件包，把包整理成已验证的 InstalledPlugin；然后刷新安装快照，同步 agent 的 Supervisor。Supervisor 通过 Lifecycle 启动 Deno 插件，插件注册能力后，收到 agent/start，再通过 Host 启动真正的 Agent CLI。Ora 与 CLI 完成 ACP 初始化后，连接进入 Ready。用户发送消息时，Ora 创建或恢复独立的 agent 会话，通过 agent/acp 通道发送 ACP 消息，并按会话 ID 把回复分发给对应 actor。对话历史由 Ora 保存，所以可以脱离某个 agent 长期存在，也能交给另一个 agent。发生故障时，Supervisor 按类型决定重试，Lifecycle 和 Host 负责回收旧进程，避免旧连接和旧执行者干扰新的一代。

自测时，再尝试只回答这五个追问：

1. 安装成功、插件 Running、agent Ready，分别证明了什么？
2. 为什么插件决定启动方式，Host 却持有真实进程？
3. 为什么 ACP 要套在 notification 里？
4. 同一个插件重启后，哪些身份不变，哪些运行状态需要重建？
5. 如果没有 Ora 自己保存的历史，切换 agent 会遇到什么问题？
