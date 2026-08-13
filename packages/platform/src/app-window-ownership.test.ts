import { describe, expect, it, vi } from "vitest";
import { createWebLockWindowOwnership } from "./app-window-ownership";

describe("web lock app-window ownership", () => {
  it("transfers ownership to a waiting page after the active page releases", async () => {
    const lockManager = new TestLockManager();
    const ownership = createWebLockWindowOwnership(() => lockManager as unknown as LockManager);
    const first = await ownership.acquire({
      signal: new AbortController().signal,
      onWaiting: vi.fn(),
    });
    expect(lockManager.requests[0]).toEqual({ mode: "exclusive", ifAvailable: true });
    const onWaiting = vi.fn();
    const secondLease = ownership.acquire({
      signal: new AbortController().signal,
      onWaiting,
    });

    expect(onWaiting).toHaveBeenCalledOnce();
    first.release();
    const second = await secondLease;

    second.release();
  });
});

/** Minimal exclusive LockManager behavior needed to exercise the ownership interface. */
class TestLockManager {
  private active = false;
  private readonly waiters: Array<() => void> = [];
  readonly requests: LockOptions[] = [];

  /** Runs a lock callback immediately, with null, or after the active callback completes. */
  request(
    _name: string,
    options: LockOptions,
    callback: (lock: Lock | null) => Promise<void>,
  ): Promise<void> {
    this.requests.push(options);
    if (this.active && options.ifAvailable === true) {
      return Promise.resolve(callback(null));
    }
    if (this.active) {
      return new Promise<void>((resolve, reject) => {
        this.waiters.push(() => void this.run(callback).then(resolve, reject));
      });
    }
    return this.run(callback);
  }

  /** Holds the exclusive slot until the callback promise settles. */
  private async run(callback: (lock: Lock) => Promise<void>): Promise<void> {
    this.active = true;
    try {
      await callback({ name: "ora:app-window", mode: "exclusive" });
    } finally {
      this.active = false;
      this.waiters.shift()?.();
    }
  }
}
