import { act, render } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { PlatformProvider, type PluginInstallProgress } from "../../platform";
import { createStubPlatform } from "../../test/stub-platform";
import { usePluginOperationStore } from "../../state/stores/plugin-operation-store";
import { PluginOperationEventBridge } from "./plugin-operation-event-bridge";

afterEach(() => {
  act(() => usePluginOperationStore.setState({ activities: {} }));
});

it("records install progress independently of the marketplace page lifecycle", async () => {
  let report: ((progress: PluginInstallProgress) => void) | undefined;
  const stop = vi.fn();
  const platform = {
    ...createStubPlatform(),
    pluginMarketplace: {
      onInstallProgress: vi.fn(async (listener) => {
        report = listener;
        return stop;
      }),
    },
  };
  usePluginOperationStore.getState().begin("official/weather", "install");
  const view = render(
    <PlatformProvider adapter={platform}>
      <PluginOperationEventBridge />
    </PlatformProvider>,
  );
  await act(async () => Promise.resolve());

  act(() => {
    report?.({ pluginId: "official/weather", downloaded: 40, total: 100 });
  });

  expect(usePluginOperationStore.getState().activities).toEqual({
    "official/weather": {
      state: "pending",
      kind: "install",
      progress: {
        pluginId: "official/weather",
        downloaded: 40,
        total: 100,
      },
    },
  });
  view.unmount();
  expect(stop).toHaveBeenCalledOnce();
});
