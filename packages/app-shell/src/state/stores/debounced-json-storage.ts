import {
  createJSONStorage,
  type PersistStorage,
  type StateStorage,
} from "zustand/middleware";

/** Default coalesce window for keystroke-heavy draft / composer parks. */
export const DEBOUNCED_PERSIST_MS = 300;

type PendingWrite = {
  value: string;
  timer: ReturnType<typeof setTimeout>;
};

const flushers = new Set<() => void>();
let lifecycleBound = false;

/** Flushes independent stores defensively so one unavailable storage cannot block the rest. */
function flushAll(): void {
  for (const flusher of flushers) {
    try {
      flusher();
    } catch {
      // Lifecycle teardown cannot surface a useful recovery UI. Keep moving so
      // every other store still gets its final durability attempt.
    }
  }
}

/**
 * Registers a storage flush so pending localStorage writes survive tab close,
 * window hide, and Electron page teardown without waiting out the debounce.
 */
function registerLifecycleFlusher(flush: () => void): void {
  flushers.add(flush);
  if (lifecycleBound || typeof window === "undefined") return;
  lifecycleBound = true;
  const run = flushAll;
  window.addEventListener("pagehide", run);
  window.addEventListener("beforeunload", run);
  document.addEventListener("visibilitychange", () => {
    if (document.visibilityState === "hidden") run();
  });
}

/**
 * Wraps `localStorage` so rapid `setItem` calls coalesce. `getItem` sees the
 * pending value so rehydrate / same-tick reads stay consistent; `removeItem`
 * and lifecycle events flush immediately for durability.
 */
export function createDebouncedStateStorage(
  debounceMs: number = DEBOUNCED_PERSIST_MS,
  getStorage: () => Storage = () => window.localStorage,
): StateStorage & { flush: () => void } {
  const pending = new Map<string, PendingWrite>();

  /** Writes every queued key synchronously and clears timers. */
  const flush = () => {
    const storage = getStorage();
    for (const [name, entry] of pending) {
      clearTimeout(entry.timer);
      try {
        storage.setItem(name, entry.value);
        // A newer write may have replaced this entry while setItem ran.
        if (pending.get(name) === entry) pending.delete(name);
      } catch {
        // Retain failed writes for a later lifecycle/manual flush. A quota or
        // security failure for one key must not strand unrelated queued keys.
      }
    }
  };

  registerLifecycleFlusher(flush);

  return {
    getItem: (name) => {
      const queued = pending.get(name);
      if (queued !== undefined) return queued.value;
      return getStorage().getItem(name);
    },
    setItem: (name, value) => {
      const previous = pending.get(name);
      if (previous !== undefined) clearTimeout(previous.timer);
      const timer = setTimeout(() => {
        const current = pending.get(name);
        if (current === undefined || current.timer !== timer) return;
        try {
          getStorage().setItem(name, value);
          pending.delete(name);
        } catch {
          // Keep the value readable from the pending map and retry it when a
          // later explicit or lifecycle flush runs.
        }
      }, debounceMs);
      pending.set(name, { value, timer });
    },
    removeItem: (name) => {
      const previous = pending.get(name);
      if (previous !== undefined) {
        clearTimeout(previous.timer);
        pending.delete(name);
      }
      getStorage().removeItem(name);
    },
    flush,
  };
}

/**
 * Zustand JSON persist storage that debounces disk writes while keeping memory
 * state immediate. Call `flushDebouncedPersistStorage` in tests before reading
 * `localStorage`, or rely on pagehide / visibility for production durability.
 */
export function createDebouncedJSONStorage<S>(
  debounceMs: number = DEBOUNCED_PERSIST_MS,
): PersistStorage<S, unknown> {
  const json = createJSONStorage<S>(() =>
    createDebouncedStateStorage(debounceMs),
  );
  if (json === undefined) {
    throw new Error("createJSONStorage returned undefined");
  }
  return json;
}

/** Drains every registered debounced persist queue (tests + rare sync needs). */
export function flushDebouncedPersistStorage(): void {
  flushAll();
}
