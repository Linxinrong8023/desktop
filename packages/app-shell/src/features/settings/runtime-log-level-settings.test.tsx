import { createChatStore } from "@ora/chat";
import type {
  ContractsClient,
  RuntimeLogLevelStateResponse,
} from "@ora/contracts";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { appI18n } from "../../i18n/i18n-instance";
import {
  createHookWrapper,
  createTestQueryClient,
} from "../../test/hook-harness";
import {
  createMockClient,
  createMockClientState,
} from "../../test/mock-client";
import { RuntimeLogLevelSettings } from "./runtime-log-level-settings";

describe("RuntimeLogLevelSettings", () => {
  beforeEach(async () => {
    await appI18n.changeLanguage("en-US");
  });

  it("locks the selector while the initial authoritative state is loading", () => {
    const client = createMockClient(createMockClientState());
    client.runtimeLogLevel.get = vi.fn(
      () => new Promise<RuntimeLogLevelStateResponse>(() => undefined),
    );

    renderSettings(client);

    expect(screen.getByRole("combobox", { name: "Log level" })).toBeDisabled();
    expect(screen.getByText("Loading…")).toBeInTheDocument();
  });

  it.each(["Web", "Desktop"])(
    "applies a successful update through the %s contracts client",
    async () => {
      const user = userEvent.setup();
      const state = createMockClientState();
      const client = createMockClient(state);
      const setLevel = vi.spyOn(client.runtimeLogLevel, "set");
      renderSettings(client);

      const selector = await screen.findByRole("combobox", {
        name: "Log level",
      });
      await user.click(selector);
      await user.click(await screen.findByRole("option", { name: "Debug" }));

      await waitFor(() =>
        expect(setLevel).toHaveBeenCalledWith({ level: "debug" }),
      );
      await waitFor(() => expect(selector).toHaveTextContent("Debug"));
      expect(state.runtimeLogLevel.effectiveLevel).toBe("debug");
    },
  );

  it("displays an effective startup override as the selected level without exposing its source", async () => {
    const state = createMockClientState();
    state.runtimeLogLevel = {
      configuredLevel: "info",
      effectiveLevel: "trace",
      startupOverride: "trace",
    };

    renderSettings(createMockClient(state));

    const selector = await screen.findByRole("combobox", { name: "Log level" });
    await waitFor(() =>
      expect(selector).toHaveTextContent("Trace (most detailed)"),
    );
    expect(screen.queryByText(/ORA_LOG_LEVEL/)).not.toBeInTheDocument();
    expect(screen.getByText(/substantially more logs/)).toBeInTheDocument();
  });

  it("prevents duplicate submissions while an update is pending", async () => {
    const user = userEvent.setup();
    const client = createMockClient(createMockClientState());
    let resolveUpdate:
      ((value: RuntimeLogLevelStateResponse) => void) | undefined;
    client.runtimeLogLevel.set = vi.fn(
      () =>
        new Promise<RuntimeLogLevelStateResponse>((resolve) => {
          resolveUpdate = resolve;
        }),
    );
    renderSettings(client);

    const selector = await screen.findByRole("combobox", { name: "Log level" });
    await user.click(selector);
    await user.click(await screen.findByRole("option", { name: "Debug" }));

    await waitFor(() => expect(selector).toBeDisabled());
    expect(screen.getByRole("status")).toHaveTextContent("Applying log level…");
    await user.click(selector);
    expect(client.runtimeLogLevel.set).toHaveBeenCalledTimes(1);

    resolveUpdate?.({
      configuredLevel: "debug",
      effectiveLevel: "debug",
      startupOverride: null,
    });
    await waitFor(() => expect(selector).toBeEnabled());
  });

  it("retains the last authoritative selection after an update fails", async () => {
    const user = userEvent.setup();
    const client = createMockClient(createMockClientState());
    client.runtimeLogLevel.set = vi
      .fn()
      .mockRejectedValue(new Error("persistence failed"));
    renderSettings(client);

    const selector = await screen.findByRole("combobox", { name: "Log level" });
    await waitFor(() => expect(selector).toBeEnabled());
    expect(selector).toHaveTextContent("Info (recommended)");
    await user.click(selector);
    await user.click(await screen.findByRole("option", { name: "Debug" }));

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "The log level update failed. The last effective setting has been restored.",
    );
    expect(selector).toHaveTextContent("Info (recommended)");
  });
});

/** Renders the shared settings control with the same providers as the application shell. */
function renderSettings(client: ContractsClient) {
  const queryClient = createTestQueryClient();
  const Wrapper = createHookWrapper(
    client,
    queryClient,
    createChatStore(client.session),
  );
  return render(<RuntimeLogLevelSettings />, { wrapper: Wrapper });
}
