import { useQuery } from "@tanstack/react-query";
import { useContractsClient } from "../../contracts-client-context";
import { queryKeys } from "./query-keys";

/** Loads the visible project list through the contracts client and caches it. */
export function useProjects() {
  const client = useContractsClient();
  return useQuery({
    queryKey: queryKeys.projects,
    queryFn: () =>
      client.project.list({}).then((response) => response.projects),
  });
}
