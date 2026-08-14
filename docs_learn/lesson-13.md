# 第 13 课：Spec 管理与租约（ProjectWorkContext）（总结）

> 对应对话内容：PWC 租约移除的分支差异、Spec 管理是什么（白话版）、有界发现、安全约束、scheduler 实际用途。
> 代码地图（实际分支 project_learn）：`crates/application/src/spec/`、`docs/spec-management.md`、`crates/scheduler/`、`crates/db/src/migration/schema_v0011.rs`（租约移除证据）。
> 一句话：**Spec 管理 = 把项目里已有的规格 md 文档（OpenSpec/Superpowers/自定义目录）索引成目录树，在右面板只读预览**——只管"发现+展示"，不管"创建+修改"。

## ⚠️ 重要发现：PWC 租约在当前分支已被移除

LESSON-PLAN 原写的第 13 课含"窗口租约机制（120s 过期、窗口独占、Web 占座 vs Desktop 不支持）"——但当前分支已删除：

```
crates/db/src/migration/schema_v0011.rs（tail migration）：
  DROP INDEX idx_project_work_contexts_*;
  DROP TABLE IF EXISTS project_work_contexts;    ← 整个表删掉

crates/application/src/：无 project_work_context 模块
crates/domain/src/：无 project_work_context.rs
后端：无 lease 逻辑
scheduler：只被 title acquisition 当定时器用，无租约清理任务
```

**结论**：PWC 租约是**旧设计，已删除**（tail migration 让新旧数据库都收敛到"无此功能"的 schema）。**LESSON-PLAN 滞后于代码**（第 6 课 PR #169、第 7 课分支差异后第三次）。**以当前分支代码为准。**

## 一、Spec 管理是什么（白话版）

- **背景**：项目里通常有一堆"规格文档"（OpenSpec/Superpowers 或团队约定的 md），描述"这个项目该怎么做"——**不是 Ora 生成的**；
- **Ora 做了什么**：① 发现（去约定目录找 md）→ ② 索引（生成有效文档清单 catalog）→ ③ 展示（右面板目录树 + 只读预览）；
- **不做什么**：不创建、不编辑、不删除这些文档（属于用户自己的文件系统工具）；跟 workflow 对话状态无关。

## 二、Targets 和配置

- **SpecTarget**：`project` 或 `task`（tagged）；task 解析用**和 agent session 相同的 cwd**（linked worktrees + project-root tasks，第 6 课）；
- **源覆盖按项目持久化**（`project_spec_source_overrides` 表）→ 主 checkout 和每个 worktree 一致；替换原子；项目级联软删。

### 默认候选目录

```
OpenSpec：    openspec/specs, openspec/changes
Superpowers： docs/superpowers/specs, docs/superpowers/plans, docs/plans
Custom：      specs, docs/specs
```

### 有界发现规则

- 只在受控 `spec`/`specs` 目录 + workflow 的 `changes`/`plans` 目录找 Markdown/MDX；
- **遵守 Git ignore** + 排除生成目录；显式启用的源单独枚举（ignore 关闭）；
- 精确重复路径用宿主文件系统大小写语义合并；**重叠文档属于最深的启用源**（同 skill 的"最近归属"思路）。

## 三、API 和安全

```
spec 客户端命名空间：catalog / read / source resolution / project-source replacement / watch
```

- **catalog 和 read 从不暴露绝对根**；
- 文件操作 **canonicalize** 目标和选中路径；
- 只接受 `.md`/`.mdx` **且仍在当前有效 catalog 中**——防遍历、symlink 逃逸、过期源授权；
- 发现用注入的 ripgrep：**15s / 8MiB / 10000 结果限制** + 截断上报（第 14 课再见面）。

## 四、前端行为（简述）

- `WorkspaceReviewLayout`：900px 可调右面板；project context 只有 **Files**；task context 有 **Changes + Files**；
- Files 面板三个子视图：**Specs / Explorer / Search**（project 默认开 Specs，task 默认开 Explorer）；
- Specs 子视图：左只读内容 + 右分组源树；无自动选择（空视图直到点选）；
- **安全**：原始 HTML / MDX JSX **不执行**、本地图片**阻止**、只有 catalog 成员相对 Markdown 链接可导航；
- **项目根不能注册为源**（必须包含子目录）——防无关仓库 Markdown 被重新归类为规格；
- 文档只读；编辑/删除属于用户正常文件系统工具。

## 五、scheduler（实际用途）

`ora-scheduler`：进程内调度服务（cron jobs + setTimeout 风格延迟）。

```
Job { stable name, cron expression, async work }   // cron 任务（当前无使用者）
Scheduler::schedule_after(delay, future)           // 一次性延迟（当前唯一用途）
```

**当前唯一用途**：title acquisition / title polling 的定时器（60s 手柄、10s/3s 重试节奏，第 8 课）。

**特性**（README）：迭代**永不重叠**（tick 顺序 await）；错过的 tick **跳过不重试**（`Schedule::after` 严格 now 之后）；`DelayHandle` drop = 取消（除非 detach）；`shutdown` 从任何 clone 发起、abort 所有任务（含 detached）。

## 六、术语表新增

Spec 管理（规格索引）、SpecTarget（project/task）、catalog（有效文档清单）、源（source，被索引目录）、OpenSpec / Superpowers（规格目录约定）、有界发现（bounded discovery）、canonicalize（路径规范化）、ripgrep（搜索工具）、只读审查界面（read-only review surface）、WorkspaceReviewLayout（右面板）、Scheduler / Job / CronHandle / DelayHandle（调度器家族）、tail migration（尾部迁移）。详见桌面 software technical terms.md。

## 七、下一课预告

> 第 14 课：task_diff 与文件系统层——diff 视图的数据哪来的？`ora-fs` 提供什么？工作区文件浏览怎么限制？（ripgrep 注入、15s/8MiB/10000 限额会在这里再次出现）
