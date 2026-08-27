# Architecture

## Overview

Forgeyard is a Tauri application: a Rust "engine" process embedding a
system webview, with a React/TypeScript frontend talking to it over
Tauri's IPC bridge. All state is local — a SQLite database in the OS's
per-app data directory, plus whatever the registered projects' own
filesystems and Git repositories contain. There is no server component
and no network calls made on the user's behalf.

```
┌──────────────────────────────┐
│       Tauri UI (WebView)     │
│       React + TypeScript     │
└──────────────┬───────────────┘
               │ IPC (commands + events)
┌──────────────▼───────────────┐
│         Rust Engine          │
│                               │
│  project     — registration  │
│  detect      — health scan   │
│  deps        — manifest parse│
│  process     — subprocess mgr│
│  git         — git CLI shim  │
│  tasks       — task config   │
│  logs        — log store     │
│  automation  — watch + run   │
└──────────────┬───────────────┘
               │
       ┌───────┴────────┐
       ▼                ▼
   SQLite           Local OS
  (app state)          │
              ┌─────────┼─────────┐
              ▼         ▼         ▼
            git    subprocesses  filesystem
         (CLI shim)  (services)   (notify)
```

## Module breakdown

Each module under `src-tauri/src/` owns one concern and exposes plain
functions or a small struct — there's no shared trait hierarchy or plugin
registry, since the module list is fixed and small enough that one isn't
needed.

- **`project`** — SQLite-backed CRUD for registered projects (id, name,
  absolute path). This is the only module with truly persistent identity;
  everything else is either recomputed live from the filesystem/Git, or
  keyed off a `project_id` foreign key.
- **`detect`** — marker-file based detection of languages, package
  managers, Docker configuration, and env files. Pure functions over a
  `&Path`, no state, recomputed on every request rather than cached —
  a project's stack can change between two clicks on it, and the scan is
  cheap (a handful of `Path::is_file` checks and small file reads).
- **`deps`** — one parser per ecosystem (`package.json`,
  `requirements.txt` + `pyproject.toml`, `go.mod`, `Cargo.toml`) producing
  a unified `Dependency` list, plus a conflict check that flags a package
  name resolving to more than one version spec across manifests.
