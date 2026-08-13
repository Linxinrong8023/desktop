import { render, screen, waitFor } from "@testing-library/react";
import { createChatStore } from "@ora/chat";
import type { AppEvent } from "@ora/contracts";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AppEventGate } from "./state/app-event-gate";
import { createMockClient, createMockClientState } from "./test/mock-client";
import { createHookWrapper, createTestQueryClient } from "./test/hook-harness";

afterEach(() => {
  vi.useRealTimers();
});

/** Waits until the stream is cancelled without adding a second reconnect source to the test. */
function waitForAbort(signal: AbortSignal | undefined): Promise<void> {
  return new Promise((resolve) => {
    if (signal === undefined || signal.aborted) {
      resolve();
      return;
    }
    signal.addEventListener("abort", () => resolve(), { once: true });
  });
}

describe("AppEventGate", () => {
  it("lets a waiting page enter after the active page releases ownership", async () => {
    const client = createMockClient(createMockClientState());
    client.appEvents.watch = async function* (_request, options): AsyncGenerator<AppEvent> {
      yield { type: "ready" };
      await waitForAbort(options?.signal);
    };
    const ownership = createTestAppWindowOwnership();
    const FirstWrapper = createHookWrapper(client, createTestQueryClient(), createChatStore(client.session));
    const SecondWrapper = createHookWrapper(client, createTestQueryClient(), createChatStore(client.session));
    const first = render(
      <FirstWrapper>
        <AppEventGate client={client} ownership={ownership}>
          <div data-testid="first-page">first page</div>
        </AppEventGate>
      </FirstWrapper>,
    );
    const second = render(
      <SecondWrapper>
        <AppEventGate client={client} ownership={ownership}>
          <div data-testid="second-page">second page</div>
        </AppEventGate>
      </SecondWrapper>,
    );

    await waitFor(() => expect(screen.getByTestId("first-page")).toBeInTheDocument());
    expect(screen.queryByTestId("second-page")).not.toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "应用已在其他页面打开" })).toBeInTheDocument();

    first.unmount();
    await waitFor(() => expect(screen.getByTestId("second-page")).toBeInTheDocument());

    second.unmount();
  });
});

/** Creates an exclusive ownership adapter whose waiters acquire in request order. */
function createTestAppWindowOwnership() {
  let releaseActive: (() => void) | undefined;
  const waiters: Array<() => void> = [];

  return {
    acquire: ({
      signal,
      onWaiting,
    }: {
      signal: AbortSignal;
      onWaiting: () => void;
    }) => new Promise<{ release(): void }>((resolve, reject) => {
      const acquire = () => {
        const release = () => {
          if (releaseActive !== release) return;
          releaseActive = undefined;
          waiters.shift()?.();
        };
        releaseActive = release;
        resolve({ release });
      };
      if (signal.aborted) {
        reject(signal.reason);
      } else if (releaseActive === undefined) {
        acquire();
      } else {
        onWaiting();
        waiters.push(acquire);
      }
    }),
  };
}

describe("AppEventGate reconnect behavior", () => {
  it("refetches and backs off after a stream ends, then resets after Ready", async () => {
    const client = createMockClient(createMockClientState());
    let attempts = 0;
    client.appEvents.watch = async function* (_request, options): AsyncGenerator<AppEvent> {
      attempts += 1;
      yield { type: "ready" };
      if (attempts === 1) return;
      await waitForAbort(options?.signal);
    };
    const queryClient = createTestQueryClient();
    const refetch = vi.spyOn(queryClient, "refetchQueries");
    const Wrapper = createHookWrapper(client, queryClient, createChatStore(client.session));
    const { unmount } = render(
      <Wrapper>
        <AppEventGate client={client} ownership={createTestAppWindowOwnership()}>
          <div data-testid="business-content">business workload</div>
        </AppEventGate>
      </Wrapper>,
    );

    await waitFor(() => expect(refetch).toHaveBeenCalledTimes(2));
    expect(attempts).toBe(1);
    await new Promise((resolve) => setTimeout(resolve, 100));
    expect(attempts).toBe(1);
    await waitFor(() => expect(attempts).toBe(2), { timeout: 2_500 });
    expect(refetch).toHaveBeenCalledTimes(3);

    unmount();
  });
});
