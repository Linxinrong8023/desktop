import type { QueryClient } from "@tanstack/react-query";
import { diffKeys, invalidateWorkspaceDiffs } from "./diff";
import {
  invalidateWorkspaceStatus,
  workspaceStatusKeys,
} from "./workspace-status";

type ReviewRefresh = {
  generation: number;
  completion: Promise<void>;
  resolve: () => void;
  reject: (error: unknown) => void;
};

// A completion belongs to the workspace's overlapping refreshes, not to one
// cancellable query request. Separate clients must never share waiters.
const refreshes = new WeakMap<QueryClient, Map<string, ReviewRefresh>>();

/**
 * Replaces pre-refresh requests (including uncached initial loads) and refreshes
 * all review scopes and status. Overlapping callers wait for the latest round;
 * inactive queries become stale. Query failures remain on their queries rather
 * than making a successful Git write look failed and encouraging a repeat write.
 */
export function refreshWorkspaceReview(
  queryClient: QueryClient,
  workspaceId: string,
): Promise<void> {
  let workspaces = refreshes.get(queryClient);
  if (workspaces === undefined) {
    workspaces = new Map();
    refreshes.set(queryClient, workspaces);
  }
  let refresh = workspaces.get(workspaceId);
  if (refresh === undefined) {
    let resolve!: () => void;
    let reject!: (error: unknown) => void;
    const completion = new Promise<void>((onResolve, onReject) => {
      resolve = onResolve;
      reject = onReject;
    });
    refresh = { generation: 0, completion, resolve, reject };
    workspaces.set(workspaceId, refresh);
  }
  const current = refresh;
  const generation = ++current.generation;
  const run = async () => {
    // invalidateQueries alone reuses in-flight requests when data is undefined.
    // Explicit cancellation prevents a pre-write snapshot from satisfying this
    // refresh, even if the transport ignores cancellation and replies later.
    await Promise.all([
      queryClient.cancelQueries({
        queryKey: diffKeys.workspaceDiffs(workspaceId),
      }),
      queryClient.cancelQueries({
        queryKey: workspaceStatusKeys.workspaceStatus(workspaceId),
      }),
    ]);
    // A newer call may arrive during cancellation. Only its round may start
    // replacement queries or settle the shared completion.
    if (current.generation !== generation) return;
    const invalidations = [
      invalidateWorkspaceDiffs(queryClient, workspaceId),
      invalidateWorkspaceStatus(queryClient, workspaceId),
    ];
    // Invalidation starts refetches synchronously but its promise skips paused
    // queries. Capture this round's actual request promises before yielding so
    // offline time and paused retries cannot be mistaken for completion.
    const cache = queryClient.getQueryCache();
    const requests = [
      ...cache.findAll({
        queryKey: diffKeys.workspaceDiffs(workspaceId),
        type: "active",
      }),
      ...cache.findAll({
        queryKey: workspaceStatusKeys.workspaceStatus(workspaceId),
        type: "active",
      }),
    ]
      .filter((query) => query.state.fetchStatus !== "idle")
      .map((query) => query.promise?.catch(() => undefined));
    await Promise.all([...invalidations, ...requests]);
  };
  void run().then(
    () => {
      if (current.generation !== generation) return;
      workspaces.delete(workspaceId);
      current.resolve();
    },
    (error: unknown) => {
      if (current.generation !== generation) return;
      workspaces.delete(workspaceId);
      current.reject(error);
    },
  );
  return current.completion;
}