- **`process`** — the subprocess manager. See [Process lifecycle](#process-lifecycle)
  below.
- **`git`** — shells out to the system `git` binary and parses porcelain
  v2 output, rather than vendoring `libgit2`. See
  [Why shell out to git](#why-shell-out-to-git).
- **`tasks`** — reads a project's `forgeyard.toml` (explicit task groups)
  and, if present, derives an additional group from `package.json`
  `scripts` so a project is runnable with zero Forgeyard-specific config.
- **`logs`** — persists process output as leveled, filterable log rows in
  SQLite, with a fixed retention cap per project.
- **`automation`** — rule CRUD, a `notify`-based filesystem watcher per
  enabled rule, and a sequential task-chain runner used by both the
  watcher and a manual "run now" command.
- **`commands`** — the `#[tauri::command]` functions that are the actual
  IPC surface; thin translation from `(project_id, ...)` arguments to
  calls into the modules above.

## IPC design

Two channels, matching Tauri's own split:

- **Commands** (frontend → backend, request/response): registering a
  project, reading dependency/health reports, Git operations, task runs,
  automation rule CRUD. Every command returns `Result<T, AppError>`, and
  `AppError` implements `Serialize` as a plain string message so the
  frontend gets a readable error rather than a Rust debug dump.
- **Events** (backend → frontend, fire-and-forget): `process:log` and
  `process:status` stream live process output and state transitions;
  `automation:run` streams per-stage results as an automation chain
  executes. The frontend subscribes with `@tauri-apps/api/event`'s
  `listen()`; nothing is buffered for a listener that isn't attached yet
  (a page opened after a process started gets its state via
  `list_processes`/`get_logs`, then live updates via events from there).

Commands are the only place argument validation happens; the module
functions underneath assume valid input (a real path, a project that
exists) and return `AppError` variants for the cases that legitimately can
fail (path doesn't exist, git command failed, task not found).

## Process lifecycle

The process manager (`process::ProcessManager`) tracks one child process
per `(project_id, service_name)` key. States: `Running → Stopping →
{Stopped, Crashed}`.

The tricky part is who owns the `tokio::process::Child` handle. Two things
need access to it concurrently: whatever calls `stop`/`kill`, and the task
that awaits the process's exit to update its state. Sharing a `Child`
behind a lock doesn't work well here — `Child::wait()` can block for the
entire lifetime of a long-running dev server, and holding a lock across
that would starve `stop`/`kill` for as long as the process runs.

Instead: the OS pid is captured once at spawn time, and the `Child` is
moved *entirely* into the task that awaits its exit — nothing else ever
touches it again. `stop` and `kill` act on the pid directly:

- **Unix**: `stop` sends `SIGTERM` via `libc::kill`; `kill` sends
  `SIGKILL`. A `stop` that doesn't exit in time can be escalated to `kill`
  by the caller (this is what `restart` does).
- **Windows**: there's no POSIX-signal equivalent for an arbitrary
  process, so both `stop` and `kill` hard-terminate the process tree via
  `taskkill /PID <pid> /T /F`. This is a real platform asymmetry, not an
  oversight — documented here rather than papered over.

An `expected_stop: Arc<AtomicBool>` flag, set right before a signal is
sent, is how the exit-watcher task distinguishes "we stopped this on
purpose" from "this crashed": a clean `exit(0)` or an exit following a
flagged stop is reported as `Stopped`; anything else (nonzero exit,
terminated by signal, unflagged) is `Crashed`.

Known limitation: signaling by pid has the same theoretical race as `kill
-9 <pid>` anywhere else — if the process already exited and the OS reused
its pid before the signal is sent, the wrong process could be signaled.
In practice the window is microseconds and the risk is the same one every
process-supervisor tool built on bare pids accepts; a fully race-free
version would track the process via a job object (Windows) / process
group (Unix) instead, noted here as a possible future hardening rather
than built now.

## Why shell out to `git`

Forgeyard is a tool for developers who, by definition, already have a
working `git` installation — vendoring `libgit2` (via the `git2` crate)
would mean compiling a C library from source on every contributor's
machine for functionality the system binary already provides, correctly,
for free. Git's porcelain output (`--porcelain=v2`, structured `log`
formatting with unit/record separators) is a stable, documented interface
that doesn't drift across git versions the way scraping human-readable
output would. The tradeoff is that `git` must be on `PATH` — true for
essentially every machine this app runs on.

## Persistent local state

SQLite (via `rusqlite`, bundled) holds:

- `projects` — registered project records
- `log_entries` — unified log history, capped at 5,000 rows per project
  (oldest trimmed on every write) so a long-running session doesn't grow
  the database without bound
- `automation_rules` / `automation_runs` — rule definitions and their run
  history

Everything else (detected languages, dependency lists, Git status,
currently-running processes) is intentionally *not* persisted — it's
either cheap to recompute from the filesystem/Git on demand, or (for
running processes) inherently tied to the current app session and
shouldn't survive a restart pretending nothing changed.

## Crash recovery and resource cleanup

Every spawned child is created with `kill_on_drop(true)`, so if the
Forgeyard process itself is killed, its OS-level children don't become
orphans. Automation watchers are plain `notify::RecommendedWatcher`
values; dropping one (via `unwatch`, or as part of re-registering a rule)
stops that subscription with no separate teardown step. Nothing in the
app assumes a graceful shutdown happened.

## Testing strategy

- **Unit tests** live next to the code they test (`#[cfg(test)] mod
  tests` in the same file) for the detectors, dependency parsers, task
  config loader, and log store.
- **Integration tests** for the process manager spawn real short-lived
  OS processes (a cross-platform `exit N` via the shell) and assert on
  observed state transitions, including the crash-vs-stopped distinction.
- **Integration tests** for the git engine run against real temporary Git
  repositories (`git init` in a `tempfile::tempdir()`, with a local
  `user.name`/`user.email` so tests don't depend on the host's global git
  config) rather than mocking `git` output — porcelain parsing is exactly
  the kind of logic that's easy to get subtly wrong against synthetic
  fixtures and right against a real repo.
- **Frontend component tests** (Vitest + Testing Library) cover the UI
  logic most likely to have real bugs: filtering, formatting, and event
  handling in the process, log, and Git panels.
- **Not included**: full end-to-end tests driving the packaged desktop
  app. Tauri's WebDriver-based E2E story requires a display and a
  platform-specific driver (`tauri-driver` + WebView2/WebKitWebDriver),
  which doesn't fit a CI matrix that also has to build native installers
  on constrained runners. This is a deliberate scope cut, not an
  oversight — noted here so it's an explicit tradeoff rather than a
  silent gap.

## Engineering decisions worth calling out

- **No ORM.** `rusqlite` directly, with hand-written SQL. The schema is
  small (four tables) and stable enough that a query builder or ORM would
  add a dependency and an abstraction layer without meaningfully reducing
  code.
- **No custom glob dependency.** The automation watcher's glob matching
  (`*.rs`, `src/*.ts`) is a ~15-line recursive matcher rather than a
  dependency on a full glob-spec crate, since rules only ever need `*` as
  a wildcard.
- **Outdated-package checking is out of scope.** A real "2 packages
  outdated" check requires querying package registries over the network.
  Forgeyard is local-first and doesn't make outbound calls on the user's
  behalf; the dependency panel focuses on inventory and cross-manifest
  conflicts, both fully computable offline.
- **Task run history beyond logs isn't separately tracked.** Task output
  flows through the same process manager and log store as any other
  service (under a `group:task` service name), so "what did this run
  print" is already covered; a dedicated pass/fail run-history table for
  ad hoc tasks (as opposed to automation runs, which do get one) was cut
  as duplicate bookkeeping for a “not that much” v1.
