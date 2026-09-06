import type { Plugin } from "./plugin.ts";
import {
  CHILD_PROCESS_METHODS,
  CHILD_PROCESS_NOTIFICATIONS,
  type ChildProcessExit,
  type ChildProcessSpawnResult,
  type JsonValue,
} from "./protocol/index.ts";
import { decodeBase64, encodeBase64 } from "./storage.ts";

/** Everything a spawn request carries apart from the program to run. */
interface HostChildProcessInvocation {
  args?: string[];
  /** Working directory of the child, independent of how its program was resolved. */
  cwd?: string;
  /** Added to the environment the child inherits from the host; it is not a replacement. */
  env?: Record<string, string>;
}

/**
 * Options for one subprocess the host spawns on this plugin's behalf.
 *
 * The program is named exactly one of two ways. `command` is resolved by the operating system: a
 * PATH lookup, or a path this plugin already knows. `packageCommand` is a package-relative path
 * the host joins onto this plugin's own install root, which is how a plugin runs an executable it
 * ships — a plugin is told no host path and cannot reliably compute one, and a relative `command`
 * would be resolved against a different directory depending on the platform.
 */
export type HostChildProcessOptions =
  & HostChildProcessInvocation
  & (
    | { command: string; packageCommand?: never }
    | { packageCommand: string; command?: never }
  );

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

  plugin.onNotification(CHILD_PROCESS_NOTIFICATIONS.stdout, (params) => {
    forwardChunk(processes, params, "stdoutController");
  });
  plugin.onNotification(CHILD_PROCESS_NOTIFICATIONS.stderr, (params) => {
    forwardChunk(processes, params, "stderrController");
  });
  plugin.onNotification(CHILD_PROCESS_NOTIFICATIONS.exit, (params) => {
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
      // Only the field actually supplied is sent: the host reads the two program forms as
      // mutually exclusive, and an explicit null would read as naming neither.
      const program: Record<string, string> =
        options.packageCommand === undefined
          ? { command: options.command }
          : { packageCommand: options.packageCommand };
      const result = await plugin.request(CHILD_PROCESS_METHODS.spawn, {
        ...program,
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
          discardResult(plugin.request(CHILD_PROCESS_METHODS.write, {
            processId: spawned.processId,
            bytesBase64: encodeBase64(bytes),
          })),
        closeStdin: () =>
          discardResult(
            plugin.request(CHILD_PROCESS_METHODS.closeStdin, {
              processId: spawned.processId,
            }),
          ),
        kill: () =>
          discardResult(
            plugin.request(CHILD_PROCESS_METHODS.kill, {
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
  | ChildProcessExit
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
): ChildProcessSpawnResult {
  if (
    !isRecord(result) || typeof result.processId !== "string" ||
    (typeof result.pid !== "number" && result.pid !== null)
  ) {
    throw new Error(
      `${CHILD_PROCESS_METHODS.spawn} returned an invalid result`,
    );
  }
  return { processId: result.processId, pid: result.pid as number | null };
}

async function discardResult(result: Promise<JsonValue>): Promise<void> {
  await result;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
