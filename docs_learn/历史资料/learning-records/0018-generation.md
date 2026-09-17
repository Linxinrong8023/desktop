# 已掌握旧 generation 消息必须丢弃

## Learner response

用户回答 generation 1 的迟到 ACP notification 不应交给 generation 2，而应直接丢弃，否则会让对话历史显得突兀和错误。

## Demonstrated understanding

- 理解同一 Plugin ID 下不同 generation 的消息不能混用。
- 理解旧代异步消息进入新代会破坏用户可见会话一致性。

## Precision added

问题不仅是展示突兀。ACP request id、provider session id、pending correlation 和 Agent 上下文都绑定旧连接；交给新代可能误配另一个请求或污染状态。重启后的会话需要通过正常 load/re-establishment 恢复，而不是接收旧代尾帧。

## Mastery status

generation 隔离的核心目的已掌握。下一检查点：理解 Plugin Lifecycle 为什么必须是插件进程的唯一所有者。
