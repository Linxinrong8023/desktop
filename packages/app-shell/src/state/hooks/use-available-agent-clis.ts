import { useMemo } from "react";
import {
  AGENT_CLI_ORDER,
  type KnownAgentCli,
} from "../../features/chat/model-catalog";
import { useAgentRuntimeStatus } from "./use-agent-runtime-status";

/**
 * Resolves which agents the pickers may offer on this installation, in the catalog's order.
 *
 * Which agents exist is no longer knowable at build time. A built-in CLI ships with every build
 * but its executable may be absent from this machine; an agent supplied by a plugin does not exist
 * here at all unless its package is installed, and the user can revoke it at any time by disabling
 * that package. Offering an agent in either of those states advertises something no session could
 * be opened on, so the list is the agents the runtime actually reports reaching.
 *
 * `starting` counts alongside `ready` because an agent still completing its handshake is on its
 * way to being usable, and an entry that vanished for the first second of every launch would take
 * the surfaces pointing at it along with it. `unavailable` and `failing` do not: the first means
 * nothing answered — a missing executable, an uninstalled or disabled package — and the second
 * means the supervisor has given up on it for the rest of the process.
 *
 * While the status is still loading the full catalog is offered. "Not answered yet" is not "not
 * detected", and answering it as such would move every surface onto a different agent for as long
 * as that query is in flight.
 */
export function useAvailableAgentClis(): KnownAgentCli[] {
  const { data: statuses } = useAgentRuntimeStatus();
  return useMemo(() => {
    if (statuses === undefined) return AGENT_CLI_ORDER;
    const detected = new Set(
      statuses
        .filter(
          (status) => status.status === "ready" || status.status === "starting",
        )
        .map((status) => status.agentRef),
    );
    return AGENT_CLI_ORDER.filter((agentCli) => detected.has(agentCli));
  }, [statuses]);
}
