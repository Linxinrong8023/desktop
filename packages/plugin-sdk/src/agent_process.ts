import { AGENT_NOT_INSTALLED, AGENT_UNUSABLE } from "./agent.ts";
import { HostRequestError, PluginMethodError } from "./plugin.ts";
import type {
  HostChildProcess,
  HostChildProcessOptions,
  HostProcesses,
} from "./process.ts";

/**
 * The ways one agent plugin can reach the CLI it drives, in the order they are tried.
 *
 * A plugin whose source is published both ways names both: it cannot know at build time whether it
 * ended up in a package that bundles the CLI for one target triple or in one that carries only the
 * plugin, and the host answers that at spawn time. A plugin whose CLI is only ever distributed on
 * its own names just `command` — omitting `packageCommand` is how "this package ships no
 * executable" is stated, rather than naming a path the package is known not to carry and relying
 * on the host to report it missing.
 */
export interface AgentProgram {
  /**
   * Package-relative path to the CLI this package ships, for a plugin that is published bundled.
   *
   * Left out entirely by a plugin that has no bundled form. It is not a guess the ladder tolerates
   * being wrong: naming a path means this package may carry an executable there, and one that
   * turns out to be present but unrunnable fails the agent outright.
   */
  packageCommand?: string;
  /**
   * Command resolved from the host's PATH when the package ships no CLI of its own, or several
   * spellings of it to try in order.
   *
   * More than one is worth naming wherever installers disagree about what they put on PATH. On
   * Windows a native install leaves `tool.exe` while npm and bun leave a `tool.cmd` shim, and the
   * host's PATH lookup appends `.exe` to an extensionless name rather than trying either — so a
   * plugin that names only the bare command finds nothing on a machine that has the CLI.
   */
  command: string | readonly string[];
}

/** Everything one agent spawn carries apart from the program to run. */
export type AgentInvocation = Omit<
  HostChildProcessOptions,
  "command" | "packageCommand"
>;

/**
 * Spawns an agent CLI, preferring the one this package ships over the user's own install.
 *
 * The ladder falls through on exactly one condition — the host reporting that the package does not
 * carry the executable at all — because that is how a package built without a bundled CLI
 * announces itself. Every other failure of a package-supplied executable is a property of the
 * package rather than of this machine: it will fail identically on every retry, so it is raised as
 * {@link AGENT_UNUSABLE}, which Ora reports once instead of retrying. A PATH lookup that finds
 * nothing is the opposite kind of failure and is raised as {@link AGENT_NOT_INSTALLED}, which Ora
 * retries quietly, so installing the CLI is enough to bring the agent up without restarting Ora.
 *
 * A program that names no `packageCommand` starts at the PATH lookup. That is a different
 * statement from a bundled path the host reports missing, even though both end up there: the
 * plugin is saying it has no bundled form at all, so nothing is asked of the host and no answer
 * about this package could change where the CLI comes from.
 */
export async function spawnAgentProcess(
  processes: HostProcesses,
  program: AgentProgram,
  invocation: AgentInvocation = {},
): Promise<HostChildProcess> {
  const packageCommand = program.packageCommand;
  if (packageCommand !== undefined) {
    try {
      return await processes.spawn({ packageCommand, ...invocation });
    } catch (error) {
      if (!isHostErrorKind(error, "package_command_missing")) {
        throw new PluginMethodError(
          AGENT_UNUSABLE,
          `the bundled agent \`${packageCommand}\` cannot run: ${
            describe(error)
          }`,
        );
      }
    }
  }
  const candidates = typeof program.command === "string"
    ? [program.command]
    : program.command;
  for (const command of candidates) {
    try {
      return await processes.spawn({ command, ...invocation });
    } catch (error) {
      // Only "this spelling is not on PATH" moves on to the next one. A candidate that resolved
      // and then failed for any other reason is a real fault, and trying further spellings would
      // bury it — or, worse, start a different CLI than the one that just failed.
      if (!isHostErrorKind(error, "program_not_found")) {
        throw error;
      }
    }
  }
  throw new PluginMethodError(
    AGENT_NOT_INSTALLED,
    `${candidates.map((command) => `\`${command}\``).join(", ")} ${
      candidates.length === 1 ? "was" : "were"
    } not found on this machine`,
  );
}

/** Reports whether a thrown value is a host request failure of one stable classification. */
function isHostErrorKind(error: unknown, kind: string): boolean {
  return error instanceof HostRequestError && error.kind === kind;
}

/** Renders any thrown value as a message safe to carry back to the host. */
function describe(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
