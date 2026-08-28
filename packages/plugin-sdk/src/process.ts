import type { Plugin } from "./plugin.ts";
import type { JsonValue } from "./protocol.ts";
import { decodeBase64, encodeBase64 } from "./storage.ts";

const CHILDPROCESS_SPAWN_METHOD = "ora/childprocess/spawn";
const CHILDPROCESS_WRITE_METHOD = "ora/childprocess/write";
const CHILDPROCESS_CLOSE_STDIN_METHOD = "ora/childprocess/closeStdin";
const CHILDPROCESS_KILL_METHOD = "ora/childprocess/kill";
const CHILDPROCESS_STDOUT_METHOD = "ora/childprocess/stdout";
const CHILDPROCESS_STDERR_METHOD = "ora/childprocess/stderr";
const CHILDPROCESS_EXIT_METHOD = "ora/childprocess/exit";

/** Options for one subprocess the host spawns on this plugin's behalf. */
export interface HostChildProcessOptions {
  command: string;
  args?: string[];
  cwd?: string;
  env?: Record<string, string>;
}

/** How one host-managed child process ended; `signal` is only ever set on Unix. */
export interface HostChildProcessExit {
  code: number | null;
  signal: number | null;
}

/**
 * One subprocess the host owns on this plugin's behalf instead of the plugin spawning it itself.
 *
 * The host, not this plugin's own sandboxed runtime, holds the OS process handle: it is killed,
 * best effort, the moment this plugin generation stops for any reason, on top of whatever this
 * object's own `kill()` requests.
 */
export interface HostChildProcess {
  readonly pid: number | undefined;
  readonly stdout: ReadableStream<Uint8Array>;
  readonly stderr: ReadableStream<Uint8Array>;
  readonly exited: Promise<HostChildProcessExit>;
  /** Writes bytes to the process's stdin. */
  write(bytes: Uint8Array): Promise<void>;
  /** Signals EOF on the process's stdin without killing it. */
  closeStdin(): Promise<void>;
  /** Requests best-effort tree-wide termination of the process. */
  kill(): Promise<void>;
}

/** Spawns subprocesses the host owns instead of this plugin's own sandboxed runtime. */
export interface HostProcesses {
  spawn(options: HostChildProcessOptions): Promise<HostChildProcess>;
}

/** One process's demultiplexed stdout/stderr sinks and its `exited` resolver. */
interface TrackedProcess {
  stdoutController: ReadableStreamDefaultController<Uint8Array> | undefined;
  stderrController: ReadableStreamDefaultController<Uint8Array> | undefined;
  resolveExit: (exit: HostChildProcessExit) => void;
}

/**
 * Builds the client for `ora/childprocess/*`: the host spawns, owns, and best-effort kills a
 * subprocess for this plugin instead of the plugin spawning one itself (which is what
 * `opencode-agent` did directly with `Deno.Command` before this existed).
 *
 * Must be called before `plugin.run()`, like `createStorage`: it registers the notification
 * handlers the host pushes, unprompted, for every process this client spawns — `stdout` and
 * `stderr` as raw byte chunks (this plugin owns any line framing) and `exit` once — all
 * demultiplexed by `processId`.
 */
export function createHostProcesses(plugin: Plugin): HostProcesses {
  const processes = new Map<string, TrackedProcess>();

  plugin.onNotification(CHILDPROCESS_STDOUT_METHOD, (params) => {
    forwardChunk(processes, params, "stdoutController");
  });
  plugin.onNotification(CHILDPROCESS_STDERR_METHOD, (params) => {
    forwardChunk(processes, params, "stderrController");
  });
  plugin.onNotification(CHILDPROCESS_EXIT_METHOD, (params) => {
    const exit = parseExitNotification(params);
    if (exit === undefined) {
      return;
    }
    const tracked = processes.get(exit.processId);
    if (tracked === undefined) {
      return;
    }
    processes.delete(exit.processId);
    tracked.stdoutController?.close();
    tracked.stderrController?.close();
    tracked.resolveExit({ code: exit.code, signal: exit.signal });
  });

  return {
    async spawn(options) {
      const result = await plugin.request(CHILDPROCESS_SPAWN_METHOD, {
        command: options.command,
        args: options.args ?? [],
        cwd: options.cwd ?? null,
        env: options.env ?? {},
      } as JsonValue);
      const spawned = parseSpawnResult(result);

      let stdoutController:
        | ReadableStreamDefaultController<Uint8Array>
        | undefined;
      const stdout = new ReadableStream<Uint8Array>({
        start(controller) {
          stdoutController = controller;
        },
      });
      let stderrController:
        | ReadableStreamDefaultController<Uint8Array>
        | undefined;
      const stderr = new ReadableStream<Uint8Array>({
        start(controller) {
          stderrController = controller;
        },
      });
      const exited = new Promise<HostChildProcessExit>((resolveExit) => {
        processes.set(spawned.processId, {
          stdoutController,
          stderrController,
          resolveExit,
        });
      });

      return {
        pid: spawned.pid ?? undefined,
        stdout,
        stderr,
        exited,
        write: (bytes) =>
          discardResult(plugin.request(CHILDPROCESS_WRITE_METHOD, {
            processId: spawned.processId,
            bytesBase64: encodeBase64(bytes),
          })),
        closeStdin: () =>
          discardResult(
            plugin.request(CHILDPROCESS_CLOSE_STDIN_METHOD, {
              processId: spawned.processId,
            }),
          ),
        kill: () =>
          discardResult(
            plugin.request(CHILDPROCESS_KILL_METHOD, {
              processId: spawned.processId,
            }),
          ),
      };
    },
  };
}

/** Decodes one `stdout`/`stderr` notification and enqueues it on the matching stream. */
function forwardChunk(
  processes: Map<string, TrackedProcess>,
  params: JsonValue,
  controllerField: "stdoutController" | "stderrController",
): void {
  if (
    !isRecord(params) || typeof params.processId !== "string" ||
    typeof params.bytesBase64 !== "string"
  ) {
    console.warn("Ignoring a malformed ora/childprocess output notification");
    return;
  }
  const tracked = processes.get(params.processId);
  // An unknown processId is not a defect: the process may already have exited and been
  // untracked, with a trailing chunk still in flight.
  tracked?.[controllerField]?.enqueue(decodeBase64(params.bytesBase64));
}

/** Validates one `exit` notification's shape. */
function parseExitNotification(
  params: JsonValue,
):
  | { processId: string; code: number | null; signal: number | null }
  | undefined {
  if (
    !isRecord(params) || typeof params.processId !== "string" ||
    (typeof params.code !== "number" && params.code !== null) ||
    (typeof params.signal !== "number" && params.signal !== null)
  ) {
    console.warn("Ignoring a malformed ora/childprocess/exit notification");
    return undefined;
  }
  return {
    processId: params.processId,
    code: params.code as number | null,
    signal: params.signal as number | null,
  };
}

/** Validates the wire shape of a `spawn` result. */
function parseSpawnResult(
  result: JsonValue,
): { processId: string; pid: number | null } {
  if (
    !isRecord(result) || typeof result.processId !== "string" ||
    (typeof result.pid !== "number" && result.pid !== null)
  ) {
    throw new Error(`${CHILDPROCESS_SPAWN_METHOD} returned an invalid result`);
  }
  return { processId: result.processId, pid: result.pid as number | null };
}

async function discardResult(result: Promise<JsonValue>): Promise<void> {
  await result;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
