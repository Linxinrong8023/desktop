# 学习笔记（docs_learn）

> 本文件夹用于存放学习 Ora Desktop 项目过程中的**个人理解笔记**，不是项目正式文档。
> 内容使用中文，解释风格以通俗易懂为主。
>
> **重要约束**：本文件夹只允许在 `project_learn` 分支上进行修改。

## 目录

- [第 1 课：认识仓库、程序与依赖地图（总结）](./lesson-01.md)
- [第 2 课：同一个"项目"的三种样子 + 数据链路（总结）](./lesson-02.md)
- [第 3 课：接口、handler、组合根与完整流程（总结）](./lesson-03.md)
- [第 4 课：错误体系 + 请求生命周期 + Backend 生命周期（总结）](./lesson-04.md)
- [第 5 课：数据库持久化——迁移、连接池、repository（总结）](./lesson-05.md)
- [第 6 课：Task 与 Git Worktree（gitlancer 与任务工作树）（总结）](./lesson-06.md)
- [第 7 课：ACP Agent Runtime（完整：三讲全部完成）](./lesson-07.md)
- [第 8 课：Session 生命周期与 Warm Session 深入（总结）](./lesson-08.md)
- [第 9 课：⭐ 保存上下文信息——ora-history 会话历史与 transcript（总结）](./lesson-09.md)
- [第 10 课：⭐ 模型选择与切换（Model Selector）（总结）](./lesson-10.md)
- [第 11 课：⭐ 切换 Agent（会话换绑）（总结）](./lesson-11.md)
- [第 12 课：Skill 体系与 AgentDefinition（简单总结）](./lesson-12.md)
- [第 13 课：Spec 管理与租约（ProjectWorkContext）（总结）](./lesson-13.md)
- [第 14 课：task_diff 与文件系统层（总结）](./lesson-14.md)
- [第 15 课：Workflow 定义与版本管理（总结）](./lesson-15.md)
- [第 16 课：Workflow 运行引擎（总结）](./lesson-16.md)
- [第 17 课：Workflow 前端设计模式 + 模块全貌（总结）](./lesson-17.md)

> 上述“已学”表示有历史课程记录，不等于已经掌握。插件专题根据用户最新反馈从零基础重新开始，并以主动复述作为掌握证据。

## 学习路线图

- [第 6 课起的章节规划（含四个关键专题）](./LESSON-PLAN.md)
- [Agent 教学引导（如何给用户讲解）](./AGENT-TEACHING-GUIDE.md)
- 已学完第 1~17 课（第二段 Workflow 专线全部完成）；第 18~19 课偏前端（可不学/粗读），第 20 课 Web 服务器运行时、第 21 课桌面运行时（Tauri）值得细学。

## 插件系统专题（Teach 工作区）

> 当前最高优先级：为面试建立可口述、可追问、可落到源码的完整插件系统理解；开发扩展能力为第二优先级。

- [学习使命](./MISSION.md)
- [可信资料索引](./RESOURCES.md)
- [插件系统速查图](./reference/plugin-system-map.html)
- [专题第 1 课：插件到底是什么](./lessons/0001-package-contribution-runtime.html)
- [专题第 2 课：插件文件、插件进程与插件数据](./lessons/0002-files-processes-and-data.html)
- [专题第 3 课：Manifest 是插件的身份证](./lessons/0003-manifest.html)
- [专题第 4 课：Manifest 与 Plugin Manager 的两阶段验证](./lessons/0004-two-stage-validation.html)
- [专题第 5 课：InstalledPlugin 是验证后的可信宿主视图](./lessons/0005-installed-plugin.html)
- [专题第 6 课：PluginContribution 与类型化能力](./lessons/0006-plugin-contribution.html)
- [专题第 7 课：Contribution 不等于 Runtime](./lessons/0007-contribution-versus-runtime.html)
- [专题第 8 课：Agent 插件的双进程模型](./lessons/0008-agent-two-processes.html)
- [专题第 9 课：Plugin JSON-RPC 与 ACP 的协议分层](./lessons/0009-plugin-rpc-and-acp.html)
- [专题第 10 课：一条 Prompt 的端到端协议路径](./lessons/0010-one-prompt-end-to-end.html)
- [专题第 11 课：ora/register 与运行时能力合同](./lessons/0011-runtime-registration.html)
- [专题第 12 课：Plugin ID 与 Process Generation](./lessons/0012-plugin-generation.html)
- [专题第 13 课：Plugin Lifecycle 是唯一进程所有者](./lessons/0013-lifecycle-sole-owner.html)
- [专题第 14 课：插件 Running 不等于 Agent Ready](./lessons/0014-running-versus-ready.html)
- [Agent 插件主线阶段总结](./reference/agent-plugin-mainline.html)
- [专题第 15 课：connection 与 ensure_running](./lessons/0015-connection-versus-ensure-running.html)
- [专题第 16 课：为什么 Agent 需要 Runtime 与 Supervisor](./lessons/0016-runtime-and-supervisor.html)
- [专题第 17 课：一个 Agent 如何准确服务多个 Session](./lessons/0017-shared-agent-session-routing.html)

当前进度：Agent 插件从安装到 Prompt 渲染的全链路口述验收已通过；正在学习共享 Agent connection 如何通过 Agent Session ID、AcpPeer correlation 与 RouteRegistry 隔离多个 Session。
