# Forgeyard

A local-first developer workspace for running, inspecting, testing, and
managing multiple software projects from one interface. Forgeyard replaces
a desk full of terminal tabs — dev server, worker, database, test runner —
with a single desktop app that understands your project's stack, tracks
its running services, and gives you one place to watch logs, run Git
workflows, and fire off common tasks.

It's a native desktop app: a Rust engine underneath a Tauri + React UI, with
SQLite for local state. Nothing leaves your machine.

```
project/
├── Go
├── PostgreSQL
├── Docker
├── Redis
└── 14 dependencies
```

## Why

Most developer tooling either lives in the terminal (fast, but siloed —
one shell per concern) or is a heavyweight IDE feature (integrated, but
tied to one editor). Forgeyard is neither: a small, focused desktop app
that sits next to your editor and terminal, and answers the questions you
otherwise `cd` and `ps` and `tail -f` your way through — what's running,
what changed, what's outdated, what broke.

## Features

**Project management** — register any local Git repository. Forgeyard
detects its languages, package manager, Docker setup, and env files
without any configuration, and shows a health summary per project.

**Process manager** — start, stop, restart, and kill project services
(dev server, worker, database, ...) from one panel. Crashes are detected
and distinguished from intentional stops, with exit codes surfaced
immediately.

```
API          RUNNING     :8000
Frontend     RUNNING     :5173
Postgres     RUNNING     :5432
Redis        STOPPED
Worker       RUNNING
```

**Git workflows** — status, staged/unstaged diffs, branches, checkout,
merge, stash, commit, and recent history for the day-to-day Git operations
you actually use, without leaving the app.

**Dependency intelligence** — parses `package.json`, `requirements.txt`,
`pyproject.toml`, `go.mod`, and `Cargo.toml` into one dependency list, and
flags packages that resolve to conflicting versions across manifests
(e.g. `requirements.txt` and `pyproject.toml` disagreeing).

**Unified logs** — every service's stdout/stderr streams into one
timestamped, filterable log view instead of four terminal tabs.

```
09:41:02 API       GET /users       200
09:41:03 WORKER    Job completed
09:41:03 POSTGRES  connection accepted
09:41:04 API       GET /projects    500
```

**Task runner** — define Development/Database/etc. command groups in a
`forgeyard.toml`, or just let Forgeyard pick up your `package.json`
scripts automatically. Run them from the UI, with live output.

**Automation** — "when files matching `*.rs` change, run lint then test"
style rules, backed by a real filesystem watcher, with per-stage
pass/fail results and run history.

## Stack

- **Backend**: Rust, [Tauri 2](https://tauri.app/), Tokio, SQLite
  (via `rusqlite`)
- **Frontend**: React, TypeScript, Vite
- **Testing**: Rust unit/integration tests (including against real
  temporary Git repositories), Vitest + Testing Library
- **CI/CD**: GitHub Actions — format, lint, test, and build on every push;
  packaged Windows and Linux installers on tagged releases

See [ARCHITECTURE.md](ARCHITECTURE.md) for how the pieces fit together,
the IPC design, and the engineering tradeoffs behind them.

## Getting started

Prerequisites: [Rust](https://rustup.rs/) (stable) and
[Node.js](https://nodejs.org/) 20+. See
[CONTRIBUTING.md](CONTRIBUTING.md) for platform-specific build
dependencies.

```bash
npm install
npm run tauri dev
```

## Configuring tasks

Drop a `forgeyard.toml` in a project's root to define runnable task
groups:

```toml
[[groups]]
name = "Development"
tasks = [
  { name = "Start", command = "npm run dev" },
  { name = "Test", command = "npm test" },
  { name = "Lint", command = "npm run lint" },
]

[[groups]]
name = "Database"
tasks = [
  { name = "Migrate", command = "npm run db:migrate" },
  { name = "Seed", command = "npm run db:seed" },
]
```

No `forgeyard.toml`? If the project has a `package.json`, its `scripts`
show up automatically under a "Scripts" group.

## License

[MIT](LICENSE)
