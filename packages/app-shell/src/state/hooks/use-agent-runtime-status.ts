import { useQuery } from "@tanstack/react-query";
import type { AgentRuntimeStatus } from "@ora/contracts";
import { useContractsClient } from "../../contracts-client-context";
import { queryKeys } from "./query-keys";

/** Short-poll cadence used only while at least one CLI is still completing its ACP handshake. */
const STARTING_POLL_INTERVAL_MS = 1500;

/**
 * Slow cadence used while some agent is merely missing, so one that appears is eventually noticed.
 *
 * An unavailable agent is expected configuration rather than a fault, and it can become usable
 * without anything telling this client: a CLI installed into `PATH`, or a plugin whose supervisor
 * is most of a backoff interval away from retrying it. Polling on is what lets it come back.
 */
const UNAVAILABLE_POLL_INTERVAL_MS = 15_000;

/** Chooses how often to ask again, or `false` once nothing is expected to change on its own. */
function pollInterval(statuses: AgentRuntimeStatus[] | undefined) {
  if (statuses === undefined) return false as const;
  if (statuses.some((status) => status.status === "starting"))
    return STARTING_POLL_INTERVAL_MS;
  // `failing` is deliberately excluded: the supervisor has opened that agent's restart circuit and
  // will not retry it for the rest of the process, so asking again cannot change the answer.
  if (statuses.some((status) => status.status === "unavailable"))
    return UNAVAILABLE_POLL_INTERVAL_MS;
  return false as const;
}

/**
 * Loads the live per-agent detection status (ready/starting/unavailable/failing) that decides
 * which agents the pickers offer.
 *
 * The set is whatever this installation actually supervises: a built-in CLI is always supervised
 * and reports whether its executable was found, while a plugin-supplied agent is only supervised
 * while its package is installed and only reaches `ready` once the lifecycle agrees to start it.
 * That makes one answer cover "not installed on this machine", "package uninstalled", and
 * "package disabled" without the client having to model any of them separately.
 */
export function useAgentRuntimeStatus() {
  const client = useContractsClient();
  return useQuery({
    queryKey: queryKeys.agentRuntimeStatus,
    queryFn: () =>
      client.agentRuntime.getStatus({}).then((response) => response.statuses),
    refetchInterval: (query) => pollInterval(query.state.data),
  });
}
