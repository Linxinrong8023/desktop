import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import {
  ENABLE_LATENCY,
  INSTALL_LATENCY,
  simulatedLatencyMs,
  usePluginInstallStore,
} from "./plugin-install-store";

beforeEach(() => {
  usePluginInstallStore.setState({
    installedIds: [],
    disabledIds: [],
    pendingInstallIds: [],
    pendingEnableIds: [],
  });
});

describe("simulatedLatencyMs", () => {
  it("spans exactly centre ± jitter", () => {
    expect(simulatedLatencyMs(INSTALL_LATENCY, () => 0)).toBe(3141.5 - 618);
    expect(simulatedLatencyMs(INSTALL_LATENCY, () => 0.5)).toBe(3141.5);
    expect(simulatedLatencyMs(INSTALL_LATENCY, () => 1)).toBe(3141.5 + 618);
    expect(simulatedLatencyMs(ENABLE_LATENCY, () => 0)).toBe(618 - 272);
    expect(simulatedLatencyMs(ENABLE_LATENCY, () => 1)).toBe(618 + 272);
  });
});

describe("usePluginInstallStore", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("holds the install pending until the simulated round-trip elapses", async () => {
    const settled = usePluginInstallStore.getState().toggleInstalled("figma");

    expect(usePluginInstallStore.getState()).toMatchObject({
      installedIds: [],
      pendingInstallIds: ["figma"],
    });

    await vi.advanceTimersByTimeAsync(
      INSTALL_LATENCY.centerMs + INSTALL_LATENCY.jitterMs,
    );
    await settled;

    expect(usePluginInstallStore.getState()).toMatchObject({
      installedIds: ["figma"],
      pendingInstallIds: [],
    });
  });

  it("ignores a second toggle while the first is still running", async () => {
    const first = usePluginInstallStore.getState().toggleInstalled("figma");
    await usePluginInstallStore.getState().toggleInstalled("figma");

    expect(usePluginInstallStore.getState().pendingInstallIds).toEqual([
      "figma",
    ]);

    await vi.advanceTimersByTimeAsync(
      INSTALL_LATENCY.centerMs + INSTALL_LATENCY.jitterMs,
    );
    await first;

    // The re-entrant call was dropped rather than queued, so the plugin installs once.
    expect(usePluginInstallStore.getState().installedIds).toEqual(["figma"]);
  });

  it("holds the enable toggle pending on its own shorter round-trip", async () => {
    const settled = usePluginInstallStore.getState().toggleEnabled("figma");

    expect(usePluginInstallStore.getState()).toMatchObject({
      disabledIds: [],
      pendingEnableIds: ["figma"],
    });

    await vi.advanceTimersByTimeAsync(
      ENABLE_LATENCY.centerMs + ENABLE_LATENCY.jitterMs,
    );
    await settled;

    expect(usePluginInstallStore.getState()).toMatchObject({
      disabledIds: ["figma"],
      pendingEnableIds: [],
    });
  });
});
