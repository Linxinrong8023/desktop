import { waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { AgentStatus } from "@ora/contracts";
import {
  createMockClient,
  createMockClientState,
  type MockClientState,
} from "../../test/mock-client";
import { renderHookWithClient } from "../../test/hook-harness";
import { AGENT_CLI_ORDER } from "../../features/chat/model-catalog";
import { useAgentRuntimeStatus } from "./use-agent-runtime-status";
import { useAvailableAgentClis } from "./use-available-agent-clis";

/** The catalog without OpenCode, which every "one agent is missing" case here drops. */
const WITHOUT_OPENCODE = AGENT_CLI_ORDER.filter(
  (agentCli) => agentCli !== "ora-space.opencode",
);

/** Replaces what the runtime reports about one agent, leaving the rest detected. */
function reportOpenCode(status: AgentStatus) {
  return (state: MockClientState) => {
    const entry = state.agentRuntimeStatuses.find(
      (candidate) => candidate.agentRef === "ora-space.opencode",
    );
    entry!.status = status;
  };
}

/**
 * Renders the hook against a settled detection status and returns what it offers.
 *
 * The status is awaited through the same query the hook reads, because the loading answer is the
 * whole catalog: asserting before it settles would pass for reasons the test is not about.
 */
async function offeredAgentClis(
  seed: (state: MockClientState) => void,
): Promise<string[]> {
  const state = createMockClientState();
  seed(state);
  const { result } = renderHookWithClient(
    () => ({
      offered: useAvailableAgentClis(),
      statuses: useAgentRuntimeStatus(),
    }),
    createMockClient(state),
  );
  await waitFor(() => expect(result.current.statuses.isSuccess).toBe(true));
  return result.current.offered;
}

describe("useAvailableAgentClis", () => {
  it("offers every agent the runtime reports reaching", async () => {
    expect(await offeredAgentClis(() => {})).toEqual(AGENT_CLI_ORDER);
  });

  it("offers an agent still completing its handshake", async () => {
    expect(await offeredAgentClis(reportOpenCode("starting"))).toEqual(
      AGENT_CLI_ORDER,
    );
  });

  it("withholds an agent nothing answered for", async () => {
    expect(await offeredAgentClis(reportOpenCode("unavailable"))).toEqual(
      WITHOUT_OPENCODE,
    );
  });

  it("withholds an agent the supervisor has given up on", async () => {
    expect(await offeredAgentClis(reportOpenCode("failing"))).toEqual(
      WITHOUT_OPENCODE,
    );
  });

  it("withholds an agent nothing supervises at all", async () => {
    expect(
      await offeredAgentClis((state) => {
        state.agentRuntimeStatuses = state.agentRuntimeStatuses.filter(
          (status) => status.agentRef !== "ora-space.opencode",
        );
      }),
    ).toEqual(WITHOUT_OPENCODE);
  });

  it("withholds a built-in CLI that is missing from this machine", async () => {
    expect(
      await offeredAgentClis((state) => {
        state.agentRuntimeStatuses = state.agentRuntimeStatuses.filter(
          (status) => status.agentRef !== "ora-space.claude",
        );
      }),
    ).toEqual(
      AGENT_CLI_ORDER.filter((agentCli) => agentCli !== "ora-space.claude"),
    );
  });

  it("offers the whole catalog while the detection status is still loading", () => {
    const { result } = renderHookWithClient(
      () => useAvailableAgentClis(),
      createMockClient(createMockClientState()),
    );

    expect(result.current).toEqual(AGENT_CLI_ORDER);
  });
});
