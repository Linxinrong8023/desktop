import type { MiniCloneRequest } from "@ora/contracts";

const key = "ora.minicloud.pending-clone.v1";

/** Only an unresolved request is retained here; Controller remains the source of operation history. */
export function readPending(storage: Storage): MiniCloneRequest | null {
  const raw = storage.getItem(key);
  if (!raw) return null;
  try {
    const value: unknown = JSON.parse(raw);
    if (
      typeof value === "object" &&
      value !== null &&
      "requestId" in value &&
      typeof value.requestId === "string" &&
      "repository" in value &&
      typeof value.repository === "string" &&
      "branch" in value &&
      typeof value.branch === "string"
    ) {
      return {
        requestId: value.requestId,
        repository: value.repository,
        branch: value.branch,
      };
    }
  } catch {
    /* Invalid browser data is not an execution record. */
  }
  throw new Error("pending request is unreadable");
}

/** Persists identity before HTTP dispatch so response loss or reload cannot create a fresh execution. */
export function writePending(
  storage: Storage,
  input: MiniCloneRequest | null,
): void {
  if (input) storage.setItem(key, JSON.stringify(input));
  else storage.removeItem(key);
}
