import { useQuery } from "@tanstack/react-query";
import { useContractsClient } from "../../contracts-client-context";
import { pluginKeys } from "../data/plugins";

/**
 * Loads every recorded pack installation with each member reconciled against the installed
 * tree. Read-only: the backend identifies drift, it never repairs the journal or the tree.
 */
export function usePackInstallations() {
  const client = useContractsClient();
  return useQuery({
    queryKey: pluginKeys.packInstallations,
    queryFn: () =>
      client.plugin
        .listPackInstallations({})
        .then((response) => response.packs),
  });
}
