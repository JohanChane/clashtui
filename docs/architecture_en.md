# Development Documentation

This document describes the code architecture of Clashtui for developers to understand the project structure.


## Tech Stack

- Language: Rust (Edition 2024)
- Build System: Cargo
- TUI Framework: ratatui + crossterm
- Async Runtime: tokio
- Config/Persistence: serde + serde_yml
- HTTP Client: minreq
- CLI Parser: clap

## Directory Structure

```
src/
├── main.rs              # Entry point: five-phase startup flow
├── cli.rs               # CLI module entry, re-exports from cli/
│   └── cli/
│       ├── handler.rs   # CLI subcommand handling (profile/service/mode/update)
│       ├── widgets.rs   # CLI interactive components (Confirm, Select)
│       └── utils.rs     # Version info, shell completion generation
├── config.rs            # Config module entry, re-exports from config/
│   └── config/
│       ├── core.rs      # CoreType enum, ServiceController
│       ├── database.rs  # ProfileManager (Profile persistence structures)
│       └── util.rs      # Config dir resolution, load_save! macro, file path constants
├── functions.rs         # Business logic entry, re-exports from functions/
│   └── functions/
│       ├── command.rs   # System operations (file permissions, service start/stop, directory opening)
│       ├── file.rs      # File operations (Profile import/update, Template processing)
│       └── restful.rs   # REST API calls (config/mode/proxies/connections)
├── tui.rs               # TUI module entry, re-exports from tui/
│   └── tui/
│       ├── app.rs       # App struct, event-driven loop, key routing
│       ├── business.rs  # Business commits and read-only views
│       ├── core.rs      # Service/API state, request ownership, data slots
│       ├── app/
│       │   ├── actions.rs   # Global actions, tab targets, and default bindings
│       │   └── tests.rs     # Global shortcut and tab-label regression tests
│       ├── key.rs       # Keys, typed sequences, default-binding macro, and loading entry point
│       ├── key/
│       │   ├── defaults.rs  # Typed single-key and chord declarations
│       │   ├── config.rs    # Parsing, overrides, unbinding, scope validation
│       │   ├── runtime.rs # Binding store, ID lookup tables, and global conflict validation
│       │   ├── matcher.rs   # Pending-chord state owned by App
│       │   └── notation.rs  # Key notation parsing and formatting
│       ├── signals.rs   # OS signal handling
│       ├── term.rs      # Terminal setup, acknowledged leases, input worker
│       ├── theme.rs     # Theme loading
│       ├── utils.rs     # Utility functions
│       ├── popmsg.rs    # Popup definitions (Confirm, Input, etc.)
│       ├── widget/
│       │   ├── mod.rs   # new_type_impl_tuiwidget! macro
│       │   ├── dualtab.rs  # Dual-panel tab container
│       │   ├── help.rs  # Scrollable help panel
│       │   ├── which.rs # Pending-chord hints
│       │   ├── popmsg.rs   # Popup container (rendering + event dispatch)
│       │   └── tab.rs   # Single-panel tab container + BasicTabContent/TabContent trait
│       └── tab/
│           ├── mod.rs   # Tab enum, newtype_tab!/enum_dispatch! macros
│           ├── status.rs     # Status tab
│           ├── files.rs      # FileTab (DualTab: Profile + Template)
│           ├── proxies.rs    # Proxies tab
│           ├── connections.rs # Connections tab
│           ├── logs.rs       # Logs view; logs/collector.rs owns background capture
│           ├── settings.rs   # Settings tab
│           └── srvctl.rs     # Core service control tab
```

## Startup Flow (5 Phases)

The program entry point is `src/main.rs`, which executes five phases in order:

1. **CLI Parse** — Parse command-line arguments and environment variables (`CLASHTUI_CONFIG_DIR`), handle early exits (e.g. `--generate-shell-completion`)
2. **Config Init** — Determine config directory, load `config.yaml` + `clashtui.db`, create missing directories and files
3. **TUI Init** — Load keymap (`keymap.yaml`), theme (`theme.yaml`), set terminal raw mode, register panic hook
4. **Event Loop** — Run `App::serve()`, looping to process rendering, events, and async tasks
5. **Restore & Save** — Exit raw mode, save `clashtui.db`

