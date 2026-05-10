# service-control Delta Specification

## MODIFIED Requirements

### Requirement: Start the clash service
The system SHALL start the clash service using the configured service controller (systemctl or rc-service), respecting the `is_user` config flag for `--user` mode, and dispatching to the correct service name based on the active core type.

#### Scenario: Start mihomo with systemd
- **WHEN** `core_type` is `"mihomo"`, `hack.service_controller` is `Systemd`, and `service.is_user` is `false`
- **THEN** executes `systemctl start <mihomo_service_name>` and returns stdout/stderr

#### Scenario: Start singbox with systemd
- **WHEN** `core_type` is `"singbox"`, `hack.service_controller` is `Systemd`, and `service.is_user` is `false`
- **THEN** executes `systemctl start <singbox_service_name>` and returns stdout/stderr

#### Scenario: Systemd with is_user=true
- **WHEN** `hack.service_controller` is `Systemd` and `service.is_user` is `true`
- **THEN** executes `systemctl --user start <service_name>` and returns stdout/stderr

#### Scenario: OpenRc with is_user=false
- **WHEN** `hack.service_controller` is `OpenRc` and `service.is_user` is `false`
- **THEN** executes `rc-service <service_name> start` and returns stdout/stderr

#### Scenario: OpenRc with is_user=true
- **WHEN** `hack.service_controller` is `OpenRc` and `service.is_user` is `true`
- **THEN** executes `rc-service <service_name> start --user` and returns stdout/stderr

### Requirement: Stop the clash service
The system SHALL stop the clash service using the configured service controller, respecting the `is_user` flag and dispatching to the correct service name based on the active core type.

#### Scenario: Stop mihomo service
- **WHEN** `core_type` is `"mihomo"`, service controller is `Systemd`, and `is_user` is `false`
- **THEN** executes `systemctl stop <mihomo_service_name>` and returns stdout/stderr

#### Scenario: Stop singbox service
- **WHEN** `core_type` is `"singbox"`, service controller is `Systemd`, and `is_user` is `false`
- **THEN** executes `systemctl stop <singbox_service_name>` and returns stdout/stderr

#### Scenario: Stop with is_user=true
- **WHEN** `is_user` is `true`
- **THEN** appends `--user` flag to the stop command

### Requirement: Restart the clash service
The system SHALL restart the clash service by stopping then starting it (sequential operations), using the service name for the active core type.

#### Scenario: Restart mihomo service
- **WHEN** `core_type` is `"mihomo"` and user requests restart
- **THEN** the system first stops the mihomo service, then starts it; output from both operations is collected and returned combined

#### Scenario: Restart singbox service
- **WHEN** `core_type` is `"singbox"` and user requests restart
- **THEN** the system first stops the sing-box service, then starts it; output from both operations is collected and returned combined

### Requirement: Service name from config
All service control operations SHALL use the service name from config, selecting the mihomo service name or sing-box service name based on the active core type.

#### Scenario: Mihomo service name from config
- **WHEN** `core_type` is `"mihomo"` and config contains `service.clash_service_name: "mihomo"`
- **THEN** start/stop/restart operations target the "mihomo" service

#### Scenario: Singbox service name from config
- **WHEN** `core_type` is `"singbox"` and config contains `service.singbox_service_name: "sing-box"`
- **THEN** start/stop/restart operations target the "sing-box" service

#### Scenario: Default service names
- **WHEN** no service name is configured
- **THEN** mihomo operations target "mihomo" and sing-box operations target "sing-box"

### Requirement: Error handling for failed commands
When a service control command exits with a non-zero status, the system SHALL return a descriptive error including the exit code and stderr output.

#### Scenario: Service not found
- **WHEN** user tries to start a service that does not exist
- **THEN** the function returns `Err` containing the systemctl/rc-service error message

#### Scenario: Permission denied without sudo
- **WHEN** user with `is_user=false` tries to start a service without privileges
- **THEN** systemctl returns a non-zero exit status and stderr is included in the error

## ADDED Requirements

### Requirement: Config validation per core type
The system SHALL validate the core's configuration using the appropriate CLI command before starting or reloading the service. For mihomo, SHALL use `mihomo -t -d <dir> -f <file>`. For sing-box, SHALL use `sing-box check -c <file>`.

#### Scenario: Validate mihomo config
- **WHEN** `core_type` is `"mihomo"`
- **THEN** the system runs `{mihomo_bin_path} -t -d {clash_config_dir} -f {clash_config_path}`

#### Scenario: Validate singbox config
- **WHEN** `core_type` is `"singbox"`
- **THEN** the system runs `{singbox_bin_path} check -c {singbox_config_path}`

#### Scenario: Validation failure
- **WHEN** config validation exits with non-zero
- **THEN** the system SHALL display the error output to the user
