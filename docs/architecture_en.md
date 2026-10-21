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
│       ├── app.rs       # App struct, event loop (~50fps), key routing
│       ├── key.rs       # Keys, default-binding macro, and loading entry point
│       ├── key/
│       │   ├── config.rs    # Parsing, overrides, unbinding, scope validation
│       │   ├── effective.rs # Page/focus maps and global conflict validation
│       │   ├── matcher.rs   # Pending-chord state owned by App
│       │   └── notation.rs  # Key notation parsing and formatting
│       ├── signals.rs   # OS signal handling
│       ├── term.rs      # Terminal raw mode enter/exit/suspend
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
│           ├── logs.rs       # Logs tab
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

Runs in `App::serve()`, looping at ~50fps (20ms/frame):

```
Per-frame flow:
1. Handle resize (atomic flag, processed at frame top to avoid races)
2. terminal.draw(render) — render current frame
3. sync() — advance completed async tasks
4. tokio::select! wait for next event (key/tick/resize)
5. Handle key event
```

### Effective Binding Maps and Input State

`key_map!` declares typed defaults and configuration parsers. `Keymaps::from_config` applies core-specific scopes, default overrides, and unbindings, then checks each page against global bindings. Identical complete sequences and single-key/chord prefix overlaps stop startup with an error; `gp` and `gg` can share a prefix.

Each page/focus context has an immutable `EffectiveKeymap`. Profile and Template get separate maps; Status uses the global map. The maps are installed in a read-only `OnceLock` only after every context passes validation.

Input handling:

1. Popups capture raw keys and cancel any pending chord.
2. Help handles scrolling and dismissal. Starting a global chord closes help and retains only global candidates for that sequence.
3. App's `Matcher` uses the effective map and returns `Pending`, `Execute(Action)`, `Cancelled`, or `Unmatched`.
4. App executes global actions or passes typed page actions to `TuiTab::handle_action`.

App owns the only mutable pending-chord state. Tab changes, focus changes, and popups do not inherit old candidates. Esc and unmatched second keys are consumed. Which, help, and tab labels read the same effective map. App displays the pending group name in the top bar; pages need no matching state.

### TuiWidget Trait

`TuiWidget` provides the UI lifecycle:

- `render(&mut self, f, area)` — draw UI
- `sync(&mut self)` — advance async tasks
- `on_enter(&mut self)` / `on_leave(&mut self)` — tab switch callbacks

`TuiTab` additionally exposes `title`, `input_scope`, and `handle_action`. Page containers receive resolved actions. Text popups handle raw keys through their own `handle_key_event` methods.

### Tab System

#### Single Panel (Tab)

`Tab<C>` is a generic container. `C` must implement two traits:

- `BasicTabContent` — defines the typed action enum `Key`, `State` type, title
- `TabContent` — defines `init`, `handle_key_event`, `render`

#### Dual Panel (DualTab)

`DualTab<C1, C2>` is used for scenarios needing two panel switching (e.g. Files tab with Profile and Template). The two content types reference each other via `DualTabContent` / `DualTabContentMate` traits.

#### Tab Enum

The `enum_dispatch!` macro unifies all tabs into a single `Tab` enum. Each variant uses the `newtype_tab!` macro to generate a wrapper and implement `TuiWidget` and `TuiTab`.

### Async Task Model

Async I/O operations are managed via `FutureSet<C>` (i.e. `tokio::task::JoinSet`):

- Spawn async tasks in `handle_key_event` or `init` via `task_set.spawn(async { ... })`
- Completed tasks produce `Callback<C>` (i.e. `Box<dyn FnOnce(&mut C)>`)
- `sync()` advances completed callbacks each frame; state changes happen here uniformly

Error handling:
- `tri!()` macro — capture errors and display a popup to the user
- `tri!(, or_cancel)` — silently swallow errors

### Popups

Popups use a `oneshot` channel pattern:
- Call `Input::new().with_title(...).build_and_send().await` to block and wait for user input
- Popup events are managed by `PopUp::check()` and `PopUp::handle_key_event()`
- Use popups only when user input is required; simple confirm/error uses inline state display

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

`key_map!(Logs, Key, DefaultMap::new()...)` declares the scope's action variant and defaults. The generated `km::bindings` applies overrides and returns typed `Action` bindings for context composition.

`keymap.yaml` uses lists such as `[{on: j, action: MoveDown, desc: "Move down"}]`. The `on` field accepts a single key or a two-key sequence; `action: null` unbinds it. See [Getting Started](getting_started_en.md) for the configuration contract.

## Features

| Feature | Default | Description |
|---------|---------|-------------|
| `customized-theme` | ☑ | Custom theme support (auto-enables `tui`) |
| `tui` | (indirect) | ratatui + crossterm + tokio dependencies |
| `deprecated` | ☐ | Deprecated functionality |

Use `#[cfg(feature = "tui")]` rather than `#[cfg(feature = "customized-theme")]` for conditional compilation, unless it's theme-specific code.

## Business Logic

The `functions/` directory contains all business logic, divided into three modules:

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
2. Add `Scope`, `Action`, and a configuration loader in `key/effective.rs`; register the configuration scope in `key/config.rs`.
3. Declare defaults with `key_map!`, generate the wrapper with `newtype_tab!(MyTab(Tab<MyContent>), MyScope)`, and register it in `tab/mod.rs`'s `enum_dispatch!`.
4. Add the tab to `App::new()` and update `TabTarget` and default navigation bindings. Keep their order consistent with the tab list.
5. Dual panels expose separate actions and scopes; `input_scope` selects the effective map by focus.
