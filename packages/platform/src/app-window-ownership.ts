const APP_WINDOW_LOCK_NAME = "ora:app-window";

/** Releases one acquired application-window ownership claim. */
export interface AppWindowOwnershipLease {
  release(): void;
}

/** Grants exclusive use of the application shell for one host window or browser tab. */
export interface AppWindowOwnershipCapability {
  acquire(options: {
    signal: AbortSignal;
    onWaiting: () => void;
  }): Promise<AppWindowOwnershipLease>;
}

/** Creates ownership for a native host that enforces one application window itself. */
export function createSingleWindowOwnership(): AppWindowOwnershipCapability {
  return {
    acquire: async ({ signal }) => {
      if (signal.aborted) throw signal.reason;
      return { release: () => undefined };
    },
  };
}

/** Creates same-origin browser-tab ownership backed by the injected Web Locks lifecycle. */
export function createWebLockWindowOwnership(
  getLockManager: () => LockManager | undefined = () => globalThis.navigator?.locks,
): AppWindowOwnershipCapability {
  return {
    acquire: ({ signal, onWaiting }) => {
      const lockManager = getLockManager();
      if (lockManager === undefined) {
        return Promise.reject(new Error("Web Locks are required to coordinate Ora browser tabs"));
      }

      return new Promise<AppWindowOwnershipLease>((resolve, reject) => {
        const holdLock = () => new Promise<void>((release) => {
          let released = false;
          resolve({
            release: () => {
              if (released) return;
              released = true;
              release();
            },
          });
        });
        // Web Locks forbids combining `ifAvailable` with an abort signal. The probe's
        // callback runs immediately, and the queued acquisition below remains abortable.
        void lockManager.request(
          APP_WINDOW_LOCK_NAME,
          { mode: "exclusive", ifAvailable: true },
          (lock) => {
            if (lock !== null) return holdLock();
            onWaiting();
            return lockManager.request(
              APP_WINDOW_LOCK_NAME,
              { mode: "exclusive", signal },
              holdLock,
            );
          },
        ).catch(reject);
      });
    },
  };
}
