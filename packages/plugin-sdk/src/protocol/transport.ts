import { JSON_RPC_FRAME_TYPE, MAX_FRAME_LENGTH } from "./constants.ts";
import type { JsonValue } from "./json.ts";

/** Byte streams used by one plugin protocol process. */
export interface PluginTransport {
  readable: ReadableStream<Uint8Array>;
  writable: WritableStream<Uint8Array>;
  redirectConsole: boolean;
}

/** Encodes one JSON value into Ora's binary JSON-RPC frame envelope. */
export function encodeFrame(message: JsonValue): Uint8Array {
  const payload = new TextEncoder().encode(JSON.stringify(message));
  const length = payload.byteLength + 1;
  if (length > MAX_FRAME_LENGTH) {
    throw new Error(`Plugin frame exceeds ${MAX_FRAME_LENGTH} bytes`);
  }

  const frame = new Uint8Array(length + 4);
  new DataView(frame.buffer).setUint32(0, length, false);
  frame[4] = JSON_RPC_FRAME_TYPE;
  frame.set(payload, 5);
  return frame;
}

/** Decodes arbitrarily fragmented bytes into complete JSON-RPC messages. */
export async function* decodeFrames(
  readable: ReadableStream<Uint8Array>,
): AsyncGenerator<unknown> {
  let buffer = new Uint8Array();
  // Acquiring a reader keeps this portable to DOM stream typings that omit async iteration.
  const reader = readable.getReader();
  try {
    while (true) {
      const { done, value: chunk } = await reader.read();
      if (done) {
        break;
      }
      const combined = new Uint8Array(buffer.byteLength + chunk.byteLength);
      combined.set(buffer);
      combined.set(chunk, buffer.byteLength);
      buffer = combined;

      while (buffer.byteLength >= 4) {
        const length = new DataView(
          buffer.buffer,
          buffer.byteOffset,
          buffer.byteLength,
        ).getUint32(0, false);
        if (length < 1 || length > MAX_FRAME_LENGTH) {
          throw new Error(`Invalid plugin frame length ${length}`);
        }
        if (buffer.byteLength < length + 4) {
          break;
        }
        if (buffer[4] !== JSON_RPC_FRAME_TYPE) {
          throw new Error(`Unsupported plugin frame type ${buffer[4]}`);
        }

        const payload = buffer.slice(5, length + 4);
        buffer = buffer.slice(length + 4);
        yield JSON.parse(new TextDecoder().decode(payload));
      }
    }
  } finally {
    reader.releaseLock();
  }

  if (buffer.byteLength !== 0) {
    throw new Error("Plugin protocol stream ended inside a frame");
  }
}

/** Creates the production transport backed by Deno stdin and stdout. */
export function createDenoTransport(): PluginTransport {
  return {
    readable: Deno.stdin.readable,
    writable: Deno.stdout.writable,
    redirectConsole: true,
  };
}
