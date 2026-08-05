# Ora Plugin Manager

`ora-plugin-manager` discovers installed Ora plugin packages from an Ora data directory.

## Responsibilities

- Scan direct child directories under `<data-dir>/plugins`.
- Read and validate each child package's `package.json`.
- Return a deterministic, immutable snapshot of valid installed plugins.
- Isolate malformed or unsupported packages as structured discovery issues.

## Non-responsibilities

- Installing, enabling, disabling, or removing plugins.
- Starting plugin processes or loading plugin JavaScript.
- Evaluating Ora, Bun, or plugin API engine ranges.
- Watching the filesystem after discovery completes.

## Public interface

Call `PluginManager::discover(data_dir)` once during application bootstrap. Consumers read the resulting snapshot through `installed_plugins()` and report any non-fatal problems from `discovery_issues()`.

Discovery never follows symlinked package directories, never recurses below one package directory, and never reads more than 1 MiB from one manifest. A missing plugins directory represents an empty installation and is not an error.
