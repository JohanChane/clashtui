# AGENTS.md

## Build & Run

```sh
cargo build                    # debug build
cargo run                      # run (debug)
cargo run --release            # run (release — LTO, stripped, panic=abort)
cargo check                    # type-check only
cargo test                     # run all 81 tests (inline #[test]; no tests/ dir)
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

Five-phase startup in `src/main.rs`:
1. **CLI parse** — `cli::from_env()` + `handle_early_exit()`
2. **config init** — `config::init(cmd.config_dir)` loads YAML config + database
3. **TUI init** — `tui::init()` (agent keymap + theme + raw mode + panic hook)
4. **serve** — `App::serve()` (event loop)
5. **restore + save** — `tui::restore()` then `config::CONFIG.save()`

- `src/cli.rs` — clap derive parser. `from_env()` merges CLI args with `CLASHTUI_CONFIG_DIR` env var.
- `src/config.rs` — loads YAML config + database from config dir. Must be an **existing, non-empty directory** (`src/config.rs:100-103`). Access live config via `config::CONFIG` (derefs to `Config`).
- `src/tui.rs` — entry: `init()` (agent::init → Theme::load → raw_mode::setup → set_panic_hook), `App::serve()` (event loop), `restore()`. Also exports `hold(on: bool)` which temporarily leaves/enters raw mode (used when prompting the user on stdin/stdout during TUI runtime).

### Config Directory

Resolved to (in order): exe-relative `data/` dir if it exists ("portable mode") → `$XDG_CONFIG_HOME/clashtui` → `~/.config/clashtui`. The directory must already be a non-empty directory.

Config files live inside this dir:
- `config.yaml` — main config (loaded by `ConfigFile::from_file()` in `config::util.rs`)
- `keymap.yaml` — per-tab key remappings (loaded by `agent::init()` in `src/tui/agent.rs`)
- `theme.yaml` — custom theme (only effective with `customized-theme` feature)
- `core_override_config.yaml` — core override config (overwrites profile top-level keys on select)
- `clashtui.db` — profile manager database (saved on exit via `config::CONFIG.save()`)
- `clashtui.log` — log output (writes via `env_logger` with `Pipe` target)

### Multi-file Modules

Pattern: `modname.rs` re-exports from `modname/` directory. Applies to `src/cli.rs` + `src/cli/`, `src/tui.rs` + `src/tui/`, `src/config.rs` + `src/config/`, `src/functions.rs` + `src/functions/`.

Some TUI modules use a hybrid: `src/tui/tab/` contains both `status.rs` (single file) and `proxies/` + `proxies.rs` (multi-file module pattern). The same hybrid applies to `files/` + `files.rs`.

### TUI Event Loop (~50fps, at least 20ms/frame)

Defined in `src/tui/app.rs:119` (`serve()` method, loop at line 128). Each frame:
1. Handle deferred resize via `RESIZE` atomic flag
2. `terminal.draw(render)` — render current frame
3. `sync()` — advance completed async tasks
4. Wait for key/tick/resize via `tokio::select!`
5. Process key event (Press only, Release is ignored)

Key routing is **six-layer** (`src/tui/app.rs:169`):

| Layer | Index | Handler | Purpose |
|-------|-------|---------|---------|
| PopUp | 0 | `popup.handle_key_event` | Modal dialogs steal all input |
| GlobalChord | 0.5 | `global_chord.handle` | Hardcoded chords: `Ctrl-g c` (open config dir), `Ctrl-g m` (open clash dir) |
| Help | 1 | `help.dismiss` | Dismiss help panel on any key |
| Which/Chord | 2 | `chord.handle` | Per-tab chord shortcuts (e.g. multi-key sequences) |
| Tab | 3 | `tabs[ti].handle_key_event` | Active tab handles input |
| Global | 4 | `handle_global_kv` | Tab switch (`1`-`6`, `Tab`), quit (`q`, `Ctrl-c`), help toggle (`?`) |

### Existing Tabs (6 total)

| Key | Tab | Type |
|-----|-----|------|
| `1` | StatusTab | `Tab<StatusContent>` |
| `2` | FileTab | `DualTab<ProfileContent, TemplateContent>` |
| `3` | ProxiesTab | `Tab<ProxiesContent>` |
| `4` | ConnectionsTab | `Tab<ConnectionsContent>` |
| `5` | SettingsTab | `Tab<SettingsContent>` |
| `6` | SrvCtlTab | `Tab<SrvCtlContent>` |

## Adding a New Tab

Requires edits in these places:

1. Define content type implementing `BasicTabContent` + `TabContent` (see `docs/dev.md` for full trait details)
2. In `src/tui/tab/mod.rs`: add `mod mytab;` or `mod mytab/` + `mod mytab.rs`, add `newtype_tab!`, add variant to `enum_dispatch!` and `prelude`
3. In `src/tui/app.rs`: add to `tabs` vec in `App::new()`, update `TAB_COUNT` const and `'1'..='6'` char range in `handle_global_kv`, add agent init call in `prelude::agent_init`
4. If dual-pane, implement `DualTabContent` / `DualTabContentMate` (see `src/tui/widget/dualtab.rs`)

`newtype_tab!` has two forms:
- `newtype_tab!(MyTab(Tab<MyContent>))` — derives title from `$inner::TITLE`
- `newtype_tab!(MyTab(DualTab<A, B>), "Display Name")` — uses explicit title literal

## Key Conventions

- **Never mutate in `render`** — it takes `&self`. Apply state changes in `handle_key_event` or via `sync()` callbacks.
- **`sync()` runs after `render` each frame** — so current frame shows state from the last sync cycle.
- **Async I/O**: spawn into `FutureSet` (a `JoinSet<Callback>`) via `.spawn_at(tasks)`. Callbacks are `Box<dyn FnOnce(&mut Content)>`. The event loop auto-advances them in `sync()`.
- **Error handling in async blocks**: use `tri!()` for user-visible errors (shows Confirm popup), `tri!(, or_cancel)` for silent cancel on error.
- **Keymaps**: `mod_agent!` macro in each tab module defines default key bindings. Users can override via `keymap.yaml` in config dir. See `src/tui/agent.rs`.
- **PopUp**: one-shot channel pattern. Build via `Input::new().with_title(...).build_and_send().await`. Only use PopUp when user input is needed — use inline status for simple confirmations/errors.
- **Key struct** (`src/tui/key.rs`): contains `code`, `shift`, `ctrl`, `alt`, `super_`. String format: plain chars (`a`), uppercase for shift (`A`), or `<C-S-x>` for modifiers. `From<KeyEvent>` normalizes shift for char keys.
- **Resize**: sets `RESIZE` atomic flag in event handler; processed at top of next frame loop (avoids mid-frame resize issues).
- **FULL_RENDER**: used by `hold()` to force terminal clear when leaving/entering raw mode.
- **Global chord shortcuts** (`Ctrl-g c`, `Ctrl-g m`) are hardcoded in `GLOBAL_CHORD_SHORTCUTS`, not user-configurable.

## Macros

Custom macros (defined in `src/tui/tab/mod.rs`, `src/tui/widget/mod.rs`, `src/config/util.rs`):
- `tri!` / `tri!(, or_cancel)` — error handling in async callbacks
- `mod_agent!` — per-tab key binding defaults + shortcut chords
- `newtype_tab!` — boilerplate for tab wrapper types (two forms: with/without explicit title)
- `enum_dispatch!` — dispatches `TuiWidget` + `TuiTab` to enum variants
- `new_type_impl_tuiwidget!` — auto-implements `TuiWidget` for newtype wrappers
- `load_save!` — YAML file load/save for config types

## Known .gitignore Exclusions

`src/actor.rs`, `src/actor/`, `src/backend.rs`, `src/backend/` are gitignored — these are ClashTUI legacy modules intentionally kept out of version control. Don't create files at those paths.

## OpenSpec

This repo uses OpenSpec (`openspec/config.yaml`) with `schema: spec-driven`. Active changes live in `openspec/changes/`. Run `openspec` commands or use the openspec skills in `.opencode/skills/`.

## Documentation

Key docs are in `docs/`:
- `docs/dev.md` — tab development guide, trait details
- `docs/design.md` — overall design
- `docs/get_started.md` — setup guide
- `docs/support_singbox/` — sing-box vs mihomo comparison (`cmd.md`, `config.md`, `api_data.md`) and support analysis (`singbox_support.md`)
