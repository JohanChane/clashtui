# AGENTS.md

## Build & Run

```sh
cargo build                    # debug build
cargo run                      # run (debug)
cargo run --release            # run (release — LTO, stripped, panic=abort)
cargo check                    # type-check only
cargo test                     # run all tests (inline #[test]; no tests/ dir)
```

No lint/format config files, no CI, no Makefile.

## Naming

The crate is named `demotui` (Cargo.toml), but internal identifiers use the `clashtui` legacy name: `CLASHTUI_VERSION`, `CLASHTUI_CONFIG_DIR`, config dir `~/.config/clashtui`, etc. Follow existing conventions — use `clashtui` in env vars, YAML keys, and user-facing strings.

## Feature Flags

- `default = ["customized-theme"]` → auto-enables `tui` (ratatui + crossterm + tokio)
- `tui` is NOT a direct default — it's enabled transitively via `customized-theme`
- `migration_v0_2_3` and `deprecated` are off by default
- When adding gated code, gate with `#[cfg(feature = "tui")]`, not `#[cfg(feature = "customized-theme")]`, unless it's specifically theme-related

## Edition 2024

This crate uses Rust edition 2024. Notable differences from 2021: `unsafe_op_in_unsafe_fn`, changed closure capture rules, `gen` is a reserved keyword.

## Build Script

`build.rs` generates `CLASHTUI_VERSION` env var from git (`git describe --always --tags` + branch + dirty flag). The full version string is at `env!("CLASHTUI_VERSION")` (used in CLI), not `CARGO_PKG_VERSION`.

## Architecture

Three-phase startup in `src/main.rs`: **CLI parse** → **config init** → **TUI init + serve + restore**.

- `src/cli.rs` — clap derive parser. `from_env()` merges CLI args with `CLASHTUI_CONFIG_DIR` env var.
- `src/config.rs` — loads YAML config + database from config dir. Must be an **existing, non-empty directory**.
- `src/tui.rs` — entry: `init()` (raw mode + theme + agent), `App::serve()` (event loop), `restore()`. Also exports `hold(on: bool)` which temporarily leaves/enters raw mode (used when prompting the user on stdin/stdout during TUI runtime).

### Config Directory

Resolved to (in order): exe-relative `data/` dir if it exists ("portable mode") → `$XDG_CONFIG_HOME/clashtui` → `~/.config/clashtui`. The directory must already exist and be non-empty (`src/config/util.rs:16`).

### Multi-file Modules

Pattern: `modname.rs` re-exports from `modname/` directory. Applies to `src/cli.rs` + `src/cli/`, `src/tui.rs` + `src/tui/`, `src/config.rs` + `src/config/`, `src/functions.rs` + `src/functions/`.

### TUI Event Loop (~50fps, at least 20ms/frame)

Defined in `src/tui/app.rs:82`. Each frame: `terminal.draw(render)` → `sync()` → wait for key/tick/resize via `tokio::select!`. Key routing is four-layer (see `src/tui/app.rs:124`): **PopUp**(0) → **Chord/Which**(1) → **Tab**(2) → **Global**(3). Layer 1 handles multi-key chord shortcuts (e.g. `g g`). Tab takes `&mut self` with no return; Global returns `bool`.

## Adding a New Tab

Requires edits in these places:

1. Define content type implementing `BasicTabContent` + `TabContent` (see `docs/dev.md` for full trait details)
2. In `src/tui/tab/mod.rs`: add `newtype_tab!`, add variant to `enum_dispatch!` and `prelude`
3. In `src/tui/app.rs`: add to `tabs` vec, update the local `TAB_COUNT` const and the `'1'..='5'` char range in `handle_global_kv`, add agent init call in `prelude::agent_init`
4. If dual-pane, implement `DualTabContent` / `DualTabContentMate` (see `src/tui/widget/dualtab.rs`)

`newtype_tab!` has two forms:
- `newtype_tab!(MyTab(Tab<MyContent>))` — derives title from `$inner::TITLE`
- `newtype_tab!(MyTab(DualTab<A, B>), "Display Name")` — uses explicit title literal

## Key Conventions

- **Never mutate in `render`** — it takes `&self`. Apply state changes in `handle_key_event` or via `sync()` callbacks.
- **`sync()` runs after `render` each frame** — so current frame shows state from the last sync cycle.
- **Async I/O**: spawn into `FutureSet` (a `JoinSet<Callback>`) via `.spawn_at(tasks)`. Callbacks are `Box<dyn FnOnce(&mut Content)>`. The event loop auto-advances them in `sync()`.
- **Error handling in async blocks**: use `tri!()` for user-visible errors (shows Confirm popup), `tri!(, or_cancel)` for silent cancel on error.
- **Keymaps**: `mod_agent!` macro in each tab module defines default key bindings. Users can override via `keymap.yaml`. See `src/tui/agent.rs`.
- **PopUp**: one-shot channel pattern. Build via `Input::new().with_title(...).build_and_send().await`. Only use PopUp when user input is needed — use inline status for simple confirmations/errors.

## Macros

Custom macros (defined in `src/tui/tab/mod.rs`, `src/tui/widget/mod.rs`, `src/config/util.rs`):
- `tri!` / `tri!(, or_cancel)` — error handling in async callbacks
- `mod_agent!` — per-tab key binding defaults
- `newtype_tab!` — boilerplate for tab wrapper types (two forms: with/without explicit title)
- `enum_dispatch!` — dispatches `TuiWidget` + `TuiTab` to enum variants
- `new_type_impl_tuiwidget!` — auto-implements `TuiWidget` for newtype wrappers
- `load_save!` — YAML file load/save for config types

## Known .gitignore Exclusions

`src/actor.rs`, `src/actor/`, `src/backend.rs`, `src/backend/` are gitignored — these are ClashTUI legacy modules intentionally kept out of version control. Don't create files at those paths.

## OpenSpec

This repo uses OpenSpec (`openspec/config.yaml`) with `schema: spec-driven`. Active changes live in `openspec/changes/`. Run `openspec` commands or use the openspec skills in `.opencode/skills/`.
