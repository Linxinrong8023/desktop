import {
  AGENT_NOT_INSTALLED,
  AGENT_UNUSABLE,
  createHostProcesses,
  createPlugin,
  type JsonValue,
  PluginMethodError,
  spawnAgentProcess,
} from "../src/mod.ts";
import {
  decodeFrames,
  encodeFrame,
  type PluginTransport,
} from "../src/protocol/index.ts";

/** Compares JSON-compatible values without a Node compatibility dependency. */
function assertEquals(actual: unknown, expected: unknown): void {
  const actualJson = JSON.stringify(actual);
  const expectedJson = JSON.stringify(expected);
  if (actualJson !== expectedJson) {
    throw new Error(`Expected ${expectedJson}, received ${actualJson}`);
  }
}

/** Creates paired in-memory streams for exercising the SDK without global stdio. */
function createTransportHarness(): {
  transport: PluginTransport;
  send: (message: JsonValue) => Promise<void>;
  responses: AsyncGenerator<unknown>;
} {
  const hostInput = new TransformStream<Uint8Array>();
  const pluginOutput = new TransformStream<Uint8Array>(
    undefined,
    undefined,
    new CountQueuingStrategy({ highWaterMark: Infinity }),
  );
  const inputWriter = hostInput.writable.getWriter();
  return {
    transport: {
      readable: hostInput.readable,
      writable: pluginOutput.writable,
      redirectConsole: false,
    },
    send: (message) => inputWriter.write(encodeFrame(message)),
    responses: decodeFrames(pluginOutput.readable),
  };
}

/** The program pair every test here resolves, bundled path first. */
const PROGRAM = { packageCommand: "bin/opencode", command: "opencode" };

/** Builds one host error frame answering the request `id` with a spawn failure `kind`. */
function spawnFailure(id: number, kind: string, message: string): JsonValue {
  return {
    jsonrpc: "2.0",
    id,
    error: { code: -32602, message, data: { kind } },
  };
}

Deno.test(
  "a package that ships the agent runs it without ever looking at PATH",
  async () => {
    const plugin = createPlugin();
    const processes = createHostProcesses(plugin);
    const harness = createTransportHarness();
    const run = plugin.run(harness.transport);
    await harness.responses.next();

    const spawned = spawnAgentProcess(processes, PROGRAM, {
      args: ["acp"],
      cwd: "/work",
    });

    assertEquals((await harness.responses.next()).value, {
      jsonrpc: "2.0",
      id: 1,
      method: "ora/childprocess/spawn",
      params: {
        packageCommand: "bin/opencode",
        args: ["acp"],
        cwd: "/work",
        env: {},
      },
    });
    await harness.send({
      jsonrpc: "2.0",
      id: 1,
      result: { processId: "1", pid: 4242 },
    });

    assertEquals((await spawned).pid, 4242);

    await harness.send({ jsonrpc: "2.0", method: "ora/shutdown" });
    await run;
  },
);

Deno.test(
  "a package that ships no agent falls back to the user's own install on PATH",
  async () => {
    const plugin = createPlugin();
    const processes = createHostProcesses(plugin);
    const harness = createTransportHarness();
    const run = plugin.run(harness.transport);
    await harness.responses.next();

    const spawned = spawnAgentProcess(processes, PROGRAM, { args: ["acp"] });

    await harness.responses.next();
    await harness.send(
      spawnFailure(1, "package_command_missing", "not part of this package"),
    );

    // The invocation is carried across the fallback unchanged: only the program differs.
    assertEquals((await harness.responses.next()).value, {
      jsonrpc: "2.0",
      id: 2,
      method: "ora/childprocess/spawn",
      params: { command: "opencode", args: ["acp"], cwd: null, env: {} },
    });
    await harness.send({
      jsonrpc: "2.0",
      id: 2,
      result: { processId: "1", pid: 7 },
    });

    assertEquals((await spawned).pid, 7);

    await harness.send({ jsonrpc: "2.0", method: "ora/shutdown" });
    await run;
  },
);

