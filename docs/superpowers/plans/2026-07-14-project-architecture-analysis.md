# Ora Project Architecture Analysis Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:dispatching-parallel-agents for the independent runtime, request-flow, and infrastructure investigations.

**Goal:** Produce a source-backed Chinese architecture guide that explains Ora's actual deliverables, code structure, module responsibilities, startup mechanics, interfaces, and end-to-end data flow.

**Architecture:** Treat manifests and executable entry points as the source of truth, then trace inward through transport, application, domain, and infrastructure boundaries. Analyze independent subsystems in parallel, reconcile them against tests and existing runtime documentation, and explicitly distinguish implemented behavior from specifications or unfinished wiring.

**Tech Stack:** Rust 2024, Tokio, Axum, SQLite/rusqlite, Tauri 2, React 19, TypeScript 6, Vite 8, pnpm workspaces.

## Global Constraints

- Do not modify business code.
- Every architectural claim must be traceable to a current source file, manifest, test, or runtime document.
- Separate the product vision, buildable artifacts, and incomplete or disconnected code.
- The final deliverable is `docs/project-architecture.md` in Chinese.

---

### Task 1: Establish Repository Scope and Deliverables

**Files:**
- Read: `AGENTS.md`
- Read: `README.md`
- Read: `package.json`
- Read: `Cargo.toml`
- Read: `Taskfile.yml`
- Read: `pnpm-workspace.yaml`

**Interfaces:**
- Consumes: repository manifests and documented development commands
- Produces: authoritative inventory of workspace members, build targets, and runtime entry points

- [x] Enumerate tracked source files with `rg --files` and group them by app, package, crate, documentation, and historical specification.
- [x] Compare pnpm workspace members with Cargo workspace members and record intentionally separate targets.
- [x] Trace `dev`, `build`, `tauri`, `run:web-backend`, and contract-generation commands to their concrete executables.

### Task 2: Trace Desktop and Plugin Runtime

**Files:**
- Read: `apps/desktop/**`
- Read: `packages/ui/**`
- Read: `packages/plugin-sdk/**`

**Interfaces:**
- Consumes: Vite/Tauri configuration, React source, SDK protocol and tests
- Produces: desktop startup sequence, plugin protocol sequence, and a gap analysis of unconnected components

- [x] Trace the Tauri development and production build lifecycle from root scripts through Vite and the Rust entry point.
- [x] Identify every plugin SDK export, transport message, stdin/stdout operation, and test-covered behavior.
- [x] Search the repository for plugin SDK consumers and host process launch code to determine whether a complete plugin loader exists.

### Task 3: Trace Web Requests Through the Layered Backend

**Files:**
- Read: `apps/web/server/src/**`
- Read: `crates/application/src/**`
- Read: `crates/contracts/src/**`

**Interfaces:**
- Consumes: Axum routes, transport DTOs, application handlers, repository and runtime ports
- Produces: route matrix plus step-by-step HTTP and WebSocket data-flow narratives

- [x] Trace server bootstrap from environment parsing and logging through database migration, dependency construction, routing, and graceful shutdown.
- [x] Build a route table with method/path, inputs, outputs, status codes, and error mapping.
- [x] Trace project creation, task/worktree creation, session creation, and terminal WebSocket flows across every layer.

### Task 4: Map Domain and Infrastructure Modules

**Files:**
- Read: `crates/domain/src/**`
- Read: `crates/db/src/**`
- Read: `crates/gitlancer/src/**`
- Read: `crates/process/src/**`
- Read: `crates/pty/src/**`
- Read: `crates/logging/src/**`
- Read: `xtask/src/**`

**Interfaces:**
- Consumes: public crate APIs, implementation modules, migrations, and tests
- Produces: module responsibility table, dependency graph, persistence model, and infrastructure I/O contracts

- [x] Map domain entities, typed IDs, state enums, validation, and cross-entity relationships.
- [x] Trace SQLite bootstrap, migrations, connection management, repositories, and application port implementations.
- [x] Trace Git command construction/execution/parsing, child-process abstraction, PTY lifecycle/I/O/history, and structured logging sinks.
- [x] Trace Rust contract annotations through `cargo xtask export-contracts` into the TypeScript SDK.

### Task 5: Write the Architecture Guide

**Files:**
- Create: `docs/project-architecture.md`

**Interfaces:**
- Consumes: reconciled findings from Tasks 1-4
- Produces: one standalone Chinese guide for a reader with no prior project context

- [x] Write an executive conclusion that states the actual product shape before discussing internal structure.
- [x] Add repository and source trees, layered architecture and dependency diagrams, module input/output tables, startup sequences, endpoint matrix, and end-to-end data-flow diagrams.
- [x] Explain why each major module exists and mark current limitations, stubs, and non-runtime historical specifications.

### Task 6: Verify the Document Against the Repository

**Files:**
- Verify: `docs/project-architecture.md`

**Interfaces:**
- Consumes: completed guide and current repository
- Produces: a path-valid, internally consistent, evidence-backed final document

- [x] Extract every backticked repository path from the guide and confirm that it exists or is explicitly labeled as generated output.
- [x] Re-run route and public-export searches to ensure the tables cover current code.
- [x] Attempt `git diff --check`; because this workspace is not exposed as a usable Git worktree, use direct UTF-8/fence/path/placeholder checks plus independent source review for the equivalent document validation.

### Verification Evidence

- `cargo test --workspace`: 153 passed, 0 failed.
- Tauri `cargo metadata`: documented workspace-membership failure reproduced with exit code 101.
- Guide structure: 15 main sections, 11 Mermaid blocks, 84 balanced code fences, no placeholders or replacement characters.
- Repository references: every checked path exists, except one explicitly documented generated output directory.
- Independent review: Critical 0, Important 0; all reported Minor findings addressed.
