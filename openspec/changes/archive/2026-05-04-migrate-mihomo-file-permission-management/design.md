## Context

ClashTUI (the predecessor) has a well-tested permission management system for mihomo config files in `/srv/mihomo`. Both `clashtui` and `mihomo` daemon need read/write access, but may run as different users. ClashTUI solves this with Unix group permissions: `g+s` (setgid) on the directory so new files inherit the group, group ownership matching the daemon's group, and group-writable (`g+w`) on all files.

ClashTUI's approach uses `setfsuid`/`setfsgid` (Linux filesystem ID impersonation) to avoid root permanently, plus re-exec chaining (`run_as_root` → fix → `run_as_previous_user`). Demotui will simplify this: no fid impersonation, no re-exec chaining. Instead, detect → prompt → sudo-fix-continue.

Demotui already has:
- `src/functions/command/linux.rs` with `run_as_su_by_sudo()` for interactive sudo elevation with terminal hold/release
- `src/functions/command.rs` with `set_permission()` for setcap operations
- `src/tui/tab/srvctl.rs` with Stop/Restart/SetPermission operations
- Config fields: `basic.clash_config_dir`, `basic.clash_bin_path`, `service.is_user`

## Goals / Non-Goals

**Goals:**
- Add detection of incorrect file permissions in mihomo config directory (setgid, group ownership, group-writable)
- Add repair of detected permission issues via sudo, using the existing `run_as_su_by_sudo` pattern
- Set umask to `0o002` at startup so demotui-created files are group-readable/writable
- Wire permission check into TUI startup flow with user confirmation prompt
- Expose manual permission check/repair as an operation in the SrvCtl tab (alongside Stop/Restart/SetPermission)
- Keep code concise and aligned with demotui's existing patterns (no fid, no re-exec)

**Non-Goals:**
- Filesystem ID impersonation (`setfsuid`/`setfsgid`)
- Re-exec chaining (`run_as_root` / `run_as_previous_user`)
- `CLASHTUI_EP` environment variable pattern
- Adding new config fields (group name is auto-detected from the directory's current group)
- Permission repair during CLI-only operation (must be TUI)
- Cross-platform support (Linux-only; permissions matter for mihomo service sharing)

## Decisions

### Decision 1: No fid impersonation — use `run_as_su_by_sudo` with `tui::hold()`

**Chosen:** Use the existing `run_as_su_by_sudo` pattern from `linux.rs` which calls `tui::hold(true)` to temporarily leave raw mode, runs `sudo` interactively, then `tui::hold(false)` to re-enter raw mode.

**Alternative considered:** ClashTUI's `setfsuid`/`setfsgid` impersonation.
**Why not:** Adds complexity (mock/restore fileop, initgroups, re-exec chain). The user explicitly requested the simpler sudo approach. The TUI hold/release pattern already works well in demotui.

### Decision 2: Auto-detect group from directory, not config

**Chosen:** Read the directory's current group ownership via `Group::from_gid(metadata.gid())` and use it as the target group for repair. No new config field.

**Alternative considered:** Add `mihomo_group` config field.
**Why not:** The correct group is whatever the directory already has (set by admin when creating `/srv/mihomo`). Adding a config field creates inconsistency risk and setup burden. Auto-detection is simpler and self-healing.

### Decision 3: Permission check at startup, manual operation via SrvCtl tab

**Chosen:** Run permission detection during TUI init (between `tui::init()` and `App::serve()`), and expose repair as a manual operation in the ClashSrvCtl tab.

**Alternative considered:** Automatic repair without user confirmation.
**Why not:** `sudo` requires interactive password entry. Demotui's pattern is to prompt the user and hold the terminal properly. Silent sudo would hang or fail.

### Decision 4: Umask 0o002 set in main.rs, before config init

**Chosen:** `sys::stat::umask(Mode::from_bits_truncate(0o002))` at the start of `main()`, before config init.

**Alternative considered:** Set umask only before file write operations.
**Why not:** Setting globally is simpler and covers all file creation. The umask 0o002 strips world-writable-other, not world-readable, so it's safe for non-sensitive directories too.

## Risks / Trade-offs

- **[sudo may not be available]** → The repair function checks if sudo succeeds; on failure, returns an error message. Users on systems without sudo (containers, embedded) will see a clear error, not a crash.
- **[Group auto-detection fails if directory doesn't exist yet]** → Detection returns "all OK" for non-existent directories (nothing to check). The first profile update will create the directory with correct umask-based permissions.
- **[g+s alone doesn't guarantee group inheritance on all filesystems]** → Setgid on ext4/xfs/btrfs works for directories (new files inherit directory group). Other filesystems (FAT, ntfs) are not supported and this is a Linux-mihomo feature.
- **[Repair modifies ownership of ALL files under config_dir]** → Could be surprising if users have custom permissions. Mitigation: show what will be changed before sudo, and the scope is only the mihomo config directory (a known directory).
