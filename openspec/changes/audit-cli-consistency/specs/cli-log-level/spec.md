## ADDED Requirements

### Requirement: --verbose flag controls log filter level
The system SHALL map the `--verbose` / `-v` count flag to `env_logger` filter levels: 0 (no flag) → `warn`, 1 (`-v`) → `info`, 2 (`-vv`) → `debug`, 3+ (`-vvv` or more) → `trace`. The log level SHALL be applied during `env_logger` initialization in `main.rs`.

#### Scenario: Default log level is warn
- **WHEN** the program is invoked without `--verbose` or `-v`
- **THEN** `env_logger` is initialized with a default filter of `warn`

#### Scenario: Single -v sets info level
- **WHEN** the program is invoked with `-v` or `--verbose`
- **THEN** `env_logger` is initialized with a filter of `info`

#### Scenario: Double -v sets debug level
- **WHEN** the program is invoked with `-vv`
- **THEN** `env_logger` is initialized with a filter of `debug`

#### Scenario: Triple -v sets trace level
- **WHEN** the program is invoked with `-vvv`
- **THEN** `env_logger` is initialized with a filter of `trace`

#### Scenario: Excessive -v saturates at trace
- **WHEN** the program is invoked with `-vvvv` (4+ times)
- **THEN** `env_logger` is initialized with a filter of `trace`

#### Scenario: Combined short and long flags
- **WHEN** the program is invoked with `--verbose -v`
- **THEN** `env_logger` is initialized with a filter of `debug` (count = 2)