If the command line has subcommands (`profile`, `service`, `mode`, `update`), phases 3-5 are skipped and the subcommand executes directly before exiting.

## Config System

### Config Directory Resolution

Priority from high to low:
1. `--config-dir` command-line argument
2. `CLASHTUI_CONFIG_DIR` environment variable
3. `data/` subdirectory next to the executable (portable mode)
4. `$XDG_CONFIG_HOME/clashtui`
5. `~/.config/clashtui`

### Config Loading

- `ConfigFile` — Loads core paths and service config from `config.yaml`
- `BasicInfo` — Loads API address, secret, etc. from `core_override_config.yaml`
- `ProfileManager` — Loads Profile list and current selection from `clashtui.db`
- The three are merged into a `Config` struct, globally accessible via `config::CONFIG`

### Persistence

Uses the `load_save!` macro to auto-generate `from_file()` and `to_file()` methods. Format is YAML.

## TUI Architecture

### Event Loop

`App::serve()` uses `tokio::select!` to wait for input, popups, actual task completion, log changes, terminal handoff, exit, and business or display deadlines. It has no fixed frame tick. API refresh deadlines exist only while their pages are visible; active file updates request spinner frames every 160 ms.

Business owns CoreBusiness and LogCollector. Each command or core completion commits state, invalidates dependent queries and log sessions, advances pending work, and updates the collector before returning. Service commands start only after log invalidation. App then synchronizes the visible ordinary page directly through a read-only view and draws when dirty and holding the terminal. There is no component ID, subscription table, or local notification queue. Resize arrives through the input worker.

On exit, App stops Business before leaving the visible page, then asks popup and terminal owners to stop. It continues receiving completions until tasks, log capture, terminal work, and input have actually exited. A terminal lease waits for the input worker to acknowledge its pause before an external program can read the terminal.

### Effective Binding Maps and Input State

`Key` represents normalized terminal input; page `Action` enums represent commands. `KeySequence` distinguishes a single key from a two-key chord throughout configuration and runtime. `key_map!` declares typed defaults and configuration parsers. `Keymaps::from_config` applies core-specific scopes, default overrides, and unbindings, then checks each page against global bindings. Identical complete sequences and single-key/chord prefix overlaps stop startup with an error; `gp` and `gg` can share a prefix.

`Keymaps` owns each `RuntimeBinding` once. `scope_bindings` indexes IDs by source, `key_lookup` compiles global and page IDs for each focus context, and `tab_shortcuts` caches tab-binding IDs. Profile and Template have separate lookup tables; Status uses the global table. The store is installed in a read-only `OnceLock` only after all validation succeeds.

Input handling:

1. Popups capture raw keys and cancel any pending chord.
2. Help handles scrolling and dismissal. Starting a global chord closes help and retains only global candidates for that sequence.
3. App's `Matcher` uses the effective map and returns `Pending`, `Execute(Action)`, `Cancelled`, or `Unmatched`.
4. App executes global actions or passes typed page actions to `TuiTab::handle_action`.

App owns the only mutable pending-chord state. Tab changes, focus changes, and popups do not inherit old candidates. Esc and unmatched second keys are consumed. Help and tab labels borrow the same binding store. Which borrows candidates from the original prefix branch and sorts them at display time without persistent copies of descriptions or candidate order. Its title shows the pending prefix; pages need no matching state.

### TuiWidget Trait

`TuiWidget` provides the UI lifecycle:

- `render(&mut self, f, area, view)` — draw from a read-only business view
- `on_enter(&mut self, business)` / `on_leave(&mut self, business)` — ordered lifecycle commands
- `sync_from(&mut self, view)` — update local display caches without starting work
- `advance` — let the visible Files owner advance its two loads
- `poll_tasks` / `finish_task` — wake on completion and apply valid visible-page callbacks

`TuiTab` additionally exposes `title`, `input_scope`, and `handle_action`. Page containers receive resolved actions. Text popups handle raw keys through their own `handle_key_event` methods.

### Tab System

#### Single Panel (Tab)

