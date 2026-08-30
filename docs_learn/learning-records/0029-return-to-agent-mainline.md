# 回归 Agent 插件主线

## Learner request

用户发现围绕 Claude Code Skill 热加载和 Effect coordination 的讨论再次偏离课程主线，要求立即回归。

## Mainline decision

- 将 `uninterrupted`、热加载、首次创建 Skill 目录和 coordination 枚举改进归入开发扩展旁支，不作为当前面试主线的继续条件。
- 第 15 课只保留核心结论：`connection()` 查询当前 live generation 且不会启动；`ensure_running()` 表达真实使用需求，必要时经 Lifecycle 启动并返回 generation-bound lease。
- 下一步先进行 Agent 插件端到端口述验收，把安装、注册、双进程、两层协议、Lifecycle、Supervisor、generation 和 Session actor 串成一条稳定主线；通过后再学习 Skill、MCP、Webview 插件。

## Mastery status

第 15 课核心已掌握。Effect 热加载细节不纳入当前主线掌握要求。
