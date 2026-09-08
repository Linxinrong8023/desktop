import type { QueryClient } from "@tanstack/react-query";

/** Cache identity owned by workspace staging-status data; consumers never repeat its tuples. */
export const workspaceStatusKeys = {
  workspaceStatus: (workspaceId: string) =>
    ["workspace-status", workspaceId] as const,
};

/** Refreshes the staging status for one workspace, without affecting another worktree. */
export function invalidateWorkspaceStatus(
  queryClient: QueryClient,
  workspaceId: string,
) {
  return queryClient.invalidateQueries({
    queryKey: workspaceStatusKeys.workspaceStatus(workspaceId),
  });
}
