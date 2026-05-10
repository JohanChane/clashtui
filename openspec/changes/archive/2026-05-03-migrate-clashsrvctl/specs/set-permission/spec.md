## ADDED Requirements

### Requirement: Set capabilities on clash binary
The system SHALL run `setcap cap_net_admin,cap_net_bind_service=+ep` on the clash binary path from config.

#### Scenario: is_user=true (no sudo needed)
- **WHEN** `service.is_user` is `true`
- **THEN** executes `setcap cap_net_admin,cap_net_bind_service=+ep <clash_bin_path>` directly without sudo

#### Scenario: is_user=false (needs sudo)
- **WHEN** `service.is_user` is `false`
- **THEN** first requests a sudo password via the TUI password input popup, then executes `sudo -S setcap cap_net_admin,cap_net_bind_service=+ep <clash_bin_path>` with the password piped to stdin

### Requirement: Password piped to sudo -S
When sudo is needed, the system SHALL pass the user's password to `sudo -S` via `std::process::Command.stdin()`.

#### Scenario: Valid password
- **WHEN** the correct sudo password is provided via the TUI input
- **THEN** `sudo -S` accepts the password, `setcap` runs successfully, and the output is returned

#### Scenario: Invalid password
- **WHEN** an incorrect sudo password is provided
- **THEN** `sudo -S` rejects the password, the function returns an error, and the user sees a "sudo: incorrect password" error message via `Confirm::err()`

### Requirement: setcap binary path resolution
The system SHALL resolve the `setcap` binary from the standard PATH (including `/usr/sbin`).

#### Scenario: setcap found in /usr/sbin
- **WHEN** `setcap` is at `/usr/sbin/setcap`
- **THEN** the command executes successfully

#### Scenario: setcap not found
- **WHEN** `setcap` is not available in PATH
- **THEN** the function returns an error indicating `setcap` could not be found

### Requirement: Report operation output
The system SHALL return the stdout and stderr of the setcap command for display to the user.

#### Scenario: Successful setcap with no output
- **WHEN** `setcap` succeeds and produces no output
- **THEN** the function returns an empty or minimal success string (e.g., "OK")

#### Scenario: Successful setcap with verbose output
- **WHEN** `setcap` succeeds and writes to stdout/stderr
- **THEN** the combined output is returned in the result

### Requirement: Use configured clash binary path
The system SHALL use `basic.clash_bin_path` from the config file as the target for setcap.

#### Scenario: Configured binary path
- **WHEN** `config.yaml` contains `basic.clash_bin_path: "/usr/bin/mihomo"`
- **THEN** setcap targets `/usr/bin/mihomo`

#### Scenario: Default binary path
- **WHEN** no `basic.clash_bin_path` is configured
- **THEN** setcap targets the default path `/usr/bin/mihomo`
