# minicloud 方向

[English](overview.md) | 中文

minicloud 是承载 Ora clone 最小闭环及短期需求的非生产本机应用。
它替代 Desktop 成为首个用户入口，不替代已有 Controller 和 Node 职责。
Linux client／server clone 闭环已实现，启动方式见[运行 minicloud](runtime.zh.md)。
[根 ADR](../../specs/decisions/minicloud/runtime/0-local-web-client-embedded-controller.md) 已 implemented，
[核心验证义务](../../specs/test-cases/minicloud/runtime/local-web-clone-loop.md)已在约定范围内覆盖；完整浏览器引擎验收不在本次范围内。

## 组合

- `apps/minicloud/client`：Vite、React 19、shadcn；提交仓库 URL 和分支，观察结果。
- `apps/minicloud/server`：内嵌 Controller 的薄 Rust HTTP 入口，复用持久接受与恢复。
- Node 保持独立，使用已有本机 IPC 及 host／guardian 管理的 Git 执行。

两个开发监听入口均只供本机访问，浏览器通过 Vite proxy 调用 server。
不增加认证、令牌、租户、额外安全体系或生产部署。
回环监听是部署约束，不承诺隔离恶意本机调用方。

Controller 仍拥有操作记录；server 不另建任务数据库、不执行 Git。
Controller 身份、状态目录和 Node endpoint 显式注入；不能另启 Controller 同时使用该状态目录。
既有部署隔离、文件保护及 Git 凭据配置保持不变。

## 首条闭环

持久接受 clone 意图后返回稳定身份，再轮询操作记录及结果。
成功展示 Node 路径与 commit，失败展示报告的原因；暂时无法查询或未知不等于失败。
页面刷新或 server 重启后读取原记录，不重新提交新执行。
不要求 WebSocket、Desktop 集成或 Backend 写入入口切换。

已实现基础和剩余证据见 [Node 最小闭环状态](../node/minimal-loop.zh.md)。
