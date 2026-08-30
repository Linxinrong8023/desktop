# Lifecycle 课程需要降速重讲

## Learner feedback

用户明确表示无法回答为什么 Agent Supervisor 必须通过 Lifecycle，当前逻辑混乱、细节过多；只知道“进程要统一管理”，但不知道为什么。

## Interpretation

第十三课同时引入状态机、operation lock、卸载顺序、generation lease 和单一所有权，超过当前认知负担。需要退回一个具体冲突案例，只建立“同一个事实只能有一个最终负责人”。

## Revised teaching target

只理解以下反例：如果 Lifecycle 启动进程 A，Agent Supervisor 还能私自启动进程 B，那么设置页停止 A 后 B 仍运行，系统同时出现“显示 stopped”和“真实进程仍存在”两个相互矛盾的事实。统一管理的目的，是让启动、停止和状态查询都经过同一个负责人，使显示状态等于真实状态。

## Mastery status

已建立第一层理解：Agent Supervisor 是 Ora Backend 中按已安装 Agent 插件创建的内存态监督者；用户选择 Agent 时通过 `agent_ref` 查找已有 Supervisor，而不是为聊天临时创建 Supervisor。

仍需继续检查的部分：Supervisor 与 Lifecycle 的职责边界——Supervisor 负责连接与重试，Lifecycle 是插件进程的唯一所有者。暂不引入完整状态机、锁、卸载或 lease。