`Tab<C>` is a generic container. `C` must implement two traits:

- `BasicTabContent` — defines `Action`, `State`, title, lifecycle hooks, and read-only synchronization
- `TabContent` — defines `init`, `handle_action`, `render`

#### Dual Panel (DualTab)

`DualTab<C1, C2>` is used for scenarios needing two panel switching (e.g. Files tab with Profile and Template). The two content types reference each other via `DualTabContent` / `DualTabContentMate` traits.

Both visible panes synchronize directly, regardless of keyboard focus. Focus only chooses the action target. FileTab owns a separate actual load slot for each pane.

#### Tab Enum

The `enum_dispatch!` macro unifies all tabs into a single `Tab` enum. Each variant uses the `newtype_tab!` macro to generate a wrapper and implement `TuiWidget` and `TuiTab`.

### Async Task Model

`init` initializes local state. `on_enter` activates resource demand; `on_leave` stops demand and discards page data while preserving preferences. `sync_from` and `render` receive read-only BusinessView values. Status and Settings read shared configuration directly; Proxies and Connections retain revision-based derived caches.

`CoreBusiness` owns service/API state, request handles, and accepted page data. The system service's core process provides the API: Stopped means unavailable, while Running still requires API confirmation. Unknown retains the service probe diagnostic. API checks wait for confirmed Running and for pending or actual checks of that service to finish. A new service check discards an older API reply while retaining one confirmation request; API success never rewrites the service observation. API pages reuse confirmed Running, but recheck Stopped or Unknown on entry or refresh. Startup retries are bounded and confirm service before API; external service changes have no immediate detection guarantee.

A request may commit only while its ownership remains valid. Service changes invalidate incompatible requests and log connections within the same business commit. Multi-step writes hold a revocable API permit and recheck it before configuration reloads or each batch request; restored access does not revive an old permit.

Each QuerySlot owns one InFlight task returning its actual result, plus result acceptance, pending demand, and a deadline. Invalidating an active resource preserves demand while retaining its actual task. Every core completion advances pending requests, including discarded results. Status and Proxies refresh five seconds after success; Connections uses one second; Settings has no periodic refresh. Errors stop periodic refresh until an explicit request. Paused Connections remain paused through manual refresh or API recovery.

`FutureSet<C>` owns a `JoinSet` of actual page operations plus weak identities for their visible-page callbacks. Tasks return `CallBack<C>` (`Box<dyn FnOnce(&mut C) + Send>`). Leaving invalidates callbacks, while operations still finish, clear activity, and deliver feedback. App polls their completion even when the page is hidden. Files mutations and explicit refreshes advance a shared data revision; the Files owner loads each unattempted revision while visible. Load slots retain actual JoinHandles across leave/enter, reject old results, and immediately chase a newer revision after completion. Errors count as attempts; only a new revision or enter retries them.

Selecting a profile in Files validates and merges the local configuration, writes it, and records the selection before checking API permission. Permission gates only the online reload. Without a valid permit, the configuration is saved and the user is prompted to start or restart the core. Updating the selected profile uses the same apply path and reports update and apply outcomes separately.

`LogCollector` belongs to Business and keeps up to 300 records in a `VecDeque<Arc<LogEntry>>`, including while the Logs view is hidden. A full buffer evicts its oldest record. WebSocket messages and frames are each limited to 256 KiB. Snapshots share records without copying their strings. Records are freed when their last reference is dropped. Core invalidation and pause/resume replace the connection identity, rejecting old records and diagnostics. Capture checks pause before parsing JSON, then rechecks pause and connection identity before appending.

Background capture changes use a coalesced Notify. When Logs is visible, the first change schedules a display deadline 34 ms later; subsequent notifications do not postpone it. Each loop checks the deadline before drawing or waiting, even when input or tasks win the select. Only this deadline and entering or controlling Logs copy snapshots. Local pause, clear, and level changes update immediately and cancel the old deadline. Hidden logs never request ordinary-page redraws; leaving or exiting clears the deadline. Records do not enter the general popup queue. Connection attempts have finite timeouts and interruptible reconnect backoff; the worker handle is retained until it exits. System hostname resolution remains subject to the platform resolver.

