import { spawnSync } from "node:child_process";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

/** Returns generated text in its comparison form without masking non-newline changes. */
export function normalizeGeneratedText(source: string): string {
  return source.replaceAll("\r\n", "\n");
}

/** Checks the second generated layer in isolation so verification never repairs a stale checkout. */
export async function checkContractSchema(): Promise<void> {
  const root = fileURLToPath(new URL("../", import.meta.url));
  // ts-to-zod resolves `zod` for its "Validate generated types" step from the
  // working directory's node_modules, so run it inside the package that owns it.
  const contractsDirectory = path.join(root, "packages", "contracts");
  const sourceDirectory = path.join(contractsDirectory, "src");
  const manifest = JSON.parse(
    await readFile(path.join(sourceDirectory, "..", "package.json"), "utf8"),
  );
  const generatorVersion: string = manifest.devDependencies["ts-to-zod"];
  // Keep the scratch output inside the package: ts-to-zod's validator composes
  // relative imports from the output location, which breaks if it lives in a
  // system temp directory.
  const temporary = await mkdtemp(
    path.join(contractsDirectory, ".schema-check-"),
  );
  try {
    const output = path.join(temporary, "error.schema.ts");
    const generated = spawnSync(
      Deno.execPath(),
      [
        "run",
        "-A",
        `npm:ts-to-zod@${generatorVersion}`,
        path.relative(
          contractsDirectory,
          path.join(sourceDirectory, "error.ts"),
        ),
        path.relative(contractsDirectory, output),
      ],
      {
        cwd: contractsDirectory,
        encoding: "utf8",
        env: {
          ...process.env,
          // ts-to-zod (via @oclif/core) reads os.userInfo() when SHELL is unset,
          // which Deno's node:os polyfill cannot do on Windows.
          SHELL:
            process.env.SHELL ??
            (Deno.build.os === "windows" ? "cmd.exe" : "/bin/sh"),
        },
      },
    );
    if (generated.status !== 0)
      throw new Error(
        `Schema generation failed:\n${generated.stdout}\n${generated.stderr}`,
      );
    const [expected, current] = await Promise.all([
      readFile(output, "utf8"),
      readFile(path.join(sourceDirectory, "error.schema.ts"), "utf8"),
    ]);
    if (normalizeGeneratedText(current) !== normalizeGeneratedText(expected))
      throw new Error(
        "Generated error.schema.ts differs; run task export-contracts.",
      );
  } finally {
    await rm(temporary, { recursive: true, force: true });
  }
}

if (import.meta.main) await checkContractSchema();
