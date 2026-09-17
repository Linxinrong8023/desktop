# Task Workspace 文件

[English](task-workspace-files.md) | 中文

Desktop 的任务工作区文件功能提供目录浏览、有大小限制的文本查看、文件名和内容搜索、通过行号栏 `+` 将代码行引用到聊天（点击、拖动，或在聚焦的行号上按 Ctrl/Cmd+Enter）、原生文件变更刷新，以及从文件树创建空文件或目录、剪切、复制、粘贴、重命名和确认后删除。查看器本身保持只读。

## 所有权与流程

客户端发送 task id，并在需要时附带工作区相对路径。Tauri command 通过 Desktop 文件系统服务解析任务的权威工作目录，不接受调用方提供的根目录。`ora-utils` 拥有跨平台路径验证和规范化后的包含关系检查；`ora-fs` 将它们应用到工作区根目录，并负责文件大小限制、ripgrep 执行和原生监听。

各层职责保持精简：

- `apps/desktop/src-tauri/src/workspace_files.rs` 将文件系统结果映射为契约值，并在 IPC 中保留有类型的生命周期错误。
- `apps/desktop/src-tauri/src/commands/files.rs` 负责 Tauri 参数提取、任务根目录解析以及 command/channel 边界。
- `packages/app-shell/src/features/files` 负责文件树、查看器、搜索 UI，以及通过行号栏 `+` 将代码行引用到输入框。引用在发送前后都保持为紧凑标签：prompt 携带反引号包裹的 `path:range` 引用（由 agent 自行读取正文），聊天历史将其还原为相同标签，不重复展示源码。Diff 行号栏引用是例外：它展开为小型 `diff --git` 补丁，因为变更尚未写入磁盘。

聊天内的产物链接通过 `openWorkspaceFile` 和 `WorkspaceFileRequest` 打开此面板。请求包含 `path`、`requestId`，以及可选的 `FileNavigationLocation` 字段（line/column/endLine），因此重复点击同一文件仍然生效。视图从 ACP 绝对路径中去除任务 cwd 前缀，展开祖先目录以显示文件，并选中可选的行或包含起止行的范围，让查看器高亮并滚动到对应位置。引用范围使用与固定引用相同的 `--quote-tint` 底色（包括行号栏）；搜索结果保留琥珀色和 `<mark>`。随后点击引用范围外的位置会清除底色和标题中的 `:start-end` 标签，直到下一次跳转；搜索高亮保留到下一次结果。行号栏 `+` 和其他按钮不会清除引用高亮。

任务 cwd 之外的路径不会作为 worktree 相对路径打开，聊天中也不会将这些提及转为链接。文件缺失时（包括 agent 读取后被用户删除的文件），显示本地化的路径不存在提示，不直接展示 transport 错误。新的聊天 `requestId` 会使该路径的 Files 查询失效，使再次打开时重新读取磁盘而非沿用缓存。后端仍拒绝带根的路径。Desktop File Manager 在系统文件管理器中定位操作系统绝对路径，不启动 Cursor。

Files 数据所有者 `packages/app-shell/src/state/data/files.ts` 负责查询身份和失效规则。创建、复制、移动和删除成功后，等待其 scope 刷新入口处理目录、文件和搜索查询，无需等待 watcher 事件。非活跃查询标记为过期以便下次读取，活跃查询重新获取；其他 task 和 project checkout 保持隔离。UI 协调草稿、选中和展开。

## Project checkout 文件（草稿／尚无任务）

当聊天已选择 project 但尚无 task（空白或草稿输入框）时，Files 面板的 Explorer/Search 和 `@` 文件提及基于 **project checkout 根目录**解析，不使用任务 worktree。同一个主 Workspace 也提供模型发现和首次发送创建 session 时使用的工作目录。

这些操作复用任务 API 的 `ora-fs` 列表、搜索、读取限制和相对路径规则。实时监听通过 `watchProject` 监听 project checkout，与 `watchWorkspace` 监听任务 worktree 的方式相同。当 `taskId` 和 `projectId` 同时存在时，Files 和输入框优先使用任务 worktree，保持 linked-worktree checkout 的权威性。

## Desktop 操作

| 操作                     | 请求                             | 传递方式                        |
| ------------------------ | -------------------------------- | ------------------------------- |
| `listWorkspaceDirectory` | `taskId`、可选相对 `path`        | `list_workspace_directory`      |
| `readWorkspaceFile`      | `taskId`、相对 `path`            | `read_workspace_file`           |
| `searchWorkspace`        | task id 和受限搜索查询           | `search_workspace`              |
| `watchWorkspace`         | `taskId`                         | `stream_contract` Tauri channel |
| `listProjectDirectory`   | `projectId`、可选相对 `path`     | `list_project_directory`        |
| `readProjectFile`        | `projectId`、相对 `path`         | `read_project_file`             |
| `searchProject`          | project id 和受限搜索查询        | `search_project`                |
| `watchProject`           | `projectId`                      | `stream_contract` Tauri channel |
| `createWorkspaceEntry`   | `taskId`、相对 `path`、`kind`    | `create_workspace_entry`        |
| `createProjectEntry`     | `projectId`、相对 `path`、`kind` | `create_project_entry`          |
| `copyWorkspaceEntry`     | `taskId`、`from`、目标 `path`    | `copy_workspace_entry`          |
| `copyProjectEntry`       | `projectId`、`from`、目标 `path` | `copy_project_entry`            |
| `moveWorkspaceEntry`     | `taskId`、`from`、目标 `path`    | `move_workspace_entry`          |
| `moveProjectEntry`       | `projectId`、`from`、目标 `path` | `move_project_entry`            |
| `deleteWorkspaceEntry`   | `taskId`、相对 `path`            | `delete_workspace_entry`        |
| `deleteProjectEntry`     | `projectId`、相对 `path`         | `delete_project_entry`          |

