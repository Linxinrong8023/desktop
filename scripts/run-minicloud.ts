import path from "node:path";
import { fileURLToPath } from "node:url";

const workspace = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
);
const delay = (ms: number) =>
  new Promise<void>((resolve) => setTimeout(resolve, ms));

/** Creates only missing private directories; existing files, links and permissions are never repaired. */
async function directory(name: string): Promise<void> {
  try {
    await Deno.mkdir(name, { mode: 0o700 });
  } catch (error) {
    if (!(error instanceof Deno.errors.AlreadyExists)) throw error;
  }
  const info = await Deno.lstat(name);
  if (!info.isDirectory || info.isSymlink || info.uid !== Deno.uid()) {
    throw new Error(`Expected a private directory owned by this user: ${name}`);
  }
}

/** Writes defaults exactly once, preserving deployment edits and refusing symbolic links. */
async function defaultFile(name: string, value: string): Promise<void> {
  try {
    await Deno.writeTextFile(name, value, { createNew: true, mode: 0o600 });
  } catch (error) {
    if (!(error instanceof Deno.errors.AlreadyExists)) throw error;
  }
  const info = await Deno.lstat(name);
  if (!info.isFile || info.isSymlink || info.uid !== Deno.uid()) {
    throw new Error(
      `Expected a private regular file owned by this user: ${name}`,
    );
  }
}

/** Initializes explicit deployment inputs without creating the host's exclusively-owned state directory. */
export async function initialize(root: string): Promise<void> {
  await directory(root);
  for (const name of [
    "config",
    "bin",
    "node",
    "controller",
    "repositories",
    "home",
    "vite",
  ]) {
    await directory(path.join(root, name));
  }
  // The short host name leaves room for scopes/<uuid>/control.sock in sockaddr_un.
  const host = path.join(root, "p");
  if (
    new TextEncoder().encode(
      path.join(
        host,
        "scopes",
        "00000000-0000-0000-0000-000000000001",
        "control.sock",
      ),
    ).length >= 108
  ) {
    throw new Error(
      `State path is too long for minicloud Unix sockets: ${root}`,
    );
  }
  const node = path.join(root, "node");
  const controller = path.join(root, "controller");
  const config = path.join(root, "config");
  const timezone = Intl.DateTimeFormat().resolvedOptions().timeZone;
  const json = (value: unknown) => `${JSON.stringify(value, null, 2)}\n`;
  await defaultFile(
    path.join(config, "clone.gitconfig"),
    "# Non-interactive clone deployment configuration. Add credentials/CA settings here.\n",
  );
  await defaultFile(
    path.join(config, "node.json"),
    json({
      node: {
        home_directory: node,
        identity: { Require: "minicloud-node" },
        repositories: [],
      },
      process: {
        host_directory: host,
        expected_uid: Deno.uid(),
        git_program: "/usr/bin/git",
        environment: { PATH: "/usr/bin:/bin", HOME: path.join(root, "home") },
        command_timeout_ms: 300000,
        cleanup_timeout_ms: 5000,
        shutdown_grace_ms: 2000,
      },
      clone: {
        repository_root: path.join(root, "repositories"),
        git_config: path.join(config, "clone.gitconfig"),
        search_path: ["/usr/bin"],
        ssh: { kind: "disabled" },
      },
      ipc: {
        controller_id: "minicloud-controller",
        endpoint: path.join(node, "control.sock"),
        heartbeat_ms: 1000,
        frame_timeout_ms: 10000,
      },
      timezone,
      recovery_interval_ms: 1000,
    }),
  );
  await defaultFile(
    path.join(config, "server.json"),
    json({
      listen: "127.0.0.1:4317",
      node_id: "minicloud-node",
      controller: {
        home_directory: controller,
        protected_state_directories: [node, host],
        controller_id: "minicloud-controller",
        nodes: [
          {
            node_id: "minicloud-node",
            endpoint: path.join(node, "control.sock"),
          },
        ],
        session: { io_timeout_ms: 10000, query_interval_ms: 1000 },
        reconnect_ms: 1000,
        timezone,
      },
    }),
  );
  await defaultFile(path.join(config, "client.json"), json({ port: 5174 }));
}

type Child = {
  name: string;
  process: Deno.ChildProcess;
  status: Promise<Deno.CommandStatus>;
  exited: boolean;
};

