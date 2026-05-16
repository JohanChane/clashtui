# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0-alpha.2] - 2026-05-17

### Added
- macOS support (#64)
- Windows support documentation (#68)

### Changed
- CI: matrix strategy with platform tests (#65)
- CI: unified release file naming
- Updated demo recording and shields badges

### Fixed
- macOS CI builds (#65)
- macOS: services no longer auto-start at boot after install; `launchctl load -w` removed from install script and TUI (#69)
- Mihomo template generation

## [0.3.0-reborn] - 2024-10-06

### Added
- Dual core support: sing-box alongside Mihomo
- Template system for auto-generating config files with variable expansion
- Custom themes via `theme.yaml`
- Custom key bindings via `keymap.yaml`
- fzf integration for selection
- In-program download and update
- Connection management: view and close individual or all connections
- Profile preview in profile tab
- Clash info display (version, mode, etc.)
- CLI `profile list --name-only` argument
- Environment variable templates
- Systemd timer documentation

### Changed
- **Complete TUI rewrite** with async frontend/backend architecture
- Merged `clashcli` functionality into `clashtui`
- Reimplemented CLI subsystem
- Combined lib into single binary
- Switched serialization from `serde_yaml` to `serde_yml`
- Switched encoding from `encoding` to `encoding_rs`
- Profile list now maintains stable sort order
- Left/right navigation enabled for all list-based popups
- Database state persists to disk
- Uses `LazyLock` for home directory initialization
- Updated mihomo dependency version

### Fixed
- Terminate connection support in TUI (#39)
- Service message display (now multi-line)
- `SetPermission` not working
- Cannot get version from mihomo
- Cursor cannot move at profile tab
- TUN status display error
- Config reload issues
- Current profile cannot update
- Deadlock during profile refresh
- Tab bar foreground color
- Windows build
- Various CI and build issues

### Removed
- `clashcli` standalone CLI tool

## [0.2.3] - 2025-01-05

### Added
- "No proxy provider" download option
- Profile Info display (last modify time, extra info)
- Timeout configuration field
- `--config` CLI parameter
- User mode service support (systemctl `--user`)
- ARM64 Linux build in CI
- Profile update interval configuration
- Rolling log (auto-remove old logs when too large)
- Scrollbar for HelpPopUp
- Version display via InfoPopUp
- Warning for profiles not updated in over 24 hours
- Config reload after profile update via CLI

### Changed
- Rewrote profile management: all profiles now under `profiles/` directory
- Replaced `anyhow` with structured error handling
- Moved CLI argument handling to `clap`
- Migrated from `reqwest` to `minreq`
- Reduced dependency count
- Enabled release build optimization (`opt = "s"`)
- Separated config file: `data.yaml` for data, `config.yaml` as user-friendly config
- Bit flags replace `HashSet` for internal flags

### Fixed
- User mode service status check
- Real-time state refresh after service operations
- Always downloading files through proxy (now prompts)
- Profile deletion not working
- New config not saving
- YAML validation (must be mapping)
- `SwitchMode` bug
- Windows build
- Permission issues with clash config directory files

### Removed
- Geo data update (deprecated; mihomo handles it natively)
- `ClashSrvOp`, `TestClashConfig`, `UpdateGeoData` operations
- Config tab (deprecated)
- `dependabot` integration

## [0.2.1] - 2024-04-14

### Added
- User mode service support
- Config file tests
- Upgrade without rewriting config

### Fixed
- Profile deletion not working
- `is_yaml` validation (must be mapping)
- New config not saving
- SwitchMode bug

### Changed
- Auto include `/usr/sbin` to PATH for setcap
- Using local static values instead of `Rc`

## [0.2.0] - 2024-03-24

### Added
- Mode Control: switch between Direct, Global, Rule within app
- User themes support (preparation)
- Flag system (`Flags[FirstInit]`)
- Cron job support (untested)
- Clash version display
- Help info rewrite
- Config Tab
- Status bar displaying current mode
- CI pipeline and artifact uploads
- `.deb` package build

### Changed
- Upgraded ratatui from 0.23.0 to 0.25.0
- Replaced `anyhow` with structured errors in UI
- Reduced dependencies
- Using enums instead of strings for key bindings
- Config directory init moved to `main.rs`

### Fixed
- First init config error
- Config tab popup event bug
- TUN status display in status bar

## [0.1.0] - 2023-12-29

### Added
- Profile list auto-selects currently used profile on startup
- REST API integration for Clash backend
- Soft restart order
- Mocked client for testing

### Changed
- Renamed Clash.Meta references to Mihomo
- Standalone clash interface module

### Fixed
- Undefined behavior when API is not properly set
- Config error catching

## [0.0.3] - 2023-12-17

### Added
- Flexible configuration support
- Template system (initial)
- Scoop package manager support

### Changed
- Renamed Clash.Meta to Mihomo throughout

## [0.0.2] - 2023-11-22

### Added
- Template generation support
- Basic configuration structure

## [0.0.1] - 2023-11-18

### Added
- Initial release
- Basic TUI for Clash proxy management
- Profile switching
- Service start/stop control

[Unreleased]: https://github.com/JohanChane/clashtui/compare/v0.3.0-alpha.2...HEAD
[0.3.0-alpha.2]: https://github.com/JohanChane/clashtui/compare/v0.3.0-reborn...v0.3.0-alpha.2
[0.3.0-reborn]: https://github.com/JohanChane/clashtui/compare/v0.2.3...v0.3.0-reborn
[0.2.3]: https://github.com/JohanChane/clashtui/compare/v0.2.1...v0.2.3
[0.2.1]: https://github.com/JohanChane/clashtui/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/JohanChane/clashtui/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/JohanChane/clashtui/compare/v0.0.3...v0.1.0
[0.0.3]: https://github.com/JohanChane/clashtui/compare/v0.0.2...v0.0.3
[0.0.2]: https://github.com/JohanChane/clashtui/compare/v0.0.1...v0.0.2
[0.0.1]: https://github.com/JohanChane/clashtui/releases/tag/v0.0.1
