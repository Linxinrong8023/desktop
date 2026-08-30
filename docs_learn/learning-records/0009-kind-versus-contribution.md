# 正在区分 TOML kind、PluginKind 与 PluginContribution

## Learner question

用户质疑是否可以用 Contribution 字段直接替代 TOML 的 `kind`，并指出一个 TOML 中本来不会同时出现多个 `is_agent`、`is_skill`。

## Interpretation

这是合理质疑，暴露出教学中混淆了磁盘 schema 与 Rust 内部类型。多个布尔值的反例没有先说明讨论的是内部结构，容易让人误以为 enum 的目的只是替换 TOML 字段。

## Clarification

- `kind = "agent"` 是 TOML 中的待验证字符串。
- Manifest 解析后得到 `PluginKind::Agent`，它只有类型标签。
- Manager 验证磁盘入口后得到 `PluginContribution::Agent(InstalledPluginAgent { entrypoint })`，它把标签与已验证专属数据绑定。
- 把 TOML 字段改名为 `contribution` 不改变它仍是外部输入这一事实。
- 当前 TOML 的唯一 `kind` 设计本身已经避免多个类型布尔值；布尔值反例针对的是“标签 + 多个可选数据字段”的内部建模替代方案。

## Mastery status

等待用户解释：缺少 `main.js` 时可以解析出 `PluginKind::Agent`，但不能构造 Agent contribution，因为后者必须携带已验证入口。
