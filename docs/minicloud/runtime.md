# minicloud local runtime

English | [中文](runtime.zh.md)

## One-command development environment

Run `task run:minicloud` at the repository root. The launcher installs frontend dependencies, builds
debug binaries, then starts host, Node, the server embedding Controller, and Vite. Open
`http://127.0.0.1:5174`. Linux, Deno, Cargo, Node.js, Git and `setsid` are required; root is not.

Development configuration and runtime data live under `~/.ora/minicloud/<digest>/`, where `<digest>`
is derived from the checkout path and the `workspace` file records that path. Each checkout therefore
keeps separate state; a directory claimed by another checkout is rejected. The launcher writes nothing
into the repository:

- `config/node.json`, `server.json`, `client.json`: deployment configuration and frontend port.
- `config/clone.gitconfig`: noninteractive Git configuration; the default supports credential-free HTTPS repositories. Configure private-repository credentials explicitly.
- `node/`, `controller/`: databases and Node IPC; `p/`: host/guardian state (short to leave room for Unix socket names).
- `repositories/`: clones; `home/`: workload HOME; `bin/`: identifiable versioned guardians; `vite/`: Vite cache.

Repeated runs preserve configuration edits, databases and checkouts; failed recovery never falls back
to clearing state. Logs go to the terminal. Dependencies and build outputs retain the standard
repository `node_modules`/`target` locations. Use `deno run -A scripts/run-minicloud.ts --init-only`
to initialize without starting, or `--no-build` to skip installation and compilation. Restart after
changing ports; launcher-owned state paths must not point at a different deployment.

Ctrl+C or unexpected component exit stops Vite/server, waits for Node to clean up managed Git, then
stops host and guardians belonging to this directory. Stop timeouts are reported before escalating
signals; escaped workload descendants are not guaranteed to terminate. Interrupted clones may remain
pending/unknown for recovery; they are never automatically recreated or deleted. An exclusive lock
rejects a second launcher for the same data directory.

Debug builds skip Unix permission-bit checks on trusted paths, allowing group-writable checkouts
without changing existing permissions. Owner, symlink, hard-link, type, directory-isolation and database
ownership checks remain enabled; release builds still enforce permission bits. State is keyed to the
home directory rather than the checkout because Unix socket paths are limited to 108 bytes; an
unusually long real home path is rejected, not shortened or redirected through a symlink.

## Manual deployment

The Linux HTTP executable embeds the same Controller runtime as `ora-controller`. Start Node and
host/guardian using their existing [deployment configuration](../node/repository-clone.md), then build
`cargo build -p ora-minicloud-server`. Run `ora-minicloud-server /absolute/path/minicloud.json`:

```json
{
  "listen": "127.0.0.1:4317",
  "node_id": "deployment-node",
  "controller": {
    "home_directory": "/home/node/controller",
    "protected_state_directories": ["/home/node/state", "/home/node/process"],
    "controller_id": "deployment-controller",
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
}
```

The Node owner must match this ControllerId. Stop any standalone Controller using the same state
directory before starting minicloud. All state paths remain explicitly injected and protected from
overlap. Non-loopback listening or an unconfigured target is rejected before opening Controller state.
This is a non-production application, with no authentication or additional security infrastructure.

## HTTP interface

After `deno install`, run `deno task --filter @ora/minicloud-client dev` from the repository root and
open `http://127.0.0.1:5174`. Vite proxies `/api` to `http://127.0.0.1:4317`; set
`MINICLOUD_SERVER_URL` when using a different server port. The page uses the shared shadcn components
with React 19 and polls Controller operations. An unresolved submission is saved in tab session storage
before dispatch; after response loss or reload, “retry original request” reuses its identity and input.
This storage is not operation history. Closing the page aborts HTTP and polling, not the Node execution.

- `POST /api/clones`: `{ "requestId": "stable-client-id", "repository": "https://host/repo.git", "branch": "main" }`.
  Returns HTTP 202 with `requestId`, `operationId`, and `executionId` after durable acceptance.
- `GET /api/clones`: newest accepted operations first, including pending records while Node is offline.
- `GET /api/clones/{executionId}`: one operation; 404 means no accepted operation with that identity.

State is `pending`, `succeeded` (Node path and commit), or `failed` (known reason and retained path).
Pending makes no claim about whether Git is currently running. HTTP 400 means invalid input, 409 an
identity/input conflict and 503 temporary unavailability; HTTP errors are not terminal clone failures.
The browser DTOs are generated from `ora-contracts::minicloud`, separate from the Node wire protocol.

Repeat an uncertain submission with the same request identity and input. Closing a browser or stopping
server does not cancel a Node execution. Restart with the same state directory to recover original facts.
There is no separate minicloud task database, cleanup command or automatic new-execution retry.

Real HTTP tests cover acceptance, conflicts, invalid input, offline listing, missing identities, exclusive
ownership, loopback restriction and normal server restart. Lower-level Controller crash tests remain
separate from minicloud's own end-to-end evidence.

Frontend checks: `deno task --filter @ora/minicloud-client lint`, `test`, and `build`.
`task test:minicloud` additionally runs real HTTP and Vite proxy → independent minicloud server →
Node → HTTPS Git tests, including server SIGKILL/restart and one mutation Run. It requires Linux and
installed frontend dependencies. The Vite case is explicitly opt-in for Rust-only CI runners.
The real chain also holds a SQLite writer lock to verify HTTP 503 without accepted intent, then releases
it and verifies a single Run. A real TCP proxy truncates the accepted response body; retry after server
restart preserves the original execution. Normal server shutdown during clone preserves the pinned live
Git process and eventually produces the same result with one Run. Server entry tests reject overlapping
directories and preserve unknown files, and reuse intent accepted by a separate Controller owner.
DOM tests verify UI behavior, automatic polling recovery and reload identity recovery. Full browser-engine
interaction acceptance is explicitly outside the agreed scope.
