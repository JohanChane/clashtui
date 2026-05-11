## Why

The CLI argument parser (clap) defines 5 subcommands (`profile`, `service`, `mode`, `update`, `migrate`) and a `--verbose` flag, but `--verbose` has no effect on logging output and 4 subcommands silently launch the TUI because their handler module (`handler.rs`) is commented out and references nonexistent modules (`backend`, `consts`, `utils::self_update`). Additionally, there are zero test cases for CLI argument parsing. This leaves the CLI in a broken state where advertised functionality doesn't work and regressions can't be caught.

## What Changes

- **Fix `--verbose` / `-v` flag**: Wire it to `env_logger` so `-v` sets a useful filter level (e.g., `info`), `-vv` sets `debug`, etc., matching the documented behavior "increase log level, default is Warning"
- **Remove dead subcommand handler code**: Delete `src/cli/handler.rs` and its commented-out re-exports in `src/cli.rs`, since the handler references modules (`backend`, `consts`, `utils::self_update`) that don't exist or are gitignored
- **Remove dead subcommands that cannot be implemented**: The `profile`, `service`, `mode`, and `update` subcommands depend on the nonexistent `backend` module. Remove them from the `ArgCommand` enum so `clashtui --help` shows only working commands. **BREAKING**: Users invoking `clashtui profile`, `clashtui service`, `clashtui mode`, or `clashtui update` will get an error instead of silently launching TUI
- **Clean up `migrate` subcommand**: Remove the `NotSupported` variant — when the `migration_v0_2_3` feature is off, hide the entire `Migrate` subcommand instead of showing a useless `not-supported` option
- **Add CLI parsing tests**: Cover argument parsing for all remaining flags/subcommands, env var merging (`CLASHTUI_CONFIG_DIR`), `--generate-shell-completion`, and `--verbose` flag behavior
- **Remove commented-out `Mode` conversion** at `src/cli.rs:197-206`

## Capabilities

### New Capabilities
- `cli-log-level`: `--verbose` / `-v` flag controls the log filter level via `env_logger`, with `-v` → `info`, `-vv` → `debug`, `-vvv` → `trace`, and default (no flag) → `warn`
- `cli-parsing-tests`: Unit tests covering CLI argument parsing including all remaining flags (`--config-dir`, `--generate-shell-completion`, `--verbose`, `--load-theme-realtime`), env var merging (`CLASHTUI_CONFIG_DIR`), and the `migrate` subcommand when feature-gated

### Modified Capabilities
_None_ — no existing spec requirements change.

## Impact

- `src/cli.rs`: Remove dead subcommands (`profile`, `service`, `mode`, `update`), clean up commented code, remove `NotSupported` variant
- `src/cli/handler.rs`: **Deleted** (dead code referencing nonexistent modules)
- `src/main.rs`: Wire `cmd.verbose` to `env_logger` filter level
- `Cargo.toml`: Remove commented-out dependencies (`is-root`, `self-replace`) if no longer referenced
- No API changes; no dependency changes
