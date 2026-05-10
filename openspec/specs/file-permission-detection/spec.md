# file-permission-detection Specification

## Purpose
TBD - created by archiving change migrate-mihomo-file-permission-management. Update Purpose after archive.
## Requirements
### Requirement: Detect missing setgid bit on config directory
The system SHALL check whether the mihomo config directory has the setgid (g+s, mode bit 0o2000) permission set.

#### Scenario: Setgid is set
- **WHEN** the config directory's permissions mode includes `0o2000`
- **THEN** the setgid check passes

#### Scenario: Setgid is missing
- **WHEN** the config directory's permissions mode does NOT include `0o2000`
- **THEN** the system reports that setgid is missing

### Requirement: Detect files with wrong group ownership
The system SHALL recursively check all files and directories under the mihomo config directory to verify they belong to the same group as the config directory itself.

#### Scenario: All files share the same group
- **WHEN** every file and subdirectory under the config directory has the same GID as the config directory
- **THEN** the group ownership check passes

#### Scenario: Some files have a different group
- **WHEN** one or more files or subdirectories have a GID different from the config directory's group
- **THEN** the system reports those paths as having wrong group ownership

### Requirement: Detect files missing group-writable permission
The system SHALL recursively check all files and directories under the mihomo config directory to verify they have group-writable (g+w, mode bit 0o0020) permission.

#### Scenario: All files are group-writable
- **WHEN** every file and subdirectory under the config directory has `0o0020` in its permission mode
- **THEN** the group-writable check passes

#### Scenario: Some files are not group-writable
- **WHEN** one or more files or subdirectories are missing `0o0020` in their permission mode
- **THEN** the system reports those paths as missing group-writable permission

### Requirement: Combined permission status check
The system SHALL provide a single function that returns whether all permission checks (setgid, group ownership, group-writable) pass.

#### Scenario: All permissions correct
- **WHEN** setgid is set, all files belong to the same group, and all files are group-writable
- **THEN** the function returns true (permissions are correct)

#### Scenario: Any permission incorrect
- **WHEN** at least one check fails (missing setgid, wrong group, or missing g+w)
- **THEN** the function returns false (repair is needed)

### Requirement: Null-safe detection for missing directories
The system SHALL handle the case where the config directory does not exist or is inaccessible gracefully.

#### Scenario: Config directory does not exist
- **WHEN** the config directory path does not exist on the filesystem
- **THEN** the permission check returns true (nothing to check, nothing to repair)

