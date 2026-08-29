import { copyFile, mkdir } from "node:fs/promises";
import { spawn } from "node:child_process";
import path from "node:path";
import process from "node:process";

const repositoryRoot = path.resolve(import.meta.dirname, "..");
const release = process.argv.includes("--release");
const configuredTarget =
  process.env.TARGET_TRIPLE ??
  process.env.TAURI_ENV_TARGET_TRIPLE ??
  process.env.RUST_TARGET;

/** Runs one build command with its output attached to the invoking task. */
function run(command, args) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      cwd: repositoryRoot,
      stdio: "inherit",
    });
    child.once("error", reject);
    child.once("exit", (code, signal) => {
      if (code === 0) {
        resolve();
      } else {
        reject(
          new Error(
            `${command} exited with ${code ?? `signal ${signal ?? "unknown"}`}`,
          ),
        );
      }
    });
  });
}

/** Reads rustc's canonical host triple when no cross-compilation target was requested. */
async function hostTargetTriple() {
  let output = "";
  await new Promise((resolve, reject) => {
    const child = spawn("rustc", ["-vV"], { cwd: repositoryRoot });
    child.stdout.on("data", (chunk) => {
      output += chunk.toString();
    });
    child.once("error", reject);
    child.once("exit", (code) => {
      if (code === 0) resolve();
      else reject(new Error(`rustc -vV exited with ${code}`));
    });
  });
  const host = output.match(/^host:\s*(.+)$/m)?.[1];
  if (!host) throw new Error("rustc -vV did not report a host target");
  return host.trim();
}

const target = configuredTarget ?? (await hostTargetTriple());
const cargoArgs = ["build", "--package", "ora-reaper"];
if (release) cargoArgs.push("--release");
if (configuredTarget) cargoArgs.push("--target", configuredTarget);
await run("cargo", cargoArgs);

const profile = release ? "release" : "debug";
const executableSuffix = target.includes("windows") ? ".exe" : "";
const targetDirectory = process.env.CARGO_TARGET_DIR
  ? path.resolve(repositoryRoot, process.env.CARGO_TARGET_DIR)
  : path.join(repositoryRoot, "target");
const source = configuredTarget
  ? path.join(targetDirectory, target, profile, `ora-reaper${executableSuffix}`)
  : path.join(targetDirectory, profile, `ora-reaper${executableSuffix}`);
const binaryDirectory = path.join(
  repositoryRoot,
  "apps",
  "desktop",
  "src-tauri",
  "binaries",
);
const destination = path.join(
  binaryDirectory,
  `ora-reaper-${target}${executableSuffix}`,
);
await mkdir(binaryDirectory, { recursive: true });
await copyFile(source, destination);
console.log(`Installed ora-reaper sidecar at ${destination}`);
