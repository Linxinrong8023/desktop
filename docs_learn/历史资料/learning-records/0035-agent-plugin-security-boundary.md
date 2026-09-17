# Agent 插件的安全与信任边界

## Learner response

用户明确判断：Ora 通过 Host Processes 管理 Agent CLI 并不能证明 Agent 插件安全。用户指出 Agent 插件由 Deno 启动，当前获得广泛的文件、网络等能力；Host 接管的是插件所启动 Agent 进程的管理，而不是把插件代码变成安全代码。

## Mastered

- 能区分进程所有权/可回收性与最小权限沙箱。
- 知道 Agent 插件当前权限很高，不能把 `InstalledPlugin` 或合同验证表述为恶意代码安全证明。
- 能说明 Host Processes 的主要价值是让 Agent CLI 绑定 Plugin generation，统一管理 stdio、退出和进程树清理。
- 能得出“可验证、可监督、可回收仍不等于低权限”的面试结论。

## Precision corrections

1. Deno `main.js` 当前直接获得的是 `--allow-run`、全盘 `--allow-read`、`--allow-env` 和 `--allow-net`，没有直接授予 `--allow-write`。
2. 由于它可以运行具有当前操作系统用户权限的子进程，而真实 Agent CLI 也需要修改 Workspace，所以整体风险仍接近宿主权限，不能因为缺少 Deno `--allow-write` 就称为严格只读。
3. 包验证保证结构与路径不变量，运行时注册保证协议面不变量；二者都不能证明注册 handler 内部行为无害。

## Mastery status

第 19 课通过。用户能够准确反驳“Host 管理 Agent CLI，所以插件已经安全沙箱化”的错误推论。
