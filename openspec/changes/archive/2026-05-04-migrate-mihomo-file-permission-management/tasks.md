## 1. Dependencies and Setup

- [x] 1.1 Add `nix` crate dependency to `Cargo.toml` (for `unistd::Group`, `unistd::Gid` — needed for GID-to-group-name resolution)
- [x] 1.2 Set umask to `0o002` in `src/main.rs` early in `main()` using `sys::stat::umask(Mode::from_bits_truncate(0o002))`

## 2. Permission Detection Functions

- [x] 2.1 Add `find_files_not_group_writable(dir: &Path) -> Vec<PathBuf>` to `src/functions/command/linux.rs` — recursively find files/dirs missing `0o0020` mode bit
- [x] 2.2 Add `find_files_not_in_group(dir: &Path, group_name: &str) -> Vec<PathBuf>` to `src/functions/command/linux.rs` — recursively find files/dirs not in target group, using `nix::unistd::Group::from_gid`
- [x] 2.3 Add `get_dir_group_name(dir: &Path) -> Option<String>` to `src/functions/command/linux.rs` — return the group name of a directory
- [x] 2.4 Add `check_file_permissions(dir: &Path) -> bool` to `src/functions/command/linux.rs` — combine setgid, group ownership, and group-writable checks; return `true` if all correct

## 3. Permission Repair Functions

- [x] 3.1 Add `repair_file_permissions(dir: &Path, group_name: &str)` to `src/functions/command/linux.rs` — run `chmod g+s` on dir, `chown :group` on wrong-group files, `chmod g+w` on non-group-writable files, using `tui::hold()` + `sudo` (the existing `run_as_su_by_sudo` pattern)

## 4. Wiring into the SrvCtl Tab

- [x] 4.1 Add `FixFilePermissions` variant to `SrvCtlOp` enum in `src/tui/tab/srvctl.rs`
- [x] 4.2 Wire the new variant into `SrvCtlOp::all()`, `as_str()` display, and `SrvCtlKey::Execute` handler to call `repair_file_permissions` or `check_file_permissions` as appropriate
- [x] 4.3 When permissions are already correct, show "Permissions OK, no repair needed" instead of requesting sudo password

## 5. Startup Permission Check

- [x] 5.1 In `src/tui.rs` or `src/tui/app.rs`, after `App::new()` and before entering the event loop, call `check_file_permissions` on the config directory
- [x] 5.2 If permissions are incorrect, show a Confirm popup "File permissions in <dir> need repair. Fix now?" with Yes/No; on Yes, prompt sudo password and run repair
- [x] 5.3 Ensure the startup check does not block TUI rendering (use a `task_set` spawn or pre-loop check)

## 6. Build and Verify

- [x] 6.1 Run `cargo check` to verify compilation
- [x] 6.2 Run `cargo test` to verify no regressions (note: no test suite for file permissions exists yet; this is a sanity check)
- [x] 6.3 Manually verify the SrvCtl tab shows the new "Fix File Permissions" option alongside existing operations
