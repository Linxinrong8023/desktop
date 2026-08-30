# 已掌握 Agent 插件进程的运行时注册

## Learner correction and response

用户指出 `ora/register` 的声明者应是运行 `main.js` 的插件进程，而不是真实 Agent CLI。用户解释 Manifest 的 `kind = "agent"` 只说明插件类型；运行时注册用于告诉 Ora 可以调用哪些具体功能。若缺少 `agent/acp`，应在注册/合同验证时立即失败。

## Demonstrated understanding

- 能严格区分 Agent 插件进程与 Agent CLI 进程。
- 理解 Manifest kind 与 runtime capability registration 的阶段和职责不同。
- 理解运行时合同应在用户会话开始前 fail fast。
- 理解缺少 `agent/acp` 是确定性的合同错误，不应拖到第一条 prompt。

## Mastery status

`ora/register` 的核心目的和失败时机已掌握。下一检查点：理解插件 ID 与 process generation 的区别，以及旧连接/通知为何不能跨 generation。
