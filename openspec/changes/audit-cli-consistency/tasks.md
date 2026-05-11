## 1. Remove dead subcommand code

- [ ] 1.1 Delete `src/cli/handler.rs` (unreachable dead code referencing nonexistent modules)
- [ ] 1.2 Remove `// mod handler;` and `// pub use handler::handle_cli;` lines from `src/cli.rs`
- [ ] 1.3 Remove `Profile`, `Service`, `Mode`, `Update` variants from `ArgCommand` enum in `src/cli.rs`
- [ ] 1.4 Remove related dead enum definitions: `Target`, `ProfileCommand`, `ServiceCommand`, `ModeCommand` from `src/cli.rs`
- [ ] 1.5 Remove the commented-out `Mode` conversion block (`impl From<ModeCommand> for Mode`) from `src/cli.rs`
- [ ] 1.6 Gate the `Migrate` variant with `#[cfg(feature = "migration_v0_2_3")]` so it only appears when the feature is enabled
- [ ] 1.7 Remove the `NotSupported` variant from `OldVersion` enum (no longer needed since Migrate is feature-gated)

## 2. Wire --verbose flag to env_logger

- [ ] 2.1 Add a helper function in `src/cli.rs` that maps `verbose: u8` count to a log level string: 0 → `warn`, 1 → `info`, 2 → `debug`, 3+ → `trace`
- [ ] 2.2 In `src/main.rs`, use the helper to override `env_logger`'s default filter based on `cmd.verbose` count, after `config::init()` succeeds

## 3. Add CLI parsing tests

- [ ] 3.1 Add a test module in `src/cli.rs` (or a `src/cli/tests.rs` file with `#[cfg(test)] mod tests;` in `src/cli.rs`)
- [ ] 3.2 Test `--config-dir=<path>` flag parsing (presence and absence)
- [ ] 3.3 Test `CLASHTUI_CONFIG_DIR` env var merging in `from_env()`: env var alone, flag overriding env var
- [ ] 3.4 Test `--generate-shell-completion` flag: without value, with shell value (e.g. `bash`), and absent
- [ ] 3.5 Test `--verbose` / `-v` count flag: `-v`, `-vv`, `-vvv`, `--verbose`, combined `--verbose -v`
- [ ] 3.6 Test `--verbose` to log-level mapping helper: verify 0→warn, 1→info, 2→debug, 3→trace
- [ ] 3.7 Test `migrate v0_2_3` subcommand parses correctly when `migration_v0_2_3` feature is enabled
- [ ] 3.8 Test that `profile`, `service`, `mode`, `update` subcommands are rejected with a parse error
- [ ] 3.9 Test `--load-theme-realtime` flag parsing (when `customized-theme` feature is on, the default)

## 4. Verify

- [ ] 4.1 Run `cargo check` to ensure the project compiles without errors
- [ ] 4.2 Run `cargo test` to verify all 81+ existing tests still pass and new tests pass
- [ ] 4.3 Run `cargo clippy` (if available) or manual review to check for warnings
- [ ] 4.4 Verify `cargo run -- --help` output shows only working flags and subcommands
