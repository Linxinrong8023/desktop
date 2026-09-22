import assert from "node:assert/strict";
import path from "node:path";
import { controllerArguments, initialize } from "./run-minicloud.ts";

Deno.test(
  "minicloud initialization preserves edited configuration and existing checkout files",
  async () => {
    const temporary = await Deno.makeTempDir({ prefix: "mc-" });
    const root = path.join(temporary, "dev");
    try {
      await initialize(root);
      const config = path.join(root, "config", "client.json");
      await Deno.writeTextFile(config, '{"port":5190}\n');
      const marker = path.join(root, "repositories", "user-file");
      await Deno.writeTextFile(marker, "preserve");
      await Deno.chmod(root, 0o775);
      await initialize(root);
      assert.equal((await Deno.stat(root)).mode! & 0o777, 0o775);
      assert.equal(await Deno.readTextFile(config), '{"port":5190}\n');
      assert.equal(await Deno.readTextFile(marker), "preserve");
      const node = JSON.parse(
        await Deno.readTextFile(path.join(root, "config", "node.json")),
      );
      assert.deepEqual(node.node, {
        home_directory: path.join(root, "node"),
        identity: { Require: "minicloud-node" },
        repositories: [],
      });
      assert.equal(node.process.host_directory, path.join(root, "p"));
      await assert.rejects(
        Deno.stat(path.join(root, "p")),
        Deno.errors.NotFound,
      );
      assert.equal((await Deno.stat(config)).mode! & 0o777, 0o600);
    } finally {
      await Deno.remove(temporary, { recursive: true });
    }
  },
);

Deno.test(
  "minicloud initialization rejects a symlink rather than modifying its target",
  async () => {
    const temporary = await Deno.makeTempDir({ prefix: "mc-" });
    try {
      const target = path.join(temporary, "outside");
      await Deno.mkdir(target, { mode: 0o700 });
      const root = path.join(temporary, "dev");
      await Deno.symlink(target, root);
      await assert.rejects(initialize(root), /private directory/);
      const entries = [];
      for await (const entry of Deno.readDir(target)) entries.push(entry.name);
      assert.deepEqual(entries, []);
    } finally {
      await Deno.remove(temporary, { recursive: true });
    }
  },
);

Deno.test(
  "minicloud initialization hosts the launcher's Node and keeps the Controller API on loopback",
  async () => {
    const temporary = await Deno.makeTempDir({ prefix: "mc-" });
    const root = path.join(temporary, "dev");
    try {
      await initialize(root);
      const config = path.join(root, "config");
      const controller = JSON.parse(
        await Deno.readTextFile(path.join(config, "controller.json")),
      );
      assert.deepEqual(controller.api, { node_id: "minicloud-node" });
      assert.equal(controller.controller.controller_id, "minicloud-controller");
      assert.deepEqual(controller.controller.nodes, [
        {
          node_id: "minicloud-node",
          endpoint: path.join(root, "node", "control.sock"),
        },
      ]);
      assert.equal(
        controller.single_node.node_config,
        path.join(config, "node.json"),
      );
      assert.equal(
        controller.single_node.node_executable.endsWith(
          "/target/debug/ora-node",
        ),
        true,
      );
      const node = JSON.parse(
        await Deno.readTextFile(path.join(config, "node.json")),
      );
      assert.equal(node.ipc.controller_id, controller.controller.controller_id);
      assert.equal(node.ipc.endpoint, controller.controller.nodes[0].endpoint);
      const client = JSON.parse(
        await Deno.readTextFile(path.join(config, "client.json")),
      );
      assert.deepEqual(client, { port: 5174, controllerPort: 4820 });
      const args = controllerArguments(
        path.join(config, "controller.json"),
        client.controllerPort,
      );
      assert.equal(args.includes("--single-node"), true);
      assert.deepEqual(
        args.slice(args.indexOf("--host"), args.indexOf("--host") + 2),
        ["--host", "127.0.0.1"],
      );
      assert.deepEqual(
        args.slice(args.indexOf("--port"), args.indexOf("--port") + 2),
        ["--port", "4820"],
      );
    } finally {
      await Deno.remove(temporary, { recursive: true });
    }
  },
);
