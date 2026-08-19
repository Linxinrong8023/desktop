import { createChatStore } from "@ora/chat";
import type { ContractsClient, DeveloperModeResponse } from "@ora/contracts";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { appI18n } from "../../i18n/i18n-instance";
import { useDeveloperMode } from "../../state/hooks/use-developer-mode";
import {
  createHookWrapper,
  createTestQueryClient,
} from "../../test/hook-harness";
import {
  createMockClient,
  createMockClientState,
} from "../../test/mock-client";
import { DeveloperModeSettings } from "./developer-mode-settings";

describe("DeveloperModeSettings", () => {
  beforeEach(async () => {
    await appI18n.changeLanguage("en-US");
  });

  it("keeps the switch disabled while the authoritative value is loading", () => {
    const client = createMockClient(createMockClientState());
    client.developerMode.get = vi.fn(
      () => new Promise<DeveloperModeResponse>(() => undefined),
    );

    renderSettings(client);

    expect(
      screen.getByRole("switch", { name: "Developer mode" }),
    ).toHaveAttribute("aria-disabled", "true");
    expect(screen.getByRole("status")).toHaveTextContent(
      "Loading developer mode…",
    );
  });

  it.each(["Web", "Desktop"])(
    "persists a successful update through the %s contracts client",
    async () => {
      const user = userEvent.setup();
      const state = createMockClientState();
      const client = createMockClient(state);
      const setDeveloperMode = vi.spyOn(client.developerMode, "set");
      renderSettings(client);

      const toggle = await screen.findByRole("switch", {
        name: "Developer mode",
      });
      await waitFor(() => expect(toggle).toBeEnabled());
      await user.click(toggle);

      await waitFor(() =>
        expect(setDeveloperMode).toHaveBeenCalledWith({ enabled: true }),
      );
      await waitFor(() => expect(toggle).toBeChecked());
      expect(state.developerMode).toEqual({ enabled: true });
    },
  );

  it("retains the last authoritative value and prevents duplicate pending submissions", async () => {
    const user = userEvent.setup();
    const client = createMockClient(createMockClientState());
    let rejectUpdate: ((reason: Error) => void) | undefined;
    client.developerMode.set = vi.fn(
      () =>
        new Promise<DeveloperModeResponse>((_resolve, reject) => {
          rejectUpdate = reject;
        }),
    );
    renderSettings(client);

    const toggle = await screen.findByRole("switch", {
      name: "Developer mode",
    });
    await waitFor(() => expect(toggle).toBeEnabled());
    await user.click(toggle);
    await waitFor(() =>
      expect(toggle).toHaveAttribute("aria-disabled", "true"),
    );
    await user.click(toggle);
    expect(client.developerMode.set).toHaveBeenCalledTimes(1);

    rejectUpdate?.(new Error("persistence failed"));
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "last effective setting",
    );
    expect(toggle).not.toBeChecked();
  });

  it("keeps developer mode unavailable after a read failure and supports retry", async () => {
    const user = userEvent.setup();
    const client = createMockClient(createMockClientState());
    client.developerMode.get = vi
      .fn()
      .mockRejectedValueOnce(new Error("read failed"))
      .mockResolvedValueOnce({ enabled: false });
    renderSettings(client);

    const toggle = screen.getByRole("switch", { name: "Developer mode" });
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "could not be loaded",
    );
    expect(toggle).toHaveAttribute("aria-disabled", "true");
    await user.click(screen.getByRole("button", { name: "Retry" }));
    await waitFor(() => expect(toggle).toBeEnabled());
    expect(client.developerMode.get).toHaveBeenCalledTimes(2);
  });
});

/** Renders the switch from the real hook so query and mutation behavior stay covered together. */
function renderSettings(client: ContractsClient) {
  const queryClient = createTestQueryClient();
  const Wrapper = createHookWrapper(
    client,
    queryClient,
    createChatStore(client.session),
  );

  function Harness() {
    return <DeveloperModeSettings controller={useDeveloperMode()} />;
  }

  return render(<Harness />, { wrapper: Wrapper });
}
