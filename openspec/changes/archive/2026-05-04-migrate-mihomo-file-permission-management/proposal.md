## Why

When `clashtui` and the `mihomo` daemon run as different users, they need shared read/write access to the mihomo config directory (`/srv/mihomo`). Without proper group permissions, profile updates from clashtui fail because mihomo can't read the files, or clashtui can't write them. This change migrates clashtui's proven group-permission management (g+s, group ownership, group-writable detection + repair) into demotui, simplified by removing the fid/fsuid impersonation mechanism in favor of a straightforward sudo-repair-then-continue approach.

## What Changes

- Add file permission detection for the mihomo config directory: check setgid bit, group ownership consistency, and group-writable status on all files
- Add permission auto-repair via sudo when detection finds mismatched permissions
- Set umask to `0o002` at startup so demotui-created files remain group-readable/writable
- Simplify the approach compared to clashtui: no `mock_fileop_as_sudo_user` (fsuid/fsgid impersonation), no `run_as_previous_user` re-exec; instead sudo-fix then continue normally
- Wire the permission check into the TUI startup flow, prompting the user before elevating

## Capabilities

### New Capabilities

- `file-permission-detection`: Detect incorrect permissions on mihomo config directory files (missing setgid, wrong group ownership, missing group-writable)
- `file-permission-repair`: Repair detected file permission issues via sudo, applying setgid to the directory, correcting group ownership, and adding group-writable to all files

### Modified Capabilities

- `set-permission`: Extend the existing setcap capability to also cover file-level permission repair as a separate operation (not changing setcap behavior itself, but the SrvCtl tab will expose both binary-capability and file-permission operations)

## Impact

- `src/main.rs`: Add umask `0o002` setting at startup
- `src/functions/command/linux.rs`: New permission detection and repair functions
- `src/functions/command.rs`: Re-export new permission operations
- `src/tui/tab/srvctl.rs`: Expose file permission check/repair in SrvCtl tab alongside existing setcap
- `src/config/core.rs`: May need a `mihomo_group` config field (or derive it from the directory's existing group)
- `openspec/specs/set-permission/`: Delta spec for the added file-permission operations
