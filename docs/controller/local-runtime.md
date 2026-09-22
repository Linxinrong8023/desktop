# Local Controller runtime

English | [中文](local-runtime.zh.md)

`ora-controller` owns durable local clone intent and result takeover, and its executable hosts the
transitional clone API that [minicloud](../minicloud/runtime.md) calls. It does not execute Git, replace
Backend writers, or act as Cloud authority. Linux sessions use the existing
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

`ControllerRuntime::open(RuntimeConfig)` supports embedding. `handle()` exposes durable clone
acceptance, operation listing and lookup; `run(shutdown)` owns reconnect loops without installing signal
handlers. Missing lookup is distinct from an accepted operation without a terminal result. The library
depends on no listener; `Service::start(DeploymentConfig, Transport, NodeHosting)` composes the API
listener, the sole runtime owner and an optionally hosted Node for the executable and for tests.

Build `cargo build -p ora-controller -p ora-node -p ora-process-host -p ora-process-guardian`.
Deployment state lives in one configuration file; per-process composition is given on the command line:

```text
ora-controller --config /absolute/path/controller.json [--single-node]
               [--transport tcp|unix] [--host 127.0.0.1] [--port 4820] [--socket /path/api.sock]
```

| Flag | Rule |
|---|---|
| `--transport tcp` (default) | `--host` defaults to `127.0.0.1`, `--port` to `4820`. A non-loopback host is accepted with a warning: the API has no authentication, so loopback is a deployment restriction, not a security guarantee. |
| `--transport unix` | Requires `--socket`, an absolute path directly inside `home_directory`, created with the same private-socket rules as the Node endpoint. `--host`/`--port` are rejected. |
| `--single-node` | Starts the configured Node from the `single_node` section and stops it on normal shutdown; see below. |

Invalid flag combinations and configuration are rejected before the database lease is taken.

```json
{
  "controller": {
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
  },
  "api": { "node_id": "deployment-node" },
  "single_node": {
    "node_executable": "/opt/ora/bin/ora-node",
    "node_config": "/home/node/config/node.json",
    "ready_timeout_ms": 30000,
    "stop_timeout_ms": 30000
  }
}
```

`api.node_id` names the configured Node that accepted clones are dispatched to; callers never choose a
Node. Declare all Node/host/guardian state roots in `protected_state_directories`; configured endpoint
parents are also protected. Overlap with Controller state is rejected before opening its database. The
executable recovers already accepted records; its configuration file and stdin are not business command
channels. Deploy host and Node separately unless hosting the Node, and configure Node's owner to match
this ControllerId.

With `--single-node`, `nodes` must contain exactly the `api.node_id` Node. Before opening state, the
executable reads `node_config` read-only and refuses to start when its `ipc.controller_id` or
`ipc.endpoint` does not match, or when something already accepts connections on the endpoint. It then
starts `node_executable <node_config>` in its own process group (no new session), waits up to
`ready_timeout_ms` for the endpoint, and only then binds the API. Process host and guardian are
prerequisites: the executable neither deploys nor starts them. Controller death alone signals nothing to
the Node, so an accepted clone keeps running; a group-level stop from an operator or launcher reaches
both. If the hosted Node exits on its own, the Controller shuts down and exits with failure rather than
accepting undispatchable requests.

Normal shutdown stops in order: API admission (bounded wait for in-flight requests), Node sessions, the
hosted Node (`SIGTERM`, waiting up to `stop_timeout_ms`; never escalated to `SIGKILL`), then the database
lease. Accepted Node executions are never cancelled by this process stopping.

The JSON surface is the transitional clone API documented under
[minicloud](../minicloud/runtime.md#http-interface); its DTOs live in `ora-contracts::controller_api`.
The Cloud-facing contract will be defined by proto and served on the same listener; until then the JSON
surface is the only client entry.

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
