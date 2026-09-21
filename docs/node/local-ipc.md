# Local Node control session

English | [中文](local-ipc.zh.md)

Linux `ora-node` optionally accepts the existing length-prefixed JSON protocol over a private Unix
socket. Add an `ipc` object alongside the [clone deployment configuration](repository-clone.md):

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

The endpoint must be directly inside the injected Node home. That private directory and the Node
database lease protect endpoint recovery: only a same-owner private socket with a refused connection
may be replaced. Files, symlinks and live listeners are preserved. The optional section requires clone
configuration; without it the executable remains a recovery-only owner.

Deployment binds the persistent ControllerId, not the first peer. Reconfiguration to another owner
fails. Schema v4 attributes new clones atomically with acceptance; old unclaimed records stay intact
and are not replayed to or acknowledged by this session. This is trusted local ownership checking,
not cryptographic authentication or a sandbox against same-UID code.

One connection owns the handshake/control slot. Other connections are closed without replacing it.
Hello negotiates the existing version, Node identity/incarnation and clone capability. The session
accepts clone, status and exact acknowledgement messages; unsupported/conflicting messages close it.
The Node sends heartbeats independently of the blocking Git owner and actively replays bounded pages
of unacknowledged clone events. Status replies do not acknowledge events.

Admission uses a bounded queue and a revocable session guard. Only durable admission happens under
that guard; Git runs afterwards. Disconnect or session revocation discards unaccepted queued work,
but cannot cancel already accepted clones. Reads, writes and command admission replies use the finite
`frame_timeout_ms` deadline. A busy worker can therefore cause a query/command session to close even
while heartbeats are arriving; reconnect queries the original execution, not a new attempt. An idle
Controller should periodically query its executions. Slow readers may be disconnected and reconnect
for replay. Shutdown closes admission and then performs the existing managed-process cleanup.

The real standalone test verifies owner/duplicate rejection, HTTPS clone, Node kill/restart, unchanged
result replay and exact acknowledgement. [Controller acceptance](../controller/local-runtime.md) adds
an independent-process durable-takeover and lost-Ack recovery test.

Additional real-socket tests pause HTTPS during an accepted clone, observe live heartbeats, expire a
queued command and verify it remains Unknown while the accepted clone completes. Partial-frame tests
verify timeout and fresh admission; a non-reading peer is flooded with status replies until disconnect,
then reconnects to the identical unacknowledged result. No production test-only wire messages are used.
