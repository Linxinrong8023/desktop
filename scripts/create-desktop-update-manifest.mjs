import fs from "node:fs/promises";
import path from "node:path";

const bundlesDirectory = path.resolve(process.argv[2] ?? "bundles");
const releaseTag = process.env.RELEASE_TAG;

if (!releaseTag) {
  throw new Error("RELEASE_TAG is required to create latest.json");
}

const releaseVersion = releaseTag.replace(/^v/, "");
const files = await collectFiles(bundlesDirectory);
const assets = new Map(
  files.map((file) => [path.basename(file), file]),
);

const platforms = {
  "windows-x86_64": findAsset(assets, /(?:_x64-setup|setup)\.exe$/i),
  "darwin-aarch64": findAsset(assets, /\.app\.tar\.gz$/i),
  "linux-x86_64": findAsset(assets, /\.AppImage$/),
};

const manifest = {
  version: releaseVersion,
  notes: `Ora ${releaseTag}`,
  pub_date: new Date().toISOString(),
  platforms: Object.fromEntries(
    Object.entries(platforms).map(([target, asset]) => {
      const signaturePath = `${asset.path}.sig`;
      return [
        target,
        {
          url: `https://github.com/ora-space/desktop/releases/download/${encodeURIComponent(releaseTag)}/${encodeURIComponent(asset.name)}`,
          signature: "",
          signaturePath,
        },
      ];
    }),
  ),
};

for (const platform of Object.values(manifest.platforms)) {
  platform.signature = await fs.readFile(`${platform.signaturePath}`, "utf8");
  delete platform.signaturePath;
}

await fs.writeFile(
  path.join(bundlesDirectory, "latest.json"),
  `${JSON.stringify(manifest, null, 2)}\n`,
);

function findAsset(assets, pattern) {
  for (const [name, file] of assets) {
    if (pattern.test(name) && assets.has(`${name}.sig`)) {
      return { name, path: file };
    }
  }
  throw new Error(`Could not find a signed updater asset matching ${pattern}`);
}

async function collectFiles(directory) {
  const entries = await fs.readdir(directory, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    const entryPath = path.join(directory, entry.name);
    if (entry.isDirectory()) files.push(...(await collectFiles(entryPath)));
    else files.push(entryPath);
  }
  return files;
}