创建操作拒绝替换已有路径，返回 `file_system_path_already_exists`；父目录必须已存在。Explorer 右键菜单在所选文件夹内创建，或在所选文件的父目录内创建（Cursor 的 Files 面板）；文件树空白区域在 checkout 根目录创建。剪切、复制、粘贴作用于条目本身，粘贴目标是文件夹或文件的父目录，同目录复制会添加 `name copy` 后缀。重命名通过行内编辑 basename 完成移动。删除先请求确认，再永久移除路径，包括文件、文件夹和不跟随的符号链接。

Copy Path、Copy Relative Path 和 Reveal in File Manager 复用 `joinOsAbsolutePath` 与 `locationActions.open("explorer", …)`。

所有返回路径都以斜杠分隔，并相对于解析后的 checkout。`watchWorkspace` 和 `watchProject` 发出 `data`、`error`、`end` 帧。错误帧使用共享的 `{ code, params, requestId }` 契约，因此前端使用与单次响应 command 相同的解码器。关闭期间已入队的终止错误以 `error` 发出，不作为成功的 `end`。

## 安全与 UI 行为

工作区根目录从持久化的 task 或 project 标识解析。路径必须验证为相对路径，在包含关系检查前规范化，并在读取、搜索或创建前应用限制。创建是限制在目录内的空文件写入（`create_new`）或目录创建（`create_dir`）；复制、移动和删除拒绝逃逸 checkout，且不跟随符号链接。Explorer 删除先请求确认，再永久移除路径。监听器变更是触发缓存失效的批次，不是事件日志。

查看器对不超过 400 行的文件挂载全部行。更大的文件只渲染视口附近的窗口及预渲染区域，使数 MB 的预览使用有界 DOM，不阻塞重新挂载面板的 session 切换。字符内容超过约 512 KiB 的文件还会将内容处理推迟到首次绘制之后，并显示加载遮罩，使 session 切换本身不等待预览。可滚动宽度根据最长行的等宽字符列数估算；固定范围随行挂载声明式地重新应用；语法高亮仍限制在 512 KiB。

Files 面板在 task 和 project 查看场景下都默认打开 Explorer。搜索和文件读取可通过注入的 contracts client 取消。面板卸载或 Files 范围在 task 与 project 之间切换时，停止已挂载的监听器。

任务 Changes（diff）面板同样遵循先渲染 session、再加载内容的顺序。它将后端 unified patch 拆成逐文件切片，并为每个切片缓存解析后的 `FileData`，使实时同步失效仅改写少量文件时，未变文件的 `memo` 比较仍然有效。字符内容超过 512 KiB 的补丁推迟到首次绘制之后解析；在切片解析完成前显示加载状态，不显示“无变更”。

当 diff 的文件数、总变更行数或单文件大小达到需要行窗口化的规模时，面板切换到单文件聚焦主体，由右侧文件树驱动选择，不使用滚动位置追踪，避免在一个滚动区域挂载整个大型变更集。每个文件仍使用原生 `react-diff-view` 样式（unified 和 split），使行号栏、底色和折叠行为与普通模式一致。

聚焦主体中超过 400 个变更行的文件还会按块窗口化：将文件拆成固定大小的原生 `react-diff-view` hunk，通过 `@tanstack/react-virtual` 仅挂载可见块窗口。因此数千行文件滚动时 DOM 仍有界，每个已挂载行仍是真正的 `Diff`/`Hunk`。窗口化位于文件组件内部，保留聊天引用跳行、行引用、折叠和展开等交互。跳转先让 virtualizer 滚到引用块，再滚到高亮行。

主体根据 diff 大小自动选择：大型变更集（文件多、变更行多或某个文件需要行窗口化）使用单文件聚焦主体，否则使用连续滚动主体。没有手动切换，因为连续滚动主体将每个文件挂载为一张原生表格，无法对文件内部窗口化，大文件会造成卡顿；聚焦主体通过有界 DOM 保持流畅。

Rust 契约位于 `crates/contracts/src/file_system.rs`，导出到 `packages/contracts/src/dto/file-system.ts`。端点目录 `xtask/src/frontend/namespaces/file_system.rs` 将监听器标记为流操作。修改这些 Rust 类型后，运行 `task export-contracts` 重新生成 TypeScript 契约包。
