# minicloud 本机运行时

[English](runtime.md) | 中文

## 一键开发环境

在仓库根运行 `task run:minicloud`。脚本安装前端依赖、构建 debug 程序，然后依次启动
host、`ora-controller --single-node`（由它自己启动 Node）和 Vite，打开 `http://127.0.0.1:5174`。
需要 Linux、Deno、Cargo、Node.js、Git 和 `setsid`，不需要 root。

所有开发配置及运行数据都在 `~/.ora/minicloud/<digest>/`，其中 `<digest>` 由 checkout 路径推导，
`workspace` 文件记录该路径。因此每个 checkout 各有独立状态；被其他 checkout 占用的目录会被拒绝。
启动器不向仓库内写入任何内容：

- `config/node.json`、`controller.json`、`client.json`：Node 与 Controller 部署配置（Controller 文件的
  `single_node` 段指向 `node.json`），以及 Vite 端口和启动器经命令行传给 Controller 的回环端口
  `controllerPort`（默认 4820）。已移除的 minicloud server 留下的旧 `server.json` 会被忽略。
- `config/clone.gitconfig`：非交互 Git 配置；默认只适用于无需凭据的 HTTPS 仓库，私有仓库凭据需自行配置。
- `node/`、`controller/`：数据库与 Node IPC；`p/`：host／guardian 状态（短名称为 Unix socket 长度留空间）。
- `repositories/`：clone 目录；`home/`：工作负载 HOME；`bin/`：可识别归属的版本化 guardian；`vite/`：Vite 缓存。

重复运行保留配置、数据库及 checkout，不覆盖用户编辑，不在恢复失败时清库重建。日志输出到终端；
依赖和编译产物仍使用仓库标准 `node_modules`／`target`，不复制到数据目录。
仅初始化可运行 `deno run -A scripts/run-minicloud.ts --init-only`；已构建时可用 `--no-build` 跳过安装和编译。
修改端口后重新运行；由脚本拥有的数据目录路径不能改到其他部署。

Ctrl+C 或组件异常退出会先停止 Vite，再停止 Controller——由它停止自己启动的 Node 并等待受管 Git 收尾，
随后停止 host 和本目录专用 guardian。Node 与 Controller 同属一个进程组，因此即使 Controller 崩溃，
启动器按进程组停止时仍能到达 Node。停止超时会报告并升级信号；不保证终止逃逸的工作负载后代。中断的 clone 可能保持待恢复／未知，
不会自动新建执行或删除残留。脚本使用独占锁，拒绝同一数据目录的第二个启动器。

debug 构建跳过受信路径的 Unix 权限位检查，允许组可写的项目目录，不修改已有目录权限。
所有者、符号链接、硬链接、类型、目录隔离及数据库独占校验仍生效；release 构建仍强制权限位检查。
状态目录以家目录而非 checkout 为基准，因为 Unix socket 路径上限为 108 字节；异常长的真实家目录路径会被拒绝，
不会被截短，也不能用符号链接绕过。

## 手动部署

API 由 `ora-controller` 可执行程序本身提供，没有独立的 minicloud server。先启动 host／guardian，
再按既有[部署配置](../node/repository-clone.zh.md)启动 Node，或用 `--single-node` 让 Controller 托管它。
然后运行 `ora-controller --config /absolute/path/controller.json --transport tcp --host 127.0.0.1
--port 4820 [--single-node]`；配置文件、参数与托管规则见
[Controller 运行时](../controller/local-runtime.zh.md#独立可执行入口)。

Node 配置的归属必须匹配 ControllerId。启动 minicloud 前，停止使用同一状态目录的独立 Controller。
各状态目录继续显式注入并拒绝重叠；非回环监听或未配置的目标 Node 会在打开 Controller 状态前拒绝。
这是非生产应用，不增加认证或额外安全体系。

## HTTP 接口

先运行 `deno install`，再从仓库根执行 `deno task --filter @ora/minicloud-client dev`，
打开 `http://127.0.0.1:5174`。Vite 将 `/api` 代理到 `http://127.0.0.1:4820`；
不同 Controller 端口可通过 `MINICLOUD_SERVER_URL` 配置。
页面使用共享 shadcn 组件及 React 19，轮询 Controller 操作。
提交前把未确认请求写入当前标签页 session storage；回复丢失或刷新后，“重试原请求”复用原身份与输入。
这不是操作历史数据库。关闭页面终止 HTTP 和轮询，不取消 Node 执行。

- `POST /api/clones`：`{ "requestId": "stable-client-id", "repository": "https://host/repo.git", "branch": "main" }`。
  持久接受后返回 HTTP 202，包含 `requestId`、`operationId`、`executionId`。
- `GET /api/clones`：按接受顺序倒序列出操作，Node 离线时仍能读取待协调记录。
- `GET /api/clones/{executionId}`：读取单个操作；404 表示没有该身份的接受记录。

状态为 `pending`、`succeeded`（Node 路径及 commit）、`failed`（明确原因及残留路径）。
pending 不声明 Git 是否正在运行。HTTP 400 表示输入无效，409 表示身份／输入冲突，503 表示暂时不可用；
HTTP 错误不是 clone 终态。浏览器 DTO 从 `ora-contracts::controller_api` 生成，与 Node 消息协议分开；
这套 JSON 接口是过渡契约，浏览器改经 Ora Cloud 接入后退役。

未确认接受结果时使用相同请求身份和输入重传。关闭浏览器或停止 Controller 不取消 Node 执行；
重启使用原状态目录恢复事实。不另建 minicloud 任务数据库、不提供清理命令或自动新执行重试。

真实 HTTP 测试（`cargo test -p ora-controller`）覆盖接受、冲突、无效输入、离线列表、不存在的执行身份、
独占、组合参数拒绝、Unix socket 传输及正常重启。
下层 Controller 强杀测试与 minicloud 自身端到端证据分别记录。

前端检查：`deno task --filter @ora/minicloud-client lint`、`test`、`build`。
`task test:minicloud` 另验证真实 HTTP 及 Vite proxy → 独立 `ora-controller` → Node → HTTPS Git，
包含 Controller SIGKILL／重启和唯一变更 Run；要求 Linux 且已安装前端依赖。
Vite 用例在纯 Rust CI 中显式跳过，通过专项任务执行。
真实链路还通过 SQLite writer lock 注入接受写失败，验证 HTTP 503 且没有接受记录，释放后仍只有一个 Run。
真实 TCP 代理截断接受响应 body，Controller 重启后重传仍保留原执行；clone 期间正常关闭 Controller，
固定身份的 Git 进程仍存活，最终返回同一结果且只有一个 Run。入口测试验证重叠目录拒绝、
未知文件保留，并复用独立 Controller 所有者接受的记录。`--single-node` 组合另有专项验证：强杀 Controller
后其 Node 与 Git 继续运行，endpoint 仍活跃时第二个托管 Controller 被拒绝，替换的 Controller 接管重放的
结果，正常停止时托管 Node 被收回。
DOM 测试验证页面行为、轮询自动恢复及刷新身份恢复；完整浏览器引擎交互验收明确不在本次范围内。
