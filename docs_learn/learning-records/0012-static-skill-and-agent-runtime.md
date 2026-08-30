# 已理解 Skill 静态贡献与 Agent 运行时差异

## Learner response

用户解释 Skill 插件是静态资源，不需要 Ora 为它启动 Deno 进程；Skill 如何交付给 Agent 由后续 Effect 系统处理。Agent 插件不同，需要运行 `main.js`，其中包含拉起真实 Agent 的逻辑。

## Demonstrated understanding

- 理解 Skill contribution 本身没有通用插件 runtime。
- 理解 Agent contribution 需要运行 `main.js`。
- 理解 `main.js` 是连接 Ora 与真实 Agent 的适配层，并会参与启动真实 Agent。
- 已将静态 Skill 与 Effect 集成、动态 Agent 进程联系起来。

## Precision added

- Effect 负责把 Desired Skill 安全物化到 Agent 声明的发现目录，并协调空闲/重启；它不替 Agent 理解或执行 Skill。
- `main.js` 不仅拉起 CLI，还注册 start/stop/listModels/ACP 合同，可声明 Effect surface。
- 插件逻辑上选择并管理 CLI，通常通过宿主 `ora/childprocess/spawn` 创建；宿主负责操作系统进程树回收。

## Mastery status

Contribution 与 Runtime 分离已掌握。下一检查点：完整复述 Agent 插件的 Deno 适配器进程与真实 Agent CLI 进程的职责和启动顺序。
