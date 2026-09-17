# 添加或移除功能

[English](feature-change-guide.md) | 中文

从负责该功能的用例开始，而不是从文件数量开始。DTO、生成产物、测试和文档属于不同类别；变更多个文件本身并不意味着事实重复。

## 在现有领域中添加操作

DTO 模块在 `#[ts(export_to = "...")]` 中声明的文件名统一使用 kebab-case（例如 `app-event.ts`、`workflow-run.ts`）。生成的 DTO barrel 沿用这些声明。

1. 在负责该领域的 `crates/contracts/src/<domain>.rs` 中定义请求／响应 DTO，并在同一模块的 `export` 函数中注册其 TypeScript 导出。不要把 transport 路由和 Webview 授权放入公共 DTO／manifest。真正新增的 DTO 家族还需要新增一个明确的 Rust 模块／导出组合入口。
2. 在 `xtask/src/frontend/namespaces/<namespace>.rs` 中添加逻辑操作：操作名称、客户端命名空间／成员、DTO 名称，以及明确的 `FrontendResponseMode::Unary` 或 `Stream`。没有需要更新的集中式 stream 名称回退机制。
3. 在 `apps/desktop/src-tauri/bindings/` 中添加 Desktop 所有的绑定。Unary 绑定必须显式声明 Rust handler 和 `Permission`。Stream 绑定必须声明领域启动 handler，并复用现有的、已授权的 `stream_contract`／`cancel_contract_stream` 对。即使没有 SDK 操作，新的 native command 也属于 Desktop catalog。
4. 在领域模块中实现该用例。Backend 操作使用窄领域句柄；不要添加根 `Backend` 转发方法，也不要暴露存储、锁或 supervisor。Desktop adapter 通过现有请求生命周期辅助方法调用该句柄。文件 stream 的启动属于 `commands/files.rs`；共享的 `StreamStart` 负责注册、取消、启动结算和转发。不要在领域中重复这些规则。
5. 运行 `task export-contracts`，然后运行 `task check:contracts`。不要手动编辑客户端转发、DTO barrel、transport map、stream enum／dispatch、command 注册或权限产物。这些内容由负责它们的声明生成。验证确定性时重复生成；检查不会修复漂移，也不会删除未知的手写文件。
6. 在操作真正的接口处添加测试。Backend 测试在行为本身需要时保留真实 SQLite／Git；前端测试使用生产客户端和类型化的操作 handler。验证 DTO 和 call-option 的传递、错误／requestId 行为以及相关生命周期结果。Stream 需要有消费、abort／卸载和清理证据，不能只测试构造函数返回了可迭代对象。
7. 迭代时运行最小相关测试，然后运行必要的包检查。Desktop 变更需要 `task test:tauri`，不能只运行 workspace 测试；跨仓库重构以 `task test` 收尾。Catalog 有效性检查不能替代对实际 handler 的编译。

四项手写事实是**契约**、**逻辑操作**、**Desktop 绑定**和**业务实现**。业务实现可以合理地横跨其领域句柄和宿主 adapter。目标不是恰好变更四个文件；生成文件、测试和文档分别计数。添加全新领域时还可能需要修改明确的组合根。

## 前端所有权检查清单

- 将翻译数据放在负责该功能的位置。只有新增所有者时才在根 i18n 组合中添加资源；保留一个同步的 `appI18n` 初始化和 locale 存储语义。资源 key 的对应关系／重复项必须通过检查。使用翻译的渲染测试要自行初始化该实例，并通过 clean-stderr 检查，不能依赖基于计时的警告抑制。
- 共享 Cut、Copy、Paste、Select All 文案归 `features/editor/translations.ts` 所有，包括 Chat 消息按钮使用的 Copy。保留历史 `chat.*` key 名称；Chat 专用的 Copy code 文案继续归 Chat 所有。
- 将查询身份、权威响应采用和失效放入 `state/data/` 下的数据所有者。保留有意义的 tuple／prefix 区分、删除级联和事件刷新范围。UI 负责选择和呈现，不得复制 query-key 字符串。
- 在功能的 `interface.json` 中只声明所需的公共符号，并写明其用途。其他符号即使位于同一个源文件中也保持私有。在暴露私有实现之前，先判断该职责是否真的共享，或调用方是否应使用已有的公共接口。
- 显式组合测试用 memory adapter。缺少的操作必须失败；只有测试明确选择这种合成行为时，空成功才有效。通过类型化 handler 配置自定义行为，不要替换生成客户端的方法。
- 明确 watcher、事件桥、timer、QueryClient observer 和 native surface 的所有者。范围变化以及卸载／关闭必须释放旧资源。验证缓存隔离和实际订阅终结。Files 继续负责显式的 Explorer／Search 组合；不需要通用 view／plugin 容器的功能不要添加这种容器。

## 移除功能

1. 先识别它的公共消费者和持久数据。产品删除并不授权删除用户文件。文件系统布局是硬兼容边界：如果旧路径可能与新布局冲突，或可能阻止安全启动，必须在改动前明确设计检测／迁移／隔离方案。
2. 删除其所有者实现、DTO 和本地 DTO 导出项、逻辑 catalog 项、Desktop 绑定以及确实已经过时的组合项。同时删除其本地公共接口声明和翻译。不要保留未使用的根转发或第二套实现作为永久兼容层。
3. 通过现有领域接口移除或更新消费者。对于共享数据，确认其他所有者是否仍需要它的 query factory 和失效策略。停止订阅并移除功能所有的缓存／草稿状态，但不要擦除另一个 workspace 的状态。
4. 重新生成。被移除的操作必须从生成客户端、路由、DTO 导出、注册和适用的权限中消失。检查已跟踪的差异和未跟踪的生成文件；保留未知的手写文件。分别确认主 Webview 和 plugin Webview 的 capability，并在仍有任何 stream 需要时保留共享 stream command。
5. 将测试随仍存活的职责一起移动。只有当行为被有意移除，或替代方案直接覆盖它时，才删除测试义务。相关情况下，保留失败、持久化、并发、取消／恢复和关闭的回归证据。
6. 搜索旧操作名称、import、query key、翻译 key 和 fixture wiring。运行生成、功能接口和 Rust 大小检查。删除已移除模块的过时大小例外。更新所有权文档、适用的面向用户文档和证据索引。`specs/` 是独立仓库；修改其中内容需要遵循它自己的指令并检查其状态，不能作为父仓库的附带修改。

按类别审查最终差异：所有者实现／声明、必要的组合、生成产物、测试和文档。临时的添加／移除演练应放在隔离的 worktree 中；删除其手写 fixture，重新生成，确认 worktree 干净，只保留证据，不保留临时产品操作。
