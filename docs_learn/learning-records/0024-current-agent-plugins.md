# 当前 Agent 插件与 OpenCode 二进制

## Learner question

用户询问当前是否只有 OpenCode Agent 插件，以及 OpenCode 插件是否随包携带二进制，使未单独安装 OpenCode 的用户也能使用。

## Verified on 2026-08-29

官方 `ora-space/marketplace` 的当前 main 分支包含三个 `kind = "agent"` 条目：

- `official/ora-space.opencode`，版本 0.3.0
- `official/ora-space.claude`，版本 0.1.1
- `official/ora-space.codex`，版本 0.1.0

因此“当前只有 OpenCode Agent 插件”已经过时。

OpenCode 插件确实采用 bundled binary：发布脚本从 `anomalyco/opencode` 下载对应平台的 CLI，放入 `.orax` 的 `assets/bin/opencode[.exe]`。运行时默认使用 `packageCommand` 让 Host 从插件安装根解析这个文件，不查找用户 PATH；仅当设置 `ORA_OPENCODE_BIN` 时才改用外部自定义二进制。

当前 marketplace 为 OpenCode 发布三个目标包：Apple Silicon macOS、x86_64 Linux 和 x86_64 Windows。安装器按 Host target 选择包，不支持的目标不能安装错误平台的包。

## Mastery status

已澄清事实。等待用户复述 bundled binary 与 `packageCommand` 的关系。

## Sources

- https://github.com/ora-space/marketplace
- https://github.com/ora-space/opencode-agent
