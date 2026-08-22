import { useQuery } from "@tanstack/react-query";
import { useContractsClient } from "../../contracts-client-context";
import { queryKeys } from "./query-keys";

/** Loads the visible agent session list through the contracts client and caches it. */
export function useSessions() {
  const client = useContractsClient();
  return useQuery({
    queryKey: queryKeys.sessions,
    queryFn: () =>
      client.session.list({}).then((response) => response.sessions),
  });
}
