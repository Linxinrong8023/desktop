# 第 14 课：task_diff 与文件系统层（总结）

> 对应对话内容：Task Diff（变更审查）、Workspace Files（只读文件浏览）、ora-fs 的 5 个能力详解、逃逸/文件监听白话解释。
> 代码地图：`crates/application/src/task_diff/`、`crates/backend/src/task_diff.rs`、`crates/fs/`（path/workspace/search/watch/error）、`docs/task-workspace-files.md`。
> 一句话：**这一课是两个功能——任务 Git 变更的 diff 审查 + 任务工作区的只读文件浏览；两者共用 ora-fs 这个"安全读文件层"。**

## 〇、用户视角

- **Task Diff**：任务详情里看"这轮 agent 改了什么"（Git 统一 diff）+ 在 diff 上评论（像 GitHub PR review）；
- **Workspace Files**：任务工作区文件树 → 只读查看（≤10MiB）→ 搜索 → 行选择发 chat 上下文 → 文件变化自动刷新（watcher）；Specs 子视图（第 13 课）住在这个面板里。

## 一、Task Diff（diff 审查）

### 职责

```
读 task 的 Git 统一 diff → 计算稳定 diff_id（base revision + 当前 HEAD + 完整 patch → hash）
在 diff 上建评论（root discussion + replies + 解决状态）
stage / commit / push（先验证 task/worktree/branch 身份）
```

### 端口（静态分发，测试用内存 fake）

```rust
TaskDiffReader             // 读 diff（GitTaskDiffReader：gitlancer 薄翻译）
TaskGitWriter              // git 操作
TaskDiffCommentRepository  // 评论存储（SQLite）
```

### 评论状态设计（非法状态不可表示）

- root discussion 拥有 anchor + thread 状态；reply 只拥有 parent comment id；
- **评论只能建在"仍匹配当前 patch"的地方**：diff_id/path/hunk/行范围/side/首行内容都对得上才允许——agent 改了一轮后旧评论位置对不上 → 拒绝（stale），防"评论挂在消失的行上"。

### 边界

- 超大 patch → `task_diff_too_large`（丢弃字节数不进公开契约）；
- 评论路径不能 rooted/平台前缀/父级穿越（ora-fs 路径规范）。

## 二、Workspace Files（只读文件浏览）

### HTTP 操作

| 操作 | 端点 | 说明 |
|---|---|---|
| listWorkspaceDirectory | POST files/list | 目录浏览（可选 path） |
| readWorkspaceFile | POST files/read | 只读查看（必填 path） |
| searchWorkspace | POST files/search | 文件名/内容搜索 |
| watchWorkspace | GET files/watch | NDJSON 流（data/error/end） |

**所有返回路径 slash 分隔、相对任务工作区；客户端从不提供 root 路径**（后端解析，防客户端越权）。

### 分层（每层很窄）

```
crates/fs                      路径校验、containment、文件边界、ripgrep、watch（不依赖 HTTP/前端）
web server workspace_file.rs   文件系统结果 → contracts 值
web server handlers/…          HTTP 提取、task-root 解析、NDJSON framing、生命周期
desktop workspace_files.rs     Tauri 命令映射（同一契约）
packages/app-shell/files       文件树/查看器/搜索 UI/缓存失效/行选择给 composer
```

### 错误映射

```
缺失 → file_system_path_not_found；非法/二进制/UTF-8 → invalid_request
超限 → payload-size / unprocessable；基础设施 → internal_error
（ora-fs 内部类型化，adapter 映射；隐藏路径，保留 source chain 给日志）
```

## 三、ora-fs 的 5 个能力（详解）

### ① path.rs — 路径校验（PortableRelativePath）

平台无关相对路径，统一 `/`；parse 规则：NUL → 拒、Windows 前缀（C:///）→ 拒、以 / 或 \ 开头 → 拒、`..` → 拒、空/`.` 段忽略（归一化）。例：`docs\specs//./api` → `docs/specs/api`。**目的：跨平台统一表示**（契约/数据库只有一种路径写法）。

### ② workspace.rs — canonical containment（CanonicalPathRoot）

```
root.canonicalize() → 根的真实身份
候选路径 canonicalize() → 候选的真实身份（symlink 被解析）
检查 starts_with(root) → 不在根内 = OutsideRoot 拒绝
```

**核心：canonicalize 后检查**——symlink 逃逸不拆穿就发现不了。**TOCTOU 诚实标注**：检查后到打开前 symlink 可能被替换（注释承认防不住）。

**逃逸白话**：工作区里有个快捷方式（symlink）指向墙外 → 文件"看起来在工作区、实际在墙外" → canonicalize 拆穿快捷方式 → 发现墙外 → 拒绝。

list_directory 细节：跳过 `.git`、symlink_metadata 标链接、排序（目录在前 + 忽略大小写）；read_file：≤10MiB、无 NUL、UTF-8、version token（mtime:size 供前端缓存失效）。

### ③ search.rs — ripgrep 搜索

**15s 超时 / 8MiB 输出 / 10000 结果**（第 13 课 Spec 同款）；**固定文本（非正则）**；注入 process spawner（测试 fake）；sentinel byte 区分"恰好到限制 vs 溢出"（截断上报）。

### ④ watch.rs — 文件监听

**原生事件 → 归一化 workspace-relative → 100ms 去抖合并 → NDJSON 流推前端**；rename 带新旧路径；歧义事件 → 全量 rescan。

**监听白话**：后台保安盯监控——文件一变就通知前端刷新（不用手动 F5）；连续动静打包成一条（100ms 合并）。

### ⑤ error.rs — 类型化错误

内部精细类型（WorkspaceUnavailable/PathNotRelative/PathOutsideWorkspace/PathNotFound/NotDirectory/NotFile/Io/FileTooLarge/BinaryFile/InvalidUtf8/RipgrepUnavailable/SearchTimedOut）→ adapter 映射 transport-neutral 错误；隐藏路径、保留 source chain。

## 四、安全边界汇总

| 约束 | 值 |
|---|---|
| 路径 | 绝对/穿越/Windows 前缀/解析出根 → 拒绝 |
| 只读 | 目录列出 + 文件读都只读 |
| 文件读 | ≤10MiB + 有效 UTF-8 + 无 NUL |
| 搜索 | 固定文本 + 15s/8MiB/10000 |
| watcher | 100ms 合并 + relative 归一化 + rename 双路径 + 歧义 rescan |

## 五、为什么抽 ora-fs 共享层

两个功能（工作区文件 + Spec）都要"安全读文件"——不抽出来各写一套校验会不一致（安全漏洞）；抽出来安全逻辑写一次、两处共用；**项目管理铁律（AGENTS.md）也要求优先用共享 ora-fs 能力**，别在自己模块另写。

## 六、术语表新增

Task Diff（任务变更审查）、diff_id（稳定变更标识）、anchor（评论锚点）、stale（过期）、Workspace Files（工作区文件）、ora-fs（共享文件层）、PortableRelativePath（可移植相对路径）、CanonicalPathRoot（规范化根）、containment（包含边界）、symlink 逃逸（链接逃逸）、canonicalize（规范化解析）、TOCTOU（检查-使用竞态）、ripgrep（搜索工具）、watcher（文件监听）、coalesce（合并）、sentinel byte（哨兵字节）、version token（版本令牌）。详见桌面 software technical terms.md。

## 七、下一课预告

> 第 15 课：Workflow 定义与版本管理——workflow 怎么存？draft / publish / version 生命周期？为什么草稿可改、发布快照不可变？（第二段 Workflow 专线开始）
