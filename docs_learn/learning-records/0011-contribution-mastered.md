# 已掌握 PluginContribution

## Demonstrated understanding

- 区分 TOML `kind`、`PluginKind` 标签与 `PluginContribution` 验证结果。
- 理解一个 Contribution enum 值只能选择一个互斥 variant。
- 理解 variant 与该类型的必需数据绑定，Agent contribution 不能缺少已验证 entrypoint。
- 理解布尔/Option 方案可以实现，但允许非法内存状态，需要运行时手写校验；enum 把保证提升到编译期。
- 理解“可信宿主视图”的信任范围不包含代码无恶意或运行永不失败。

## Next checkpoint

区分 contribution 与 runtime：插件提供能力不代表一定需要通用插件进程。
