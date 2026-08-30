# 已掌握 Manifest 与 Manager 的两阶段验证

## Learner response

用户解释：Manifest 模块负责检查 `orax.toml` 是否符合格式规范；Plugin Manager 根据已经解析出的 `kind`，检查每种插件包实际需要的内容。

## Demonstrated understanding

- 能区分纯声明校验与磁盘包校验。
- 理解 Manager 使用经过 Manifest 验证的 kind 分派类型专属规则。
- 理解缺少 Agent `main.js` 属于 Manager 能判断的磁盘事实。

## Precision added

Manifest 不仅检查字段语法，还检查无需访问磁盘即可判断的语义组合，例如 kind 与 kind-specific section 是否匹配。Manager 负责依赖宿主或文件系统的规则。

## Mastery status

两阶段验证已掌握。下一检查点：理解 `InstalledPlugin` 是验证后供后续模块消费的可信结构，而不是原始 Manifest 的别名。
