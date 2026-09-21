import assert from "node:assert/strict";
import path from "node:path";
import { initialize } from "./run-minicloud.ts";

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
