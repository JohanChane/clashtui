# external-directory-open Specification

## Purpose
TBD - created by archiving change migrate-pextra-keybindings. Update Purpose after archive.
## Requirements
### Requirement: open_dir spawns external file manager

The system SHALL provide a public function `open_dir(path: &str) -> Result<()>` that reads the configured `open_dir_cmd`, substitutes `%s` with the given path, and spawns the resulting command via `sh -c` with stdout/stderr detached. The function SHALL mirror the existing `edit()` function's pattern.

#### Scenario: open_dir with custom command
- **WHEN** `open_dir_cmd` is `kitty -e yazi "%s"` and `open_dir("/path/to/dir")` is called
- **THEN** the system spawns `sh -c 'kitty -e yazi "/path/to/dir"'` as a detached process

#### Scenario: open_dir with default command on Linux/macOS
- **WHEN** `open_dir_cmd` is `open %s` (default) and `open_dir("/tmp")` is called
- **THEN** the system spawns `sh -c 'open /tmp'` as a detached process

#### Scenario: open_dir with default command on Windows
- **WHEN** `open_dir_cmd` is `start %s` (default) and `open_dir("C:\\foo")` is called
- **THEN** the system spawns `sh -c 'start C:\foo'` as a detached process

### Requirement: Ctrl+g c chord opens demotui config directory

The system SHALL bind `Ctrl+g` followed by `c` (lowercase, no modifiers) as a global chord that calls `open_dir()` with the demotui config directory path. The chord SHALL be processed at a global chord layer between PopUp and Help.

#### Scenario: Ctrl+g c opens app config directory
- **WHEN** the user presses `Ctrl+g` then `c` in sequence
- **THEN** `open_dir()` is called with `config::config_dir_path()`

#### Scenario: Cancel chord on non-matching second key
- **WHEN** the user presses `Ctrl+g` then any key other than `c` or `m`
- **THEN** the chord is cancelled and the second key falls through to normal handling

### Requirement: Ctrl+g m chord opens clash config directory

The system SHALL bind `Ctrl+g` followed by `m` (lowercase, no modifiers) as a global chord that calls `open_dir()` with the clash config directory path.

#### Scenario: Ctrl+g m opens clash config directory
- **WHEN** the user presses `Ctrl+g` then `m` in sequence
- **THEN** `open_dir()` is called with `CONFIG.cfg_file.basic.clash_config_dir`

### Requirement: Help panel documents global chords

The help panel's global shortcuts section SHALL include entries for `Ctrl+g c` (Open app config directory) and `Ctrl+g m` (Open clash config directory).

#### Scenario: Help panel shows new global chords
- **WHEN** the help panel is rendered (user presses `?`)
- **THEN** the global shortcuts list includes `Ctrl+g c` with description "Open app config directory" and `Ctrl+g m` with description "Open clash config directory"

### Requirement: config_dir_path exposes config directory

The system SHALL provide a public function `config_dir_path() -> PathBuf` in `config.rs` that returns the resolved config directory path, consistent with the existing `template_path()`, `profile_yamls_path()` pattern.

#### Scenario: config_dir_path returns DATA_DIR
- **WHEN** `config_dir_path()` is called after config init
- **THEN** it returns the canonicalized path stored in `DATA_DIR`

### Requirement: Template tab supports e key to edit template files

The Template tab SHALL bind the `e` key (lowercase, no modifiers) to an `Edit` action that opens the currently selected template file in the external editor via the configured `edit_cmd`. The implementation SHALL follow the same pattern as the existing Profile tab edit (enum variant, `mod_agent!` binding, `act()` dispatch, `async fn _edit` callback).

#### Scenario: e on Template tab edits the selected template
- **WHEN** the user selects a template and presses `e` with no modifiers
- **THEN** the system resolves the template path as `config::template_path().join(template_name)` and calls `edit()` with that path

#### Scenario: e on Template tab with non-existent template
- **WHEN** the user presses `e` on a template whose file has been deleted
- **THEN** the `edit()` function spawns the editor command with the path regardless (editor shows "new file" or "not found")

