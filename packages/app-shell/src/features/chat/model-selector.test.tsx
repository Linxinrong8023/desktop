import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { TooltipProvider } from "@ora/ui";
import { PlatformProvider } from "../../platform";
import { describe, expect, it, beforeEach } from "vitest";
import { AppI18nProvider } from "../../i18n/i18n";
import {
  createHookWrapper,
  createTestQueryClient,
} from "../../test/hook-harness";
import { createStubPlatform } from "../../test/stub-platform";
import { createChatStore } from "@ora/chat";
import {
  createMockClient,
  createMockClientState,
  type MockClientState,
} from "../../test/mock-client";
import { useWorkspaceSelectionStore } from "../../state/stores/workspace-selection-store";
import {
  useSettingsStore,
  DEFAULT_SETTINGS,
} from "../../state/stores/settings-store";
import { usePendingAgentStore } from "../../state/stores/pending-agent-store";
import type { AgentStatus } from "@ora/contracts";
import { ModelSelector } from "./model-selector";

beforeEach(() => {
  useWorkspaceSelectionStore.getState().clearSelection();
  useSettingsStore.setState({ settings: DEFAULT_SETTINGS });
  usePendingAgentStore.setState({ selections: {} });
});

/** Replaces what the runtime reports about OpenCode, leaving every other agent detected. */
function reportOpenCode(status: AgentStatus) {
  return (state: MockClientState) => {
    const entry = state.agentRuntimeStatuses.find(
      (candidate) => candidate.agentRef === "ora-space.opencode",
    );
    entry!.status = status;
  };
}

function renderModelSelector(
  seed: (state: MockClientState) => void = () => {},
) {
  const state = createMockClientState();
  seed(state);
  const client = createMockClient(state);
  const chatStore = createChatStore(client.session);
  const Wrapper = createHookWrapper(client, createTestQueryClient(), chatStore);
  render(
    <Wrapper>
      <AppI18nProvider>
        <PlatformProvider adapter={createStubPlatform()}>
          <TooltipProvider>
            <ModelSelector />
          </TooltipProvider>
        </PlatformProvider>
      </AppI18nProvider>
    </Wrapper>,
  );
}

/** The collapsed trigger, which names the agent this surface is currently on. */
function picker() {
  return screen.getByRole("button", { name: /选择模型|Select model/ });
}

/**
 * Opens the picker, clicks the named agent, then closes the menu.
 *
 * Choosing an agent deliberately leaves the menu open, so the same label is on
 * screen twice until it is dismissed. Closing here keeps each assertion about
 * what the trigger settled on rather than what the open list still offers.
 */
async function pickAgent(
  user: ReturnType<typeof userEvent.setup>,
  agentLabel: RegExp,
) {
  await user.click(picker());
  const menu = await screen.findByRole("menu");
  await user.click(within(menu).getByText(agentLabel));
  await user.keyboard("{Escape}");
}

describe("ModelSelector agent isolation across not-yet-started chats", () => {
  it("keeps one task's picked agent stable while another task's pick changes", async () => {
    const user = userEvent.setup();
    renderModelSelector();

    act(() => useWorkspaceSelectionStore.getState().selectTask("t1", "p1"));
    await pickAgent(user, /Claude Code/);
    expect(within(picker()).getByText("Claude Code")).not.toBeNull();

    act(() => useWorkspaceSelectionStore.getState().selectTask("t2", "p1"));
    await pickAgent(user, /OpenCode/);
    expect(within(picker()).getByText("OpenCode")).not.toBeNull();

    act(() => useWorkspaceSelectionStore.getState().selectTask("t1", "p1"));
    expect(within(picker()).getByText("Claude Code")).not.toBeNull();

    act(() => useWorkspaceSelectionStore.getState().selectTask("t2", "p1"));
    expect(within(picker()).getByText("OpenCode")).not.toBeNull();

    act(() => useWorkspaceSelectionStore.getState().selectTask("t1", "p1"));
    expect(within(picker()).getByText("Claude Code")).not.toBeNull();
  });
});

/**
 * Opens the picker and returns its agent list, which keeps re-rendering as queries settle.
 *
 * Assertions are made against this element rather than by reopening the menu: which agents are
 * offered depends on the installed-plugin snapshot, and the loading answer is the whole catalog.
 */
async function openAgentList(user: ReturnType<typeof userEvent.setup>) {
  await user.click(picker());
  return await screen.findByRole("menu");
}

describe("ModelSelector agent availability", () => {
  it("offers every agent the runtime reports reaching", async () => {
    const user = userEvent.setup();
    renderModelSelector();

    act(() => useWorkspaceSelectionStore.getState().selectTask("t1", "p1"));
    const menu = await openAgentList(user);

    await waitFor(() =>
      expect(within(menu).queryByText("OpenCode")).not.toBeNull(),
    );
    expect(within(menu).queryByText("Claude Code")).not.toBeNull();
  });

  it("withholds an agent whose runtime nothing answered for", async () => {
    const user = userEvent.setup();
    renderModelSelector(reportOpenCode("unavailable"));

    act(() => useWorkspaceSelectionStore.getState().selectTask("t1", "p1"));
    const menu = await openAgentList(user);

    await waitFor(() =>
      expect(within(menu).queryByText("OpenCode")).toBeNull(),
    );
    expect(within(menu).queryByText("Claude Code")).not.toBeNull();
  });

  it("withholds an agent nothing supervises at all", async () => {
    const user = userEvent.setup();
    renderModelSelector((state) => {
      state.agentRuntimeStatuses = state.agentRuntimeStatuses.filter(
        (status) => status.agentRef !== "ora-space.opencode",
      );
    });

    act(() => useWorkspaceSelectionStore.getState().selectTask("t1", "p1"));
    const menu = await openAgentList(user);

    await waitFor(() =>
      expect(within(menu).queryByText("OpenCode")).toBeNull(),
    );
  });

  it("moves a surface off a stored default the installation can no longer reach", async () => {
    renderModelSelector(reportOpenCode("unavailable"));

    act(() => useWorkspaceSelectionStore.getState().selectTask("t1", "p1"));

    // The stored default names OpenCode, which the runtime cannot reach here, so
    // the surface must settle on the first agent that is genuinely available.
    await waitFor(() =>
      expect(within(picker()).queryByText("NGA")).not.toBeNull(),
    );
  });
});