/** Supervises isolated process groups so terminal signals cannot bypass ordered Node cleanup. */
async function run(): Promise<void> {
  if (Deno.build.os !== "linux")
    throw new Error("minicloud currently requires Linux.");
  if (Deno.args.some((arg) => !["--init-only", "--no-build"].includes(arg)))
    throw new Error("Usage: run-minicloud.ts [--init-only] [--no-build]");
  // State lives under the home directory, not the checkout: Unix socket paths are limited to
  // 108 bytes, and checkout locations (especially generated worktree names) are not under the
  // launcher's control. The real home path is what the kernel sees, so resolve it before measuring.
  const home = Deno.env.get("HOME");
  if (!home) throw new Error("HOME must be set to locate minicloud state.");
  const realHome = await Deno.realPath(home);
  const base = path.join(realHome, ".ora", "minicloud");
  for (let ancestor = realHome; ; ancestor = path.dirname(ancestor)) {
    const info = await Deno.lstat(ancestor);
    if (info.isSymlink || (info.uid !== 0 && info.uid !== Deno.uid())) {
      throw new Error(
        `Untrusted state ancestor: ${ancestor}. Symlinks and other owners are not supported.`,
      );
    }
    if (ancestor === path.dirname(ancestor)) break;
  }
  await directory(path.dirname(base));
  await directory(base);
  // Each checkout keeps separate state so host journals on different branches never share
  // schema migrations; the marker makes the owner visible and catches a reused digest.
  const digest = Array.from(
    new Uint8Array(
      await crypto.subtle.digest(
        "SHA-256",
        new TextEncoder().encode(workspace),
      ),
    ),
  )
    .slice(0, 4)
    .map((byte) => byte.toString(16).padStart(2, "0"))
    .join("");
  const root = path.join(base, digest);
  await directory(root);
  const marker = path.join(root, "workspace");
  await defaultFile(marker, `${workspace}\n`);
  if ((await Deno.readTextFile(marker)).trim() !== workspace) {
    throw new Error(
      `State directory ${root} belongs to another checkout; remove it or use that checkout.`,
    );
  }
  await defaultFile(path.join(root, "dev.lock"), "");
  const lock = await Deno.open(path.join(root, "dev.lock"), {
    read: true,
    write: true,
  });
  if (!(await lock.tryLock(true))) {
    lock.close();
    throw new Error("Another minicloud launcher owns this data directory.");
  }
  const children: Child[] = [];
  let stopping = false;
  let finish!: () => void;
  const stopped = new Promise<void>((resolve) => {
    finish = resolve;
  });
  const stop = () => {
    stopping = true;
    finish();
  };
  // Children run in their own sessions, so a terminal hangup reaches only the launcher; without a
  // handler Deno would exit at once and skip the ordered stop, leaving every component orphaned.
  const signals = ["SIGINT", "SIGTERM", "SIGHUP"] as const;
  for (const signal of signals) Deno.addSignalListener(signal, stop);

  /** Retains exit evidence and wakes the supervisor on unexpected service loss. */
  function start(
    name: string,
    args: string[],
    cwd = workspace,
    env: Record<string, string> = {},
  ): Child {
    if (stopping) throw new Error("Startup interrupted.");
    const process = new Deno.Command("setsid", {
      args,
      cwd,
      env,
      stdin: "null",
      stdout: "inherit",
      stderr: "inherit",
    }).spawn();
    const child: Child = {
      name,
      process,
      status: process.status,
      exited: false,
    };
    child.status.then(() => {
      child.exited = true;
    });
    children.push(child);
    return child;
  }

  /** Bounds each normal stop and reports escalation instead of claiming graceful completion. */
  async function terminate(child: Child): Promise<void> {
    if (child.exited) return;
    Deno.kill(-child.process.pid, "SIGTERM");
    let timer: ReturnType<typeof setTimeout> | undefined;
    const exited = await Promise.race([
      child.status.then(() => true),
      new Promise<boolean>((resolve) => {
        timer = setTimeout(() => resolve(false), 30000);
      }),
    ]);
    clearTimeout(timer);
    if (!exited) {
      console.error(
        `${child.name}: graceful stop timed out; killing its process group. State retained for recovery.`,
      );
      Deno.kill(-child.process.pid, "SIGKILL");
    }
    const status = await child.status;
    if (!status.success && child.name === "node") {
      console.error(
        "Node did not stop cleanly; inspect recovery on next startup.",
      );
      Deno.exitCode = 1;
    }
  }

  /** Waits for observable readiness, failing promptly when startup exits or receives Ctrl+C. */
  async function ready(
    child: Child,
    probe: () => Promise<boolean>,
  ): Promise<void> {
    const deadline = Date.now() + 30000;
    while (!stopping && !child.exited && Date.now() < deadline) {
      if (await probe()) return;
      await delay(100);
    }
    throw new Error(`${child.name} did not become ready.`);
  }

  /** Probes the actual socket, never treating a stale endpoint file as readiness. */
  async function socket(name: string): Promise<boolean> {
    try {
      const connection = await Deno.connect({ transport: "unix", path: name });
      connection.close();
      return true;
    } catch {
      return false;
    }
  }

  /** Identifies only guardians deployed into this application's private, versioned binary directory. */
  async function guardians(): Promise<
    Array<{ pid: number; identity: string }>
  > {
    const found: Array<{ pid: number; identity: string }> = [];
    for await (const entry of Deno.readDir("/proc")) {
      if (!/^\d+$/.test(entry.name)) continue;
      try {
        const executable = await Deno.readLink(
          path.join("/proc", entry.name, "exe"),
        );
        if (
          path.dirname(executable) !== path.join(root, "bin") ||
          !/^guardian-[a-f0-9]{64}$/.test(path.basename(executable))
        )
          continue;
        const stat = await Deno.readTextFile(
          path.join("/proc", entry.name, "stat"),
        );
        const fields = stat.slice(stat.lastIndexOf(")") + 2).split(" ");
        if (fields[0] !== "Z")
          found.push({ pid: Number(entry.name), identity: fields[19] });
      } catch (error) {
        if (
          !(error instanceof Deno.errors.NotFound) &&
          !(error instanceof Deno.errors.PermissionDenied)
        )
          throw error;
      }
    }
    return found;
  }

  let ownsHost = false;
  try {
    await initialize(root);
    if (Deno.args.includes("--init-only")) {
      console.log(`Initialized ${root}`);
      return;
    }
    if (!Deno.args.includes("--no-build")) {
      for (const [name, args] of [
        ["dependencies", [Deno.execPath(), "install"]],
        [
          "build",
          [
            "cargo",
            "build",
            "-p",
            "ora-minicloud-server",
            "-p",
            "ora-node",
            "-p",
            "ora-process-host",
            "-p",
            "ora-process-guardian",
          ],
        ],
      ] as const) {
        const child = start(name, [...args]);
        const status = await Promise.race([
          child.status,
          stopped.then(() => null),
        ]);
        if (!status?.success)
          throw new Error(`${name} failed or was interrupted.`);
      }
    }
    const config = path.join(root, "config");
    const nodeConfig = JSON.parse(
      await Deno.readTextFile(path.join(config, "node.json")),
    );
    const serverConfig = JSON.parse(
      await Deno.readTextFile(path.join(config, "server.json")),
    );
    const clientConfig = JSON.parse(
      await Deno.readTextFile(path.join(config, "client.json")),
    );
    if (
      nodeConfig.node.home_directory !== path.join(root, "node") ||
      nodeConfig.process.host_directory !== path.join(root, "p") ||
      nodeConfig.ipc.endpoint !== path.join(root, "node", "control.sock") ||
      serverConfig.controller.home_directory !== path.join(root, "controller")
    ) {
      throw new Error(
        "Launcher-owned state paths must remain under the launcher's state directory; use the standalone binaries for other deployments.",
      );
    }
    const url = new URL(`http://${serverConfig.listen}`);
    if (
      url.hostname !== "127.0.0.1" ||
      !Number.isInteger(clientConfig.port) ||
      clientConfig.port < 1 ||
      clientConfig.port > 65535
    )
      throw new Error("Use 127.0.0.1 and valid local ports.");
    for (const port of [Number(url.port), clientConfig.port]) {
      const listener = Deno.listen({ hostname: "127.0.0.1", port });
      listener.close();
    }
    const binary = (name: string) =>
      path.join(workspace, "target", "debug", name);
    const bytes = await Deno.readFile(binary("ora-process-guardian"));
    const hash = Array.from(
      new Uint8Array(await crypto.subtle.digest("SHA-256", bytes)),
    )
      .map((byte) => byte.toString(16).padStart(2, "0"))
      .join("");
    const guardian = path.join(root, "bin", `guardian-${hash}`);
    try {
      await Deno.writeFile(guardian, bytes, { createNew: true, mode: 0o700 });
    } catch (error) {
      if (!(error instanceof Deno.errors.AlreadyExists)) throw error;
    }
    const guardianInfo = await Deno.lstat(guardian);
    if (
      !guardianInfo.isFile ||
      guardianInfo.isSymlink ||
      (guardianInfo.mode! & 0o077) !== 0
    )
      throw new Error(`Invalid deployed guardian: ${guardian}`);
    const host = path.join(root, "p");
    if (
      (await socket(path.join(host, "host.sock"))) ||
      (await socket(nodeConfig.ipc.endpoint))
    ) {
      throw new Error(
        "A host or Node is already running for this directory; stop that owner before using the launcher.",
      );
    }
    let mode = "create";
    try {
      await Deno.lstat(host);
      mode = "recover";
    } catch (error) {
      if (!(error instanceof Deno.errors.NotFound)) throw error;
    }
    const hostChild = start("host", [
      binary("ora-process-host"),
      mode,
      host,
      guardian,
    ]);
    await ready(hostChild, () => socket(path.join(host, "host.sock")));
    ownsHost = true;
    const node = start("node", [
      binary("ora-node"),
      path.join(config, "node.json"),
    ]);
    await ready(node, () => socket(nodeConfig.ipc.endpoint));
    const server = start("server", [
      binary("ora-minicloud-server"),
      path.join(config, "server.json"),
    ]);
    const http = async (address: string) => {
      try {
        const response = await fetch(address, {
          signal: AbortSignal.timeout(1000),
        });
        await response.body?.cancel();
        return response.ok;
      } catch {
        return false;
      }
    };
    await ready(server, () => http(`${url.origin}/api/clones`));
    const vite = start(
      "vite",
      [
        path.join(workspace, "node_modules", ".bin", "vite"),
        "--port",
        String(clientConfig.port),
      ],
      path.join(workspace, "apps", "minicloud", "client"),
      {
        MINICLOUD_SERVER_URL: url.origin,
        MINICLOUD_CACHE_DIR: path.join(root, "vite"),
      },
    );
    const frontend = `http://127.0.0.1:${clientConfig.port}`;
    await ready(vite, () => http(frontend));
    console.log(
      `minicloud ready: ${frontend}\nData: ${root}\nCtrl+C stops all development components; data is preserved.`,
    );
    await Promise.race([
      stopped,
      ...[hostChild, node, server, vite].map(async (child) => {
        const status = await child.status;
        if (!stopping)
          throw new Error(
            `${child.name} exited unexpectedly (${status.code}).`,
          );
      }),
    ]);
  } finally {
    stopping = true;
    // Reverse order is Vite → server → Node → host. Guardian shutdown follows Node cleanup.
    for (const child of children.toReversed()) {
      try {
        await terminate(child);
      } catch (error) {
        console.error(`Stopping ${child.name}: ${error}`);
        Deno.exitCode = 1;
      }
    }
    if (ownsHost) {
      const deadline = Date.now() + 10000;
      while (true) {
        const remaining = await guardians();
        if (!remaining.length) break;
        for (const guardian of remaining) {
          const current = (await guardians()).find(
            (item) =>
              item.pid === guardian.pid && item.identity === guardian.identity,
          );
          if (current) {
            try {
              Deno.kill(
                current.pid,
                Date.now() < deadline ? "SIGTERM" : "SIGKILL",
              );
            } catch (error) {
              if (!(error instanceof Deno.errors.NotFound)) {
                console.error(error);
                Deno.exitCode = 1;
              }
            }
          }
        }
        if (Date.now() > deadline + 2000) {
          console.error(
            "Some minicloud guardians did not exit; inspect processes before restarting.",
          );
          Deno.exitCode = 1;
          break;
        }
        await delay(100);
      }
    }
    for (const signal of signals) Deno.removeSignalListener(signal, stop);
    lock.close();
  }
}

if (import.meta.main) {
  try {
    await run();
  } catch (error) {
    console.error(
      `minicloud: ${error instanceof Error ? error.message : error}`,
    );
    Deno.exitCode = 1;
  }
}
