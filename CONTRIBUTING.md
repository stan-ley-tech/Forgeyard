# Contributing to Forgeyard

Forgeyard is a Tauri (Rust) + React (TypeScript) desktop app. This doc covers
what you need to get a dev loop running and the conventions the codebase
follows.

## Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain; `rustfmt` and `clippy` components)
- [Node.js](https://nodejs.org/) 20+
- Platform build tools for Tauri:
  - **Windows**: the Visual C++ Build Tools (rustup will prompt to install
    these automatically the first time you build if they're missing).
  - **Linux**: `libwebkit2gtk-4.1-dev`, `libappindicator3-dev`,
    `librsvg2-dev`, `patchelf` (see `.github/workflows/ci.yml` for the exact
    apt packages used in CI).
- `git` on your `PATH` — Forgeyard shells out to the system git binary
  rather than vendoring its own (see [ARCHITECTURE.md](ARCHITECTURE.md)).

## Getting started

```bash
npm install
npm run tauri dev
```

This starts the Vite dev server and launches the Tauri window pointed at
it, with hot reload on the frontend and a rebuild-and-relaunch on Rust
changes.

## Project layout

```
src/            React + TypeScript frontend
src-tauri/      Rust backend (the "engine")
  src/
    project/    Project registration (SQLite-backed)
    detect/     Language / package manager / Docker / env-file detection
    deps/       Manifest parsers (npm, pip, go, cargo) + conflict detection
    process/    Process manager (spawn, stream logs, crash detection)
    git/        Git engine (shells out to the system git binary)
    tasks/      forgeyard.toml task definitions + npm script fallback
    logs/       Unified, persisted log store
    automation/ Watch-and-run automation rules
    commands/   The Tauri IPC surface tying the above together
```

## Running checks locally

```bash
# frontend
npm run format:check
npm run lint
npm test

# backend (run from src-tauri/)
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

These are the same checks CI runs on every push and pull request. (Skip
`--all-features` on clippy/test: it pulls in the `custom-protocol` feature,
which expects a built frontend at `../dist` to embed — that only exists
after `npm run build`.)

## Commit and PR conventions

- Prefix commit subjects with `feat:`, `fix:`, `refactor:`, `test:`,
  `docs:`, or `chore:` depending on the nature of the change.
- Keep commits scoped to one logical change — a new module, a bug fix, a
  test addition — rather than bundling unrelated work.
- Add tests alongside the code they cover. Rust modules with real logic
  (detectors, parsers, the process manager, the git engine) should have
  unit or integration tests in the same file under `#[cfg(test)]`.
- Run the checks above before opening a PR.

## Adding a new detector or parser

Detection (`src-tauri/src/detect/`) and dependency parsing
(`src-tauri/src/deps/`) are both organized as one file per ecosystem, with a
plain function taking a project path (or file contents) and returning a
typed result — no shared trait or registry. If you're adding support for a
new ecosystem, follow the existing files as a template and wire the new
function into the relevant `scan()` in `mod.rs`.

## Reporting issues

Open a GitHub issue with steps to reproduce, your OS, and (if relevant) the
contents of the project's manifest files that triggered the problem —
dependency parsing bugs are almost always manifest-shape specific.
