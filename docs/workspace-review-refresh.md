# Workspace review refresh

English | [中文](workspace-review-refresh.zh.md)

`state/data/workspace-review.ts` owns `refreshWorkspaceReview(queryClient, workspaceId)`.
It invalidates every diff scope and staging status for that workspace. Active queries
refetch concurrently; inactive queries become stale and reload when observed. Other
workspaces retain their cache state.

Staging and commit mutations await this interface inside the operation, preserving
the originating workspace even if the view switches while the write is pending.
Manual refresh, prompt completion cleanup, and debounced Agent file-change /
turn-completion events use the same interface. Event consumption does not wait for network responses.

Each refresh explicitly cancels earlier diff/status requests, including initial loads
without cached data, before starting post-trigger requests. Late responses from a
transport that ignores cancellation cannot replace the new snapshot. Overlapping
refreshes share one completion per QueryClient and workspace: a replacement round
keeps every earlier caller pending until its diff and status requests settle,
including retries. The owner captures the active round's request promises because
`invalidateQueries` itself resolves immediately for paused queries. Offline pauses
keep mutations pending until reconnection and an actual success or terminal failure.
Superseded rounds cannot settle those waiters. Coordination is
released after completion so the next refresh starts a fresh round.
Refresh failures remain query errors rather than changing a successful Git write
into a failed mutation. The view presents diff/status errors and offers manual retry;
staging write errors are displayed without launching a success refresh.

Behavior tests use the production contracts client with explicit typed transport
handlers and controlled promises: `task-diff-view.review.test.tsx` covers actions,
pending, errors and workspace switches; `use-workspace-diff-live-sync.test.ts` covers
Agent triggers; `workspace-review.test.ts` covers query scopes and isolation.
`task-diff-view.review-races.test.tsx` covers three overlapping rounds, latest and
obsolete failures, B isolation, and an uncached pre-write status response. It also
uses the production QueryClient configuration to cover offline pauses, reconnection,
terminal refresh failures and overlapping refreshes while paused.
