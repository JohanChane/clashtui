## Context

The `Cmds` struct (src/cli.rs) defines 5 subcommands and a `--verbose` counter flag via clap. Currently:
- `--verbose` is parsed but never consumed — `env_logger` is hardcoded to `warn` level regardless
- 4 subcommands (`profile`, `service`, `mode`, `update`) are parsed but their handler (`src/cli/handler.rs`) is commented out of the build; invoking them silently launches the TUI
- `handler.rs` references modules that don't exist (`backend`, `consts`, `utils::self_update`), making it unrecoverable dead code
- No test coverage exists for CLI argument parsing
- The `migrate` subcommand shows a useless `not-supported` variant when the `migration_v0_2_3` feature is off

The project uses Rust edition 2024 with clap (derive) for CLI parsing, `env_logger` for logging, and `clap_complete` for shell completions.

## Goals / Non-Goals

**Goals:**
- Wire `--verbose` / `-v` to `env_logger` filter so repeated flags increase log verbosity
- Remove dead subcommand code (`handler.rs`, the 4 dead `ArgCommand` variants) so `--help` reflects reality
- Clean up the `migrate` subcommand so it only appears when the feature is enabled
- Add unit tests for CLI argument parsing covering all working flags and the `migrate` subcommand
- Remove commented-out code (`Mode` conversion, re-exports)

**Non-Goals:**
- Re-implementing `profile`, `service`, `mode`, or `update` subcommands (requires the nonexistent `backend` module)
- Changing the `--generate-shell-completion` behavior
- Changing the `--load-theme-realtime` behavior
- Adding integration tests that actually launch the TUI

## Decisions

### D1: Map `--verbose` count to `env_logger` filter levels

- `0` (default, no flag) → `warn`
- `1` (`-v`) → `info`
- `2` (`-vv`) → `debug`
- `3+` (`-vvv`) → `trace`

**Rationale**: Matches the documented behavior ("increase log level, default is Warning") and follows Rust CLI conventions (e.g., cargo, rustc). The mapping converts the count to a level string before `env_logger` initialization in `main.rs`.

**Alternative considered**: Setting `RUST_LOG` env var. Rejected because it would interfere with user-set `RUST_LOG` and is less explicit than programmatic configuration.

### D2: Remove dead subcommands entirely instead of keeping them

Remove `profile`, `service`, `mode`, and `update` variants from `ArgCommand`. Delete `handler.rs` and the commented re-exports in `cli.rs`. Remove the `Target`, `ProfileCommand`, `ServiceCommand`, and `ModeCommand` enums since they are only used by the dead handler.

**Rationale**: Keeping non-functional subcommands in `--help` is misleading. Since the required `backend` module is gitignored and not available, there's no path to making them work. Users currently get silent TUI launch instead of an error — removing them makes the failure explicit.

**Alternative considered**: Adding a stub error message for each. Rejected — it adds maintenance burden for code paths that can never work.

### D3: Conditionally gate the entire `Migrate` subcommand

Instead of showing `migrate not-supported` when the feature is off, use `#[cfg(feature = "migration_v0_2_3")]` on the `Migrate` variant so it only appears in `--help` when useful.

**Rationale**: Improves UX — users don't see a subcommand that can only fail. The `OldVersion` enum becomes feature-gated and no longer needs the `NotSupported` variant.

### D4: Test strategy — clap-level parsing tests only

Tests use `clap::Parser::try_parse_from()` with string arguments to verify parsing behavior. No TUI initialization or config loading is needed.

**Rationale**: Fast, no setup overhead, directly tests what users type. Integration-level end-to-end tests would require a config directory and are out of scope.

## Risks / Trade-offs

- **[BREAKING] Users invoking dead subcommands will get a clap error instead of silently launching TUI** → Low risk: these subcommands never did anything; the TUI launch was misleading. The error message from clap is clear ("unrecognized subcommand").
- **Log file location timing**: `env_logger` is initialized after `config::init()`, so `--verbose` only affects logs written to the file — any early output goes to stderr directly. This is acceptable; `--verbose` primarily controls the file log verbosity.
- **No migration path for removed subcommands**: No migration needed since the functionality never existed in this build.
