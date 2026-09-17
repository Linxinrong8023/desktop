# 第 1 课：认识仓库、程序与依赖地图（总结）

> 对应对话内容：从"Ora 仓库里有什么"到"契约、序列化、xtask 生成 TS"的完整第一课。

## 一、这一课回答的核心问题

> Ora 这个仓库里到底有什么？它们怎么组织、怎么协作？

## 二、四个核心心智模型

### 1. 分层大楼（依赖只能向下）

```
┌─ 门卫层：ora_web_server · ora-desktop（两个可执行程序）
├─ 总装层：ora-backend（把下面所有零件装成整机）
├─ 规则层：ora-application（业务规则，不碰数据库）
├─ 词典层：ora-domain（领域概念）· ora-contracts（通信格式）
└─ 工具层：gitlancer（Git）· ora-logging（日志）· ora-process（进程）
```

**规则**：上层用下层，下层不认识上层。哪里出了问题，顺着箭头找它属于哪层。

### 2. 前端是一栋 4 层楼（餐厅比喻）

```
第3层 分店：apps/web/client（接浏览器）· apps/desktop（接桌面）
第2层 总店：@ora/app-shell（组装一切）
第1层 部门：@ora/chat（聊天数据）· @ora/platform（平台差异）
第0层 物资：@ora/contracts（格式）· @ora/ui（界面零件）
```

**妙处**：总店及以下，Web 版和桌面版是同一份代码；分店只负责"接线"（运输方式不同：HTTP vs Tauri IPC）。

### 3. 母版 + 复印机（单一事实来源）

- **母版**：Rust 的 `ora-contracts`（只有这一份"说了算"）
- **复印机**：`xtask export-contracts` → 自动印出 TS 文件（开头写着"不要手改"）
- **为什么**：手写两份 = 两个会漂移的谎言；生成 = 一份真相 + 编译期防漂移

### 4. 编译期 vs 运行期

| 东西 | 时间 | 作用 |
|---|---|---|
| DTO 类型（project.ts 等） | 编译期，写完就擦掉 | 检查你写的对象字段名、类型对不对 |
| endpoints 常量（endpoints.ts） | 运行期，一直存在 | 客户端真去读：该用 POST、该打哪个地址 |
| client / fetch / transport | 运行期（手写的） | 真正的工人：拼请求、发 HTTP、拆响应 |
| 序列化 / 反序列化 | 运行期 | 双方各做一半：打包和拆包 |

## 三、三个必须能脱口而出的判断

1. **request 是谁造的？** —— 不是 TS 生成的，是开发者写的对象 + `JSON.stringify` 打包出来的。
2. **为什么前后端不会"对不上话"？** —— 格式只有 Rust 一份母版，TS 是复印的；写错在编译期就被拦住。
3. **Web 和桌面为什么能共享一套代码？** —— 业务逻辑全在共享的 `ora-backend` 里，前端共享 app-shell，只有"运输方式"不同。

## 四、本课检查题（已全部答对）

1. 可独立运行的程序：3 个（ora_web_server、Ora Desktop、xtask）。
2. `ora-application` 不依赖数据库 → 测试时可以换内存假实现（依赖注入的好处）。
3. Web/Desktop 相同：共享同一套后端 crates；不同：Web 有 HTTP 服务器、Desktop 走 Tauri 且独立于根 Cargo 工作区。
4. 前后端"同一种语言"：`ora-contracts`（Rust）定义，xtask 生成 TS。
5. `target/`、`node_modules/`、`.data/`：都是生成物/运行数据，非手写源码。
6. 加按钮改 TS 世界；改数据库表结构改 Rust 世界（`ora-db` 迁移）。

## 五、本课新增术语（详见桌面临时术语表）

Binary、Library、Manifest、Package Manager、Dependency、Dependency Graph、Layered Architecture、Entry Point、Contract、DTO、Serialization、Deserialization、Single Source of Truth、Naming Convention、Composition Root、Monorepo、Workspace（双重含义）、Repository（双重含义）等。

## 六、下一课预告

> 同一个"项目"，在系统里其实有三种样子：数据库里的一行、领域模型里的对象、契约里的 DTO。为什么非要分成三种？

第一次真正深入 Rust 代码：对比阅读 `ora-domain` 和 `ora-contracts`。
