# Local Controller runtime

English | [中文](local-runtime.zh.md)

`ora-controller` owns durable local clone intent and result takeover. It does not execute Git, replace
Backend writers, expose a Client transport, or act as Cloud authority. Linux sessions use the existing
[Node IPC](../node/local-ipc.md) and length-prefixed JSON messages, without application credentials.

## Acceptance and persistence

Embedding code opens `Controller::open(home, controller_id)` and calls `accept_clone(request_id, spec)`.
The returned command contains stable operation/execution IDs. Acceptance writes the complete input and
target Node before returning; repeating a request returns the original command, while changed input is
rejected. `result(execution_id)` reads a durable terminal result; absence is not proof of failure.

The explicitly injected private directory contains `ora-controller.sqlite3`, independent of Node and
process state. Application ID `0x4f524143`, schema version 1, exact schema/integrity checks and an OS lease on
the sibling `ora-controller.sqlite3.lock` protect reopening; the lease lives beside the database so
SQLite's own file locks never collide with it on macOS or Windows. A different ControllerId or unknown existing file is rejected. No HOME-derived
storage location, database reset, task import or automatic Controller rebinding is provided.

`clone_operations` stores acceptance and the immutable terminal result. `clone_receipts` stores exact
Node event identities/content. Query completion and event delivery use the same takeover transaction;
only a received event produces an Ack after its receipt commits. Duplicate content is idempotent;
conflicting input/result/request association is rejected without acknowledgement. Historical Node
incarnations are retained, while query reporters and heartbeats must match the current session.

## Independent executable

`ControllerRuntime::open(RuntimeConfig)` also supports embedding. `handle()` exposes durable clone
acceptance, operation listing and lookup; `run(shutdown)` owns reconnect loops without installing signal
handlers. Missing lookup is distinct from an accepted operation without a terminal result. Callers stop
accepting requests, await shutdown and release handles to release the database lease. The standalone
executable uses this same runtime and supplies its own process signals.

Build `cargo build -p ora-controller -p ora-node -p ora-process-host -p ora-process-guardian`.
Deploy host and Node separately; configure Node's owner to match this ControllerId. Then run
`ora-controller /absolute/path/controller.json`:

```json
{
  "home_directory": "/home/node/controller",
  "controller_id": "deployment-controller",
  "protected_state_directories": ["/home/node/state", "/home/node/process"],
  "nodes": [
    {
      "node_id": "deployment-node",
      "endpoint": "/home/node/state/control.sock"
    }
  ],
  "session": { "io_timeout_ms": 10000, "query_interval_ms": 1000 },
  "reconnect_ms": 1000,
  "timezone": "Asia/Shanghai"
}
```

Declare all Node/host/guardian state roots in `protected_state_directories`; configured endpoint parents
are also protected. Overlap with Controller state is rejected before opening its database. The executable
recovers already accepted records; its configuration file and stdin are not business command channels.
Until a Client entry exists, acceptance is through the Rust interface, not direct SQLite editing.

Each configured Node has an independent reconnect loop over the same Controller owner. A handshake
checks Node identity and clone capability. Periodic status queries restore original execution state;
Unknown permits at most one exact command retransmission per connection, never a fresh execution.
Queries also keep sessions active; configure their interval below Node's idle frame deadline. Connection
loss, unsupported peers and persistence errors do not manufacture a failed clone or discard its records.
Normal Controller shutdown closes sessions, not accepted Node executions.

## Verification and remaining scope

Real SQLite tests cover acceptance, exclusive ownership, transaction failure, query/event ordering,
duplicate takeover and conflicting facts. Framed-session tests cover bounded Unknown retransmission
and rejection of a wrong Node identity or missing clone capability before dispatch.
The independent Controller–Node–host/guardian test performs real HTTPS clone, intercepts Ack, kills
Controller after durable takeover, restarts it offline, then checks original result, exact Ack, cleared
Node outbox and one mutation Run. Node's own IPC tests additionally cover Node restart and event replay.

A separate child process runs production `run_session` and the real SQLite owner with an injected
pre-commit barrier. The parent observes that no Ack escaped, sends SIGKILL while the takeover transaction
is open, and reopens the store to verify rollback and unchanged intent. The normal Controller executable
then takes over the replayed Node result with HTTPS refusing access. The barrier is a persistence test
dependency (`WritePoint::Commit`), not a deployment option or protocol extension.

These are not complete Client/UI, Cloud, multi-Controller or hostile-peer guarantees. Exhaustive queue
pressure, all crash boundaries and all deployment combinations remain tracked in the approved ADR's
core test cases. The existing Backend entry and Worktree coordination are unchanged.