Error handling:
- `tri!()` macro — capture errors and display a popup to the user
- `tri!(, or_cancel)` — silently swallow errors

### Popups

Background tasks submit popups through a bounded channel: `let reply = Input::new().with_title(...).build_and_send_async().await;` then `reply.await` waits for the user's answer. Full channels apply backpressure. Synchronous App callbacks collect their popups locally so they cannot block on App's own queue.

`PopUp::check()` and `PopUp::handle_key_event()` manage display and input. Confirm/error messages use the same inbox. Shutdown closes pending replies so waiting operations can finish.

## Core Macros

| Macro | Location | Purpose |
|-------|----------|---------|
| `tri!` | `tab/mod.rs` | Error handling in async callbacks |
| `key_map!` | `key.rs` | Declare default bindings and parsers, without mutable runtime state |
| `newtype_tab!` | `tab/mod.rs` | Generate Tab wrapper, implement `TuiWidget` + `TuiTab` |
| `enum_dispatch!` | `tab/mod.rs` | Dispatch Tab enum variants to trait methods |
| `new_type_impl_tuiwidget!` | `widget/mod.rs` | Auto-implement `TuiWidget` for newtype wrappers |
| `load_save!` | `config/util.rs` | Generate `from_file()` / `to_file()` for YAML config types |

### key_map! Macro

`key_map!(Logs, Action, DefaultBindings::new()...)` declares the scope's action variant and defaults. `DefaultBindings::with_keys` and `with_chords` declare single keys and chords. Configuration resolves nested sections directly to `Scope`, without intermediate path strings. The generated `km::bindings` applies overrides and returns typed `Action` bindings for context composition.

`keymap.yaml` uses lists such as `[{on: j, action: MoveDown, desc: "Move down"}]`. The `on` field accepts a single key or a two-key sequence; `action: null` unbinds it. See [Getting Started](getting_started_en.md) for the configuration contract.

## Features

| Feature | Default | Description |
|---------|---------|-------------|
| `customized-theme` | ☑ | Custom theme support (auto-enables `tui`) |
| `tui` | (indirect) | ratatui + crossterm + tokio dependencies |
| `deprecated` | ☐ | Deprecated functionality |

Use `#[cfg(feature = "tui")]` rather than `#[cfg(feature = "customized-theme")]` for conditional compilation, unless it's theme-specific code.

## Business Logic

The `functions/` directory provides command, file, and REST operations. `tui/core.rs` owns their TUI state and request lifecycle.

| Module | Responsibility |
|--------|----------------|
| `command` | System-level operations: service start/stop (systemd), file permission fixes, open directory, file editor |
| `file` | Profile management: import, update (download + parse), Template expansion, subscription type detection |
| `restful` | REST API: get/set config, switch proxies, query connections, get logs |

## Build Script

`build.rs` generates a version number via git, formatted as `{CARGO_PKG_VERSION}-{git-short-hash}[-dirty]`, stored in the `CLASHTUI_VERSION` environment variable.

## Version Naming Convention

- Crate name is `clashtui` (Cargo.toml)
- Internal identifiers use `clashtui` (`CLASHTUI_VERSION`, `CLASHTUI_CONFIG_DIR`, config directory `~/.config/clashtui`)
- All environment variables, YAML keys, and user-visible strings use `clashtui`

## Adding a Tab

1. Define content implementing `BasicTabContent` and `TabContent` (or the dual-panel traits).
2. Add `Scope`, `Action`, and a configuration loader in `key/runtime.rs`; register the configuration scope in `key/config.rs`.
3. Declare defaults with `key_map!`, generate the wrapper with `newtype_tab!(MyTab(Tab<MyContent>), MyScope)`, and register it in `tab/mod.rs`'s `enum_dispatch!`.
4. Add the tab to `App::new()` and update `TabTarget` and default navigation bindings. Keep their order consistent with the tab list.
5. Dual panels expose separate actions and scopes; `input_scope` selects the effective map by focus.
6. Keep `init` local. Submit lifecycle commands through Business; read committed data in `sync_from` and `render` without starting work. Keep operation cleanup and feedback outside disposable page callbacks.
