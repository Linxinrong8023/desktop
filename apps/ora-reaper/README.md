# ora-reaper

`ora-reaper` is the process-cleanup sidecar shipped with Ora Desktop.

Ora starts the sidecar before constructing the backend. Every OS process created through the
production `TokioProcessSpawner` is registered synchronously and unregistered after its direct
child exits. During an orderly shutdown, Ora asks the sidecar to terminate any remaining process
trees and waits for acknowledgement. If Ora crashes, closing the inherited IPC pipe triggers the
same cleanup without cooperation from the parent.

The sidecar deliberately owns no application behavior, persistence, or restart policy. Its only
interface is the private, version-locked protocol implemented by `ora-process`; Desktop and the
sidecar are always built and shipped from the same source revision.
