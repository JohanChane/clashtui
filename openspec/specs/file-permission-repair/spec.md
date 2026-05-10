# file-permission-repair Specification

## Purpose
TBD - created by archiving change migrate-mihomo-file-permission-management. Update Purpose after archive.
## Requirements
### Requirement: Repair missing setgid on config directory
The system SHALL repair a missing setgid bit on the mihomo config directory by adding `0o2020` (setgid + group-read + group-write) to the directory's mode bits via `chmod`.

#### Scenario: Setgid successfully added
- **WHEN** the repair function is invoked on a directory missing setgid
- **THEN** the directory permissions are updated to include the setgid bit, and the system reports success

#### Scenario: Setgid chmod fails
- **WHEN** `chmod` fails (e.g., insufficient permissions even with sudo, or read-only filesystem)
- **THEN** the system returns an error describing the failure

### Requirement: Repair wrong group ownership
The system SHALL change the group ownership of files and directories under the config directory to match the config directory's own group, using `chown :group`.

#### Scenario: All files corrected to proper group
- **WHEN** repair is invoked with files having wrong group ownership
- **THEN** each file's group is changed to match the config directory's group, and the system reports success

#### Scenario: chown fails for a file
- **WHEN** `chown` fails for a specific file (e.g., permission denied)
- **THEN** the system reports the error for that file and continues processing remaining files

### Requirement: Repair missing group-writable permission
The system SHALL add group-writable (`0o0020`) permission to files and directories under the config directory that are missing it.

#### Scenario: Group-writable added to all files
- **WHEN** repair is invoked on files missing group-writable permission
- **THEN** each file's mode is updated to include `0o0020`, and the system reports success

#### Scenario: chmod fails for a file
- **WHEN** `chmod` fails for a specific file
- **THEN** the system reports the error for that file and continues processing remaining files

### Requirement: Full repair executes via sudo
The system SHALL execute permission repair operations via `sudo` since changing file ownership and certain permission bits requires root privileges.

#### Scenario: Sudo with interactive password
- **WHEN** the user confirms repair and sudo requires a password
- **THEN** the system uses `tui::hold(true)` to exit raw mode, runs `sudo chmod`/`sudo chown` interactively, then calls `tui::hold(false)` to re-enter raw mode

#### Scenario: Sudo fails
- **WHEN** sudo authentication fails or the user cancels
- **THEN** the system returns an error and does not modify any files

### Requirement: Repair order is idempotent
The system SHALL apply the three repair steps (setgid → group ownership → group-writable) in order. Running repair on already-correct permissions SHALL be a no-op.

#### Scenario: Repair on already-correct permissions
- **WHEN** all three permission aspects are already correct
- **THEN** no file modifications occur and the system reports success