Deno.test(
  "a bundled agent that cannot run is terminal instead of falling back to PATH",
  async () => {
    const plugin = createPlugin();
    const processes = createHostProcesses(plugin);
    const harness = createTransportHarness();
    const run = plugin.run(harness.transport);
    await harness.responses.next();

    const failed = spawnAgentProcess(processes, PROGRAM).catch((error) =>
      error
    );

    await harness.responses.next();
    await harness.send(
      spawnFailure(1, "invalid_package_command", "not a regular package file"),
    );
    const error = await failed;

    // The next request this plugin makes still carries id 2, which is only true if no fallback
    // spawn was ever sent: a broken package must not be masked by a PATH lookup that happens to
    // succeed, because the package is what needs fixing.
    const probe = plugin.request("ora/storage/list", { path: "" }).catch(
      (probeError) => probeError,
    );
    assertEquals((await harness.responses.next()).value, {
      jsonrpc: "2.0",
      id: 2,
      method: "ora/storage/list",
      params: { path: "" },
    });
    assertEquals(
      [error instanceof PluginMethodError, error.code],
      [true, AGENT_UNUSABLE],
    );

    await harness.send({ jsonrpc: "2.0", method: "ora/shutdown" });
    await run;
    await probe;
  },
);

Deno.test(
  "a plugin with no bundled form asks the host nothing about its package",
  async () => {
    const plugin = createPlugin();
    const processes = createHostProcesses(plugin);
    const harness = createTransportHarness();
    const run = plugin.run(harness.transport);
    await harness.responses.next();

    const spawned = spawnAgentProcess(processes, { command: "codeagent" }, {
      args: ["acp"],
    });

    // The very first request is the PATH lookup: omitting packageCommand is a statement about
    // this plugin, not a question the host could answer differently.
    assertEquals((await harness.responses.next()).value, {
      jsonrpc: "2.0",
      id: 1,
      method: "ora/childprocess/spawn",
      params: { command: "codeagent", args: ["acp"], cwd: null, env: {} },
    });
    await harness.send({
      jsonrpc: "2.0",
      id: 1,
      result: { processId: "1", pid: 99 },
    });

    assertEquals((await spawned).pid, 99);

    await harness.send({ jsonrpc: "2.0", method: "ora/shutdown" });
    await run;
  },
);

Deno.test(
  "every PATH spelling is tried in order before the agent counts as missing",
  async () => {
    const plugin = createPlugin();
    const processes = createHostProcesses(plugin);
    const harness = createTransportHarness();
    const run = plugin.run(harness.transport);
    await harness.responses.next();

    const spawned = spawnAgentProcess(processes, {
      packageCommand: "bin/codeagent",
      command: ["codeagent.exe", "codeagent.cmd", "codeagent"],
    });

    await harness.responses.next();
    await harness.send(
      spawnFailure(1, "package_command_missing", "not part of this package"),
    );
    await harness.responses.next();
    await harness.send(spawnFailure(2, "program_not_found", "no .exe"));

    // The second spelling is what a machine with an npm-installed CLI actually has, and the run
    // must reach it rather than stopping at the first miss.
    assertEquals((await harness.responses.next()).value, {
      jsonrpc: "2.0",
      id: 3,
      method: "ora/childprocess/spawn",
      params: { command: "codeagent.cmd", args: [], cwd: null, env: {} },
    });
    await harness.send({
      jsonrpc: "2.0",
      id: 3,
      result: { processId: "1", pid: 11 },
    });

    assertEquals((await spawned).pid, 11);

    await harness.send({ jsonrpc: "2.0", method: "ora/shutdown" });
    await run;
  },
);

Deno.test(
  "an agent missing from PATH is reported as not installed so Ora keeps retrying",
  async () => {
    const plugin = createPlugin();
    const processes = createHostProcesses(plugin);
    const harness = createTransportHarness();
    const run = plugin.run(harness.transport);
    await harness.responses.next();

    const failed = spawnAgentProcess(processes, PROGRAM).catch((error) =>
      error
    );

    await harness.responses.next();
    await harness.send(
      spawnFailure(1, "package_command_missing", "not part of this package"),
    );
    await harness.responses.next();
    await harness.send(
      spawnFailure(2, "program_not_found", "program not found"),
    );
    const error = await failed;

    assertEquals(
      [error instanceof PluginMethodError, error.code],
      [true, AGENT_NOT_INSTALLED],
    );

    await harness.send({ jsonrpc: "2.0", method: "ora/shutdown" });
    await run;
  },
);
