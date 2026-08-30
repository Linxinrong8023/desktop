# 已理解 InstalledPlugin 的价值

## Learner response

用户解释：如果后续模块只拿原始 Manifest，每个使用插件的模块都要针对不同插件类型重复编写和执行校验逻辑；集中生成一次可信宿主视图更简单。

## Demonstrated understanding

- 理解 `InstalledPlugin` 是验证结果，而不只是原始 Manifest 的换名。
- 理解集中验证可以消除后续模块的重复工作。
- 理解后续模块应依赖已经建立的不变量。

## Precision added

性能是次要收益。更主要的收益是避免不同消费者采用不同校验规则，降低安全遗漏、行为分歧和模块耦合。`InstalledPlugin` 的“可信”只涵盖已经验证的结构与宿主规则。

## Mastery status

`InstalledPlugin` 已掌握。下一检查点：理解 `PluginContribution` 如何把 kind 变成“恰好一种、携带专属数据”的内部类型。
