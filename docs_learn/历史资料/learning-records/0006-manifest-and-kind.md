# 已理解 Manifest 身份声明与 kind 分派

## Learner response

用户指出不能因为插件包存在 `main.js` 就推断它是 Agent 插件；必须由 `orax.toml` 的 `kind` 明确声明插件类型，并按不同 kind 使用不同的包内容校验规则。

## Demonstrated understanding

- 理解文件名不足以确定插件身份和类型。
- 理解 Manifest 提供明确的 kind 声明。
- 理解不同 kind 具有不同的文件和结构要求。
- 已意识到存在专门的校验模块，而不是由所有消费者各自猜测。

## Precision corrections

- 当前只有 Agent 与 Workbench 把根目录 `main.js` 作为进程入口并要求它存在。
- Webview、MCP、Hook 明确禁止 `main.js`。
- Skill 没有进程入口；当前 `validate_skill` 只检查 `assets/<skill>/SKILL.md` 等 Skill 结构，并不会因为额外存在一个 `main.js` 就把它误认成 Agent。

## Mastery status

“Manifest 为插件提供身份和 kind”已掌握。下一检查点：区分 Manifest 纯文本/结构校验与 Plugin Manager 的磁盘包校验。
