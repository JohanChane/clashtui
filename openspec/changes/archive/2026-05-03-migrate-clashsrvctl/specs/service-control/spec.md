## ADDED Requirements

### Requirement: Start the clash service
The system SHALL start the clash service using the configured service controller (systemctl or rc-service), respecting the `is_user` config flag for `--user` mode.

#### Scenario: Systemd with is_user=false
- **WHEN** `hack.service_controller` is `Systemd` and `service.is_user` is `false`
- **THEN** executes `systemctl start <clash_service_name>` and returns stdout/stderr

#### Scenario: Systemd with is_user=true
- **WHEN** `hack.service_controller` is `Systemd` and `service.is_user` is `true`
- **THEN** executes `systemctl --user start <clash_service_name>` and returns stdout/stderr

#### Scenario: OpenRc with is_user=false
- **WHEN** `hack.service_controller` is `OpenRc` and `service.is_user` is `false`
- **THEN** executes `rc-service <clash_service_name> start` and returns stdout/stderr

#### Scenario: OpenRc with is_user=true
- **WHEN** `hack.service_controller` is `OpenRc` and `service.is_user` is `true`
- **THEN** executes `rc-service <clash_service_name> start --user` and returns stdout/stderr

### Requirement: Stop the clash service
The system SHALL stop the clash service using the configured service controller, respecting the `is_user` flag.

#### Scenario: Stop running service
- **WHEN** service controller is `Systemd` and `is_user` is `false`
- **THEN** executes `systemctl stop <clash_service_name>` and returns stdout/stderr

#### Scenario: Stop with is_user=true
- **WHEN** `is_user` is `true`
- **THEN** appends `--user` flag to the stop command

### Requirement: Restart the clash service
The system SHALL restart the clash service by stopping then starting it (sequential operations).

#### Scenario: Restart service
- **WHEN** user requests restart
- **THEN** the system first stops the service, then starts it; output from both operations is collected and returned combined

### Requirement: Service name from config
All service control operations SHALL use the `clash_service_name` field from the `service` section of the config file.

#### Scenario: Configured service name
- **WHEN** `config.yaml` contains `service.clash_service_name: "mihomo"`
- **THEN** start/stop/restart operations target the "mihomo" service

#### Scenario: Default service name
- **WHEN** no `service.clash_service_name` is configured
- **THEN** operations target the default service name "mihomo"

### Requirement: Error handling for failed commands
When a service control command exits with a non-zero status, the system SHALL return a descriptive error including the exit code and stderr output.

#### Scenario: Service not found
- **WHEN** user tries to start a service that does not exist
- **THEN** the function returns `Err` containing the systemctl/rc-service error message

#### Scenario: Permission denied without sudo
- **WHEN** user with `is_user=false` tries to start a service without privileges
- **THEN** systemctl returns a non-zero exit status and stderr is included in the error
