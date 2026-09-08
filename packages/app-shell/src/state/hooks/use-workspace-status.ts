import { useQuery } from "@tanstack/react-query";
import { useContractsClient } from "../../contracts-client-context";
import { workspaceStatusKeys } from "../data/workspace-status";

/** Loads the structured per-file staging status for one workspace checkout. */
export function useWorkspaceStatus(workspaceId: string, enabled = true) {
  const client = useContractsClient();
  return useQuery({
    queryKey: workspaceStatusKeys.workspaceStatus(workspaceId),
    queryFn: () => client.workspace.getStatus({ workspaceId }),
    enabled: enabled && workspaceId !== "",
  });
}
