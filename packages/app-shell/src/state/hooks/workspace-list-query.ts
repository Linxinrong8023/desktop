/**
 * Shared `notifyOnChangeProps` for workspace list queries.
 *
 * Omits fetch-status flags so a background invalidate/refetch does not wake
 * every sidebar subscriber until `data` (or error/status) actually changes.
 *
 * Consumers of `useProjects` / `useTasks` / `useSessions` may therefore only
 * rely on `data`, `error`, `isPending`, `isSuccess`, `status`, and anything
 * derived from `status` (`isError`, `isLoading`). Reading `isFetching`,
 * `isRefetching`, `isStale`, or `fetchStatus` compiles and runs but silently
 * stops updating — add the prop here first if a caller needs one.
 */
export const WORKSPACE_LIST_NOTIFY_PROPS = [
  "data",
  "error",
  "isPending",
  "isSuccess",
  "status",
] as const;
