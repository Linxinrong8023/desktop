import type {
  MiniCloneAccepted,
  MiniCloneOperation,
  MiniCloneRequest,
  MiniErrorCode,
} from "@ora/contracts";

/** Typed application operations; tests inject handlers at the same interface used by the UI. */
export interface CloneClient {
  submit(
    input: MiniCloneRequest,
    signal: AbortSignal,
  ): Promise<MiniCloneAccepted>;
  list(signal: AbortSignal): Promise<MiniCloneOperation[]>;
}

export class HttpError extends Error {
  constructor(readonly code: MiniErrorCode) {
    super(code);
  }
}

/** Keeps transport failures separate from durable execution facts; Vite owns the local proxy target. */
export function createHttpClient(send: typeof fetch = fetch): CloneClient {
  async function request<T>(init: RequestInit): Promise<T> {
    const response = await send("/api/clones", init);
    if (!response.ok) {
      const code: MiniErrorCode =
        response.status === 400
          ? "invalidInput"
          : response.status === 409
            ? "conflict"
            : response.status === 404
              ? "notFound"
              : "unavailable";
      throw new HttpError(code);
    }
    return response.json() as Promise<T>;
  }
  return {
    submit: (input, signal) =>
      request({
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(input),
        signal,
      }),
    list: (signal) => request({ signal }),
  };
}
