## ADDED Requirements

### Requirement: CLI parses --config-dir flag
The system SHALL accept `--config-dir=<PATH>` as an optional flag to override the config directory.

#### Scenario: --config-dir with explicit path
- **WHEN** the program is invoked with `--config-dir=/custom/path`
- **THEN** `cmd.config_dir` is `Some(PathBuf::from("/custom/path"))`

#### Scenario: --config-dir not provided
- **WHEN** the program is invoked without `--config-dir`
- **THEN** `cmd.config_dir` is `None` (before env var merge in `from_env()`)

### Requirement: CLI merges CLASHTUI_CONFIG_DIR env var
The `from_env()` function SHALL use `$CLASHTUI_CONFIG_DIR` as the config directory when `--config-dir` is not provided.

#### Scenario: Env var used when flag absent
- **WHEN** `CLASHTUI_CONFIG_DIR` is set to `/env/path` and `--config-dir` is not provided
- **THEN** `from_env()` sets `config_dir` to `Some(PathBuf::from("/env/path"))`

#### Scenario: Flag overrides env var
- **WHEN** `CLASHTUI_CONFIG_DIR` is set to `/env/path` AND `--config-dir=/flag/path` is provided
- **THEN** `from_env()` sets `config_dir` to `Some(PathBuf::from("/flag/path"))`

### Requirement: CLI parses --generate-shell-completion flag
The system SHALL accept `--generate-shell-completion[=SHELL]` as an optional flag for shell completion generation.

#### Scenario: --generate-shell-completion without value
- **WHEN** the program is invoked with `--generate-shell-completion`
- **THEN** `cmd.generate_shell_completion` is `Some(None)` (shell auto-detected)

#### Scenario: --generate-shell-completion with shell value
- **WHEN** the program is invoked with `--generate-shell-completion=bash`
- **THEN** `cmd.generate_shell_completion` is `Some(Some(Shell::Bash))`

#### Scenario: --generate-shell-completion not provided
- **WHEN** the program is invoked without `--generate-shell-completion`
- **THEN** `cmd.generate_shell_completion` is `None`

### Requirement: CLI parses --verbose count flag
The system SHALL accept `-v` and `--verbose` as a count flag (can be repeated).

#### Scenario: No verbose flag
- **WHEN** the program is invoked without `-v` or `--verbose`
- **THEN** `cmd.verbose` is `0`

#### Scenario: Single -v
- **WHEN** the program is invoked with `-v`
- **THEN** `cmd.verbose` is `1`

#### Scenario: -vv
- **WHEN** the program is invoked with `-vv`
- **THEN** `cmd.verbose` is `2`

#### Scenario: -vvv
- **WHEN** the program is invoked with `-vvv`
- **THEN** `cmd.verbose` is `3`

### Requirement: CLI parses --load-theme-realtime flag (customized-theme feature)
When the `customized-theme` feature is enabled, the system SHALL accept `--load-theme-realtime` as a boolean flag.

#### Scenario: --load-theme-realtime provided
- **WHEN** the program is invoked with `--load-theme-realtime` (and `customized-theme` feature is on)
- **THEN** `cmd.load_theme_realtime` is `true`

#### Scenario: --load-theme-realtime not provided
- **WHEN** the program is invoked without `--load-theme-realtime`
- **THEN** `cmd.load_theme_realtime` is `false`

### Requirement: CLI migrates subcommand only when feature enabled
When the `migration_v0_2_3` feature is enabled, the system SHALL accept `migrate v0_2_3` as a subcommand. When the feature is disabled, the `migrate` subcommand SHALL NOT appear in `--help` or be parseable.

#### Scenario: migrate v0_2_3 with feature enabled
- **WHEN** the program is invoked with `migrate v0_2_3` and the `migration_v0_2_3` feature is on
- **THEN** `cmd.command` is `Some(ArgCommand::Migrate { version: OldVersion::V0_2_3 })`

#### Scenario: migrate fails when feature disabled
- **WHEN** the program is invoked with `migrate` and the `migration_v0_2_3` feature is off
- **THEN** clap returns an error (unrecognized subcommand)

### Requirement: CLI rejects dead subcommands
The system SHALL NOT accept `profile`, `service`, `mode`, or `update` as subcommands. Invoking any of these SHALL produce a clap parse error.

#### Scenario: profile subcommand rejected
- **WHEN** the program is invoked with `profile list`
- **THEN** clap returns an error indicating "unrecognized subcommand 'profile'"

#### Scenario: service subcommand rejected
- **WHEN** the program is invoked with `service stop`
- **THEN** clap returns an error indicating "unrecognized subcommand 'service'"

#### Scenario: mode subcommand rejected
- **WHEN** the program is invoked with `mode rule`
- **THEN** clap returns an error indicating "unrecognized subcommand 'mode'"

#### Scenario: update subcommand rejected
- **WHEN** the program is invoked with `update clashtui`
- **THEN** clap returns an error indicating "unrecognized subcommand 'update'"

### Requirement: CLI tests cover all working flags and subcommands
The system SHALL have unit tests that verify parsing of all remaining CLI flags and the `migrate` subcommand, exercising both success and error paths.

#### Scenario: Test verifies config-dir parsing
- **WHEN** tests are run
- **THEN** at least one test case validates `--config-dir=/some/path`

#### Scenario: Test verifies config-dir env var merging
- **WHEN** tests are run
- **THEN** at least one test case validates that `CLASHTUI_CONFIG_DIR` sets `config_dir` when `--config-dir` is absent

#### Scenario: Test verifies config-dir flag overrides env var
- **WHEN** tests are run
- **THEN** at least one test case validates `--config-dir` takes priority over `CLASHTUI_CONFIG_DIR`

#### Scenario: Test verifies --generate-shell-completion parsing
- **WHEN** tests are run
- **THEN** at least one test case validates `--generate-shell-completion` with and without a shell value

#### Scenario: Test verifies --verbose count parsing
- **WHEN** tests are run
- **THEN** at least one test case validates `-v`, `-vv`, and `-vvv` produce correct counts

#### Scenario: Test verifies migrate subcommand parsing
- **WHEN** tests are run with the `migration_v0_2_3` feature enabled
- **THEN** at least one test case validates `migrate v0_2_3` parses correctly

#### Scenario: Test verifies dead subcommands are rejected
- **WHEN** tests are run
- **THEN** at least one test case validates that `profile`, `service`, `mode`, and `update` produce parse errors
