# 本机 Node 控制会话

[English](local-ipc.md) | 中文

Linux `ora-node` 可通过私有 Unix socket 接收现有长度前缀 JSON 协议。
在 [clone 部署配置](repository-clone.zh.md)旁增加 `ipc`：

```json
{
  "ipc": {
    "controller_id": "deployment-controller",
    "endpoint": "/home/node/state/control.sock",
    "heartbeat_ms": 1000,
    "frame_timeout_ms": 10000
  }
}
```

endpoint 必须直接位于注入的 Node home 内。私有目录与 Node 数据库独占锁保护 endpoint 恢复：
只替换同用户、私有且连接被拒绝的旧 socket，保留普通文件、符号链接和活动 listener。
启用此配置要求已配置 clone；不配置 IPC 时仍作为只恢复历史执行的独立程序运行。

部署指定并持久绑定 ControllerId，不由第一个连接者认领；更换归属配置会失败。
schema v4 在接受事务中为新 clone 保存归属；旧的未认领执行原样保留，不向此会话重放或允许其确认。
这是可信本机归属检查，不是密码学认证，也不隔离同 UID 恶意代码。

一个连接占有握手／控制槽，其他连接直接关闭，不顶替旧会话。Hello 协商现有版本、Node 身份／运行实例及
clone 能力；会话接收 clone、状态查询和精确确认，冲突或不支持的消息会关闭连接。
心跳独立于阻塞 Git 执行；Node 主动按有界分页重放未确认 clone 事件，查询回复不确认事件。

受理使用有界队列和可撤销会话门禁。门禁只覆盖持久受理，随后才运行 Git；断连或会话撤销丢弃尚未受理
的排队工作，不取消已经受理的 clone。读帧、写帧及命令受理回复均使用有限的 `frame_timeout_ms` 期限。
执行线程繁忙时，即使心跳正常，查询／命令会话也可能超时关闭；重连查询原执行，不创建新尝试。
空闲 Controller 应定期查询执行；慢读可被断开，
随后重连恢复投递。正常停止先关闭受理，再执行原受管进程清理。

真实独立入口测试覆盖归属／重复连接拒绝、HTTPS clone、Node 强杀重启、原结果重放和精确确认。
[Controller 验收](../controller/local-runtime.zh.md)另外覆盖独立进程持久接管及 Ack 丢失恢复。

补充真实 socket 测试会在 clone 已接受后暂停 HTTPS，观察心跳，令排队命令超时，
再验证该命令仍为 Unknown，而原 clone 继续完成。半帧测试验证超时与重新受理；
慢读测试持续发送状态查询但不读取回复，直至断连，再重连读取完全相同的未确认结果。
没有为测试增加生产协议消息。
