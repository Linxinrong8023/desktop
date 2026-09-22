# minicloud direction

English | [中文](overview.zh.md)

minicloud is the non-production local application for Ora's clone minimal loop and near-term needs.
It replaces Desktop as the first user-facing entry, not the existing Controller or Node responsibilities.
The Linux client/server clone loop is implemented; see [running minicloud](runtime.md).
The [root ADR](../../specs/decisions/minicloud/runtime/0-local-web-client-embedded-controller.md) is implemented;
its [core verification obligations](../../specs/test-cases/minicloud/runtime/local-web-clone-loop.md) are covered within the agreed scope, which excludes full browser-engine acceptance.

## Composition

- `apps/minicloud/client`: Vite, React 19 and shadcn; submit a repository URL and branch and observe results.
- `ora-controller`: the Controller executable itself serves the transitional clone API; there is no
  separate minicloud server. See the [Controller runtime](../controller/local-runtime.md).
- Node stays independent and uses the existing local IPC and host/guardian-managed Git execution; in
  development the Controller starts it with `--single-node`.

Both development listeners are local-only. The browser calls the Controller through Vite proxy.
No authentication, tokens, tenant model, additional security infrastructure or production deployment is added.
Loopback binding is a deployment restriction, not a security guarantee against hostile local callers.

Controller remains the owner of operation records; its API layer must not maintain a second task database or run Git.
Inject Controller identity, state directory and Node endpoint explicitly. Do not run a separate Controller
against the same state directory. Existing deployment isolation, file preservation and Git credential configuration remain intact.

## First loop

Persistently accept clone intent, return stable identities, then poll operation records and results.
Show the Node path and commit on success and the reported failure on failure; unavailable or unknown state
is not a failed clone. Refreshing the page or restarting the Controller must recover original records, not
resubmit new executions. No WebSocket, Desktop integration or Backend writer cutover is required.

See the [Node minimal-loop status](../node/minimal-loop.md) for implemented foundations and remaining evidence.
