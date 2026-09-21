import { useQuery } from "@tanstack/react-query";
import { useContractsClient } from "../../contracts-client-context";

/** Document identity prevents stale responses crossing edits or workspace switches. */
export function useWorkflowAnalysis(ownerId: string, graph: string) {
  const client = useContractsClient();
  return useQuery({
    queryKey: ["workflow", "analysis", ownerId, graph],
    queryFn: ({ signal }) => client.workflow.analyze({ graph }, { signal }),
    staleTime: Infinity,
    gcTime: 0,
    retry: false,
    enabled: ownerId !== "",
  });
}
