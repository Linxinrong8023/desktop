import {
  createHostProcesses,
  createPlugin,
  type JsonValue,
} from "../src/mod.ts";
import {
  decodeFrames,
  encodeFrame,
  type PluginTransport,
} from "../src/protocol.ts";

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
  // Unbounded queuing lets a plugin await a request frame write before the harness reads it,
  // the way a real stdio pipe would.
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

/** Base64 of `"hi"`, used across tests as one arbitrary chunk of process output. */
const HI_BASE64 = "aGk=";

Deno.test(
  "spawn sends command/args/cwd/env and resolves pid from the host result",
  async () => {
    const plugin = createPlugin();
    const processes = createHostProcesses(plugin);
    const harness = createTransportHarness();
    const run = plugin.run(harness.transport);
    await harness.responses.next();

    const spawned = processes.spawn({
      command: "opencode",
      args: ["acp", "--cwd", "/work"],
      cwd: "/work",
      env: { FOO: "bar" },
    });

    assertEquals((await harness.responses.next()).value, {
      jsonrpc: "2.0",
      id: 1,
      method: "ora/childprocess/spawn",
      params: {
        command: "opencode",
        args: ["acp", "--cwd", "/work"],
        cwd: "/work",
        env: { FOO: "bar" },
      },
    });
    await harness.send({
      jsonrpc: "2.0",
      id: 1,
      result: { processId: "1", pid: 4242 },
    });

    const process = await spawned;
    assertEquals(process.pid, 4242);

    await harness.send({ jsonrpc: "2.0", method: "ora/shutdown" });
    await run;
  },
);

Deno.test(
  "spawn omits args/cwd/env with the documented defaults",
  async () => {
    const plugin = createPlugin();
    const processes = createHostProcesses(plugin);
    const harness = createTransportHarness();
    const run = plugin.run(harness.transport);
    await harness.responses.next();

    const spawned = processes.spawn({ command: "opencode" }).catch(
      (error) => error,
    );

    assertEquals((await harness.responses.next()).value, {
      jsonrpc: "2.0",
      id: 1,
      method: "ora/childprocess/spawn",
      params: { command: "opencode", args: [], cwd: null, env: {} },
    });
    await harness.send({
      jsonrpc: "2.0",
      id: 1,
      result: { processId: "1", pid: null },
    });

    const process = await spawned;
    assertEquals(process.pid, undefined);

    await harness.send({ jsonrpc: "2.0", method: "ora/shutdown" });
    await run;
  },
);

Deno.test(
  "write, closeStdin, and kill send the process's id and base64 bytes",
  async () => {
    const plugin = createPlugin();
    const processes = createHostProcesses(plugin);
    const harness = createTransportHarness();
    const run = plugin.run(harness.transport);
    await harness.responses.next();

    const spawned = processes.spawn({ command: "opencode" });
    await harness.responses.next();
    await harness.send({
      jsonrpc: "2.0",
      id: 1,
      result: { processId: "7", pid: 1 },
    });
    const process = await spawned;

    const wrote = process.write(new TextEncoder().encode("hi"));
    assertEquals((await harness.responses.next()).value, {
      jsonrpc: "2.0",
      id: 2,
      method: "ora/childprocess/write",
      params: { processId: "7", bytesBase64: HI_BASE64 },
    });
    await harness.send({ jsonrpc: "2.0", id: 2, result: {} });
    await wrote;

    const closed = process.closeStdin();
    assertEquals((await harness.responses.next()).value, {
      jsonrpc: "2.0",
      id: 3,
      method: "ora/childprocess/closeStdin",
      params: { processId: "7" },
    });
    await harness.send({ jsonrpc: "2.0", id: 3, result: {} });
    await closed;

    const killed = process.kill();
    assertEquals((await harness.responses.next()).value, {
      jsonrpc: "2.0",
      id: 4,
      method: "ora/childprocess/kill",
      params: { processId: "7" },
    });
    await harness.send({ jsonrpc: "2.0", id: 4, result: {} });
    await killed;

    await harness.send({ jsonrpc: "2.0", method: "ora/shutdown" });
    await run;
  },
);

Deno.test(
  "stdout and stderr notifications are demultiplexed by processId, and exit closes both streams",
  async () => {
    const plugin = createPlugin();
    const processes = createHostProcesses(plugin);
    const harness = createTransportHarness();
    const run = plugin.run(harness.transport);
    await harness.responses.next();

    // Two concurrent processes, spawned in one order but resolved out of order, prove
    // demultiplexing is by processId rather than by spawn call order.
    const first = processes.spawn({ command: "one" });
    const second = processes.spawn({ command: "two" });
    await harness.responses.next();
    await harness.responses.next();
    await harness.send({
      jsonrpc: "2.0",
      id: 2,
      result: { processId: "second", pid: 2 },
    });
    await harness.send({
      jsonrpc: "2.0",
      id: 1,
      result: { processId: "first", pid: 1 },
    });
    const firstProcess = await first;
    const secondProcess = await second;

    await harness.send({
      jsonrpc: "2.0",
      method: "ora/childprocess/stdout",
      params: { processId: "second", bytesBase64: HI_BASE64 },
    });
    await harness.send({
      jsonrpc: "2.0",
      method: "ora/childprocess/stderr",
      params: { processId: "first", bytesBase64: HI_BASE64 },
    });

    const secondStdoutReader = secondProcess.stdout.getReader();
    assertEquals(
      [...(await secondStdoutReader.read()).value ?? []],
      [104, 105],
    );
    const firstStderrReader = firstProcess.stderr.getReader();
    assertEquals(
      [...(await firstStderrReader.read()).value ?? []],
      [104, 105],
    );

    // Neither process has exited yet: reads beyond the single enqueued chunk must not resolve.
    const firstStdoutReader = firstProcess.stdout.getReader();
    const pendingRead = firstStdoutReader.read().then(() => "resolved");
    const raced = await Promise.race([
      pendingRead,
      new Promise((resolve) => setTimeout(() => resolve("pending"), 20)),
    ]);
    assertEquals(raced, "pending");

    await harness.send({
      jsonrpc: "2.0",
      method: "ora/childprocess/exit",
      params: { processId: "first", code: 0, signal: null },
    });
    assertEquals(await firstProcess.exited, { code: 0, signal: null });
    assertEquals(await pendingRead, "resolved");
    assertEquals((await firstStdoutReader.read()).done, true);

    await harness.send({ jsonrpc: "2.0", method: "ora/shutdown" });
    await run;
  },
);

Deno.test(
  "an exit or output notification for an unknown processId is ignored rather than crashing the plugin",
  async () => {
    const plugin = createPlugin();
    createHostProcesses(plugin);
    const harness = createTransportHarness();
    const run = plugin.run(harness.transport);
    await harness.responses.next();

    await harness.send({
      jsonrpc: "2.0",
      method: "ora/childprocess/stdout",
      params: { processId: "ghost", bytesBase64: HI_BASE64 },
    });
    await harness.send({
      jsonrpc: "2.0",
      method: "ora/childprocess/exit",
      params: { processId: "ghost", code: 0, signal: null },
    });

    // The connection is still alive: a normal request still round-trips.
    const list = plugin.request("ora/storage/list", { path: "" });
    assertEquals((await harness.responses.next()).value, {
      jsonrpc: "2.0",
      id: 1,
      method: "ora/storage/list",
      params: { path: "" },
    });
    await harness.send({
      jsonrpc: "2.0",
      id: 1,
      result: { entries: [] },
    });
    assertEquals(await list, { entries: [] });

    await harness.send({ jsonrpc: "2.0", method: "ora/shutdown" });
    await run;
  },
);
