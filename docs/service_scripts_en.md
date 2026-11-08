# Scripts

CoreSrvCtl, clipboard, file editing and directory commands share one `CONFIG_ROOT/scripts.json` action format. CLI service commands, global restart, status queries use the same configuration.

```text
CONFIG_ROOT/
  config.yaml
  scripts.json
  scripts/coresrvctrl/
    systemd/service.sh
    openrc/service.sh
    launchd/service.sh
    windows/service.ps1
    windows/sysproxy.ps1
  scripts/clipboard/
    copy.sh                 # Unix
    copy.ps1                # Windows
  scripts/files/
    open.sh                 # Unix
    open.ps1                # Windows
```

The installer selects the backend (Linux: systemd/OpenRC, macOS: launchd, Windows: NSSM), installs only its scripts, and writes `scripts.json`. Unix `--is-user` selects `user`; otherwise the scope is `system`. On Unix, reinstallation backs up each replaced file as `.bak.N`; Windows backs up the configuration directory with a numbered suffix. The installer then replaces the selected default scripts and configuration. Other custom files remain in place.

Each Unix backend has one `default.json`. The installer replaces standalone `"${scope}"` arguments with `"system"` or `"user"`; other variables stay intact. Neither `${scope}` nor `${is_user}` is a runtime variable. Windows uses `windows/system.json` directly. There is no template index or runtime migration. `config.yaml` supplies service names and core paths; it contains no backend or scope selector.

For manual installation, also copy the platform's `contrib/scripts/clipboard/copy.sh` or `copy.ps1` to `CONFIG_ROOT/scripts/clipboard/`, and `contrib/scripts/files/open.sh` or `open.ps1` to `CONFIG_ROOT/scripts/files/`. Copy the chosen service backend's script(s) from `contrib/scripts/coresrvctrl/` into the matching directory above. Copy its JSON template to `CONFIG_ROOT/scripts.json`, replacing `"${scope}"` with the desired literal on Unix. The application only reads these files; missing or invalid configuration disables service actions and reports the path. Other tabs remain available.

## Configuration

```json
{
  "root": "scripts/coresrvctrl",
  "actions": [
    {
      "id": "start_core",
      "label": "Start core",
      "cmd": {
        "program": "/bin/sh",
        "args": ["${root}/systemd/service.sh", "start", "system", "${service_name}"]
      }
    },
    {
      "id": "status_core",
      "cmd": {
        "program": "/bin/sh",
        "args": ["${root}/systemd/service.sh", "status", "system", "${service_name}"],
        "interactive": false,
        "output": "silent",
        "timeout_secs": 5
      }
    }
  ]
}
```

`root`, `actions`, `id`, `cmd.program` and `cmd.args` are required. `label` is optional: omit it or use `null` to hide the action from the menu. A supplied label must be a nonblank string. Unknown fields and empty programs are rejected. IDs match `[a-z][a-z0-9_-]*`.

Every `cmd` also accepts these optional fields:

| Field | Default | Meaning |
| --- | --- | --- |
| `interactive` | `true` | Hand over the terminal for keyboard input; `false` captures output without switching screens |
| `output` | `"popup"` | Noninteractive output destination: `popup`, `silent`, or `terminal` |
| `keep_children_on_success` | `false` | Allow background children to survive only after a successful command |
| `timeout_secs` | `null` | Positive integer deadline in seconds; omission or `null` means no timeout, independently of `interactive` |

`output` is ignored for interactive execution. `popup` displays stdout/stderr as text. `silent` suppresses both streams; failed exits still report their status. `terminal` forwards successful stdout verbatim on the render thread and displays stderr separately; scripts construct any terminal control sequences themselves. Captured results remain available to internal callers in all modes.

Execution waits for the main command. An optional `timeout_secs` bounds the entire command, including stdin writes, in either interactive mode. Zero is rejected. Shipped copy and status actions set it to 5; file actions omit it so a GUI process may remain active. Automatic status queries retain a separate fixed five-second deadline. This field is independent of the network `timeout` in `config.yaml`. Capture retains at most 64 KiB per stream and marks truncation; terminal stdout allows 2 MiB and fails without forwarding if exceeded. Failure, timeout, and cancellation clean up managed children even when preservation was requested. On success, the caller collects the parent's remaining output and closes capture pipes without waiting for descendants to close them. Retained children must detach their standard streams and manage their own lifetime. These rules apply on Unix and Windows.

The reserved `copy`, `edit_file` and `open_dir` actions are excluded from CoreSrvCtl even with labels. See [clipboard copying](clipboard_en.md) for the stdin contract and [file actions](#file-actions) for target paths.

Visibility only affects the menu. Hidden actions still support background queries and shortcuts. Default templates hide `status_core`.

Optional `"cores": ["mihomo"]` or `["singbox"]` limits an action. Omission means all cores. Use the machine ID `singbox`, not the display name `sing-box`. Explicit scopes must be nonempty and contain no duplicates. Repeated IDs may have disjoint scopes only; a targeted action cannot override a shared action.

Filtering by core and label preserves array order. Custom IDs work without Rust changes. TUI loads and validates the file once at startup. Restart after changing it. Invalid startup configuration disables script actions; internal core checks and permission repair remain available. CLI restart/stop load a local configuration per invocation.

| Standard ID | Meaning |
| --- | --- |
| `start_core` | Start; already running succeeds, missing service fails |
| `stop_core` | Stop; already stopped or confirmed missing succeeds |
| `restart_core` | Restart; may start a stopped service, missing service fails |
| `status_core` | Query runtime state |
| `enable_autostart_core` | Enable persistent autostart, without starting now |
| `disable_autostart_core` | Disable persistent autostart, without stopping now |

Autostart actions require an installed service and are idempotent. Missing standard actions report unsupported. Windows defaults also provide `install_core`, `uninstall_core`, `enable_system_proxy` and `disable_system_proxy`.

## File actions

The file pane's Edit actions (Profile, Template and providers) invoke `edit_file`. Global directory shortcuts invoke `open_dir` with the existing selected directory. Actions use the startup configuration; missing or invalid actions report an error. Neither action automatically reloads or validates edited contents after returning.

The Unix default entries are:

```json
{
  "id": "edit_file",
  "cmd": {
    "program": "/bin/sh",
    "args": ["${config_root}/scripts/files/open.sh", "${path}"],
    "interactive": false,
    "output": "silent",
    "keep_children_on_success": true
  }
}
```

Add a second entry with `id: "open_dir"` and the same command. Windows defaults invoke `scripts/files/open.ps1` through PowerShell; see the [Windows template](../contrib/scripts/coresrvctrl/windows/system.json). The script uses [`Invoke-Item -LiteralPath`](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.management/invoke-item?view=powershell-5.1) to open the associated application. Unix uses `open` on macOS and `xdg-open` on other desktops. Without a Linux desktop session, it fails with a message rather than choosing a terminal tool. For diagnostics, change `output` to `popup`.

For an editor in the current terminal, replace the command with:

```json
{"program": "nvim", "args": ["${path}"], "interactive": true}
```

Use the same approach with `yazi` for `open_dir`. GUI tools or tools opening their own terminal window use `interactive: false`. No wrapper script is required for a fixed program. Do not let a noninteractive wrapper fall back to a tool requiring the current terminal.

Pass `${path}` as its own argument. It is expanded once, without shell evaluation; Windows paths use the existing normalization of extended prefixes. Inside a shell script use `"$1"`; do not embed the path into `sh -c` text. SSH executes these commands on the remote host, so use a terminal editor there when no desktop is available.

Successful launcher exit means the launch completed, not that editing was saved. A command using a GUI editor's wait option stays active until that editor returns. Running commands are cancelled on clashtui exit; successful retained children manage their own lifetime. `keep_children_on_success` does not disable a timeout or preserve an unfinished main command.

## Paths and variables

- Relative `root` is based on `CONFIG_ROOT`; absolute roots are accepted. The child working directory is always root.
- Bare `program` names such as `sudo` use PATH. `systemd/service.sh` and `./service.sh` are root-relative. Absolute programs are used directly.
- Windows accepts both separators, but rejects incomplete paths such as `C:foo` and `\foo`.
- No `~`, environment variable or shell expansion. Ordinary arguments are not automatically made root-relative.
- No implicit shell. Specify an interpreter, or use a Unix script with a shebang and execute permission.

The following variables expand once in program and each argument:

| Variable | Value |
| --- | --- |
| `${root}`, `${config_root}` | Absolute directories |
| `${core_type}` | `mihomo` or `singbox` |
| `${service_name}` | Target core's configured service name |
| `${core_bin}`, `${core_config_dir}`, `${core_config_path}` | Target core's absolute paths |
| `${proxy_port}` | Valid runtime mixed-port, otherwise port, from target `/configs` |
| `${path}` | Absolute target path, available only when a file-edit or directory shortcut invokes the action |

A bare `${core_bin}` program name is resolved through PATH to an absolute path; a relative path with separators is based on `CONFIG_ROOT`.

`$${name}` passes literal `${name}`. Spaces, Unicode, quotes and `$()` remain argument data. Unknown variables fail validation; missing values block only commands that reference them. Proxy port queries are lazy and use the target core's authentication. Secrets are never passed to scripts; disabling the proxy works while the core is offline.

## Results and terminal

Ordinary actions return exit code `0` for success and nonzero for failure. stdout is optional explanation; stderr is diagnostic. Shell `$0` is the script name: return with `exit N`, inspect with `$?`.

`status_core` must exit `0` and print exactly one UTF-8 line without BOM: `active`, `inactive` or `missing`. A final LF or CRLF is allowed. An installed but stopped or failed service is inactive. Missing means confirmed uninstalled. Diagnostics belong on stderr.

Permission errors, missing tools, nonzero/signal exits, invalid output, timeout and stdout overflow mean unknown. An absent status action means unsupported. Runtime state does not describe autostart.

Internal status queries always capture and consume the result without popups or terminal forwarding, independently of manual invocation settings. Background queries close stdin, have a total five-second deadline and capture at most 64 KiB per stream. stdout overflow fails; stderr may be truncated with a marker and remains available even on success. Both status pages use the same single-task scheduler; a user request cancels and reaps the current query before starting. Status scripts must be noninteractive; use `sudo -n` if needed, without password or UAC prompts. Scripts must not detach from their managed process group/Job. Real core services started by the service manager are outside script cleanup.

Interactive commands exclusively own the terminal after the input reader acknowledges suspension. They inherit standard streams except Copy, which always receives text on stdin. Press Enter once to return; Ctrl-C interrupts the command. CLI adds no Enter prompt.

Unix defaults invoke `/bin/sh`; scripts use sudo only for commands that require system privileges. User scope never elevates; root does not elevate again. Status queries use no sudo on systemd, OpenRC or launchd. Launchd reads the domain and plist as the current user; read failures remain query errors. Only service writes may authenticate. Custom scripts can call sudo directly without a special wrapper.

Sudo controls credential reuse. A valid tty timestamp can span scripts in the same terminal/session; ppid timestamps may require authentication for new script processes. Clashtui does not store passwords, keep credentials alive, clear caches on exit or modify sudoers. Command-specific authorization rules must match the commands actually invoked.

Command tracing is off by default. Use `/bin/sh -x service.sh restart system mihomo`, or `set -x` / `set +x` inside a script. When editing `scripts.json`, place `-x` before the script path in `/bin/sh` arguments. Tracing writes to stderr; status stdout still follows the protocol.

## Platform behavior

- systemd: enable/disable omit `--now`; user services do not enable linger automatically.
- OpenRC: user queries and writes preserve `--user`, requiring manager support and `XDG_RUNTIME_DIR`. Only the `default` runlevel is changed.
- launchd: lifecycle uses load/unload; persistent autostart uses enable/disable. User scope means login autostart. Manual start/restart uses `load -F` to run even when autostart is disabled, preserving the persistent enable/disable setting.
- Windows: NSSM manages lifecycle; SCM queries state. Autostart uses AUTO_START/DEMAND_START. Service writes request UAC when needed; cancellation fails. PowerShell files use UTF-8 BOM, protocol output does not. ExecutionPolicy Bypass cannot override group policy.
- Windows proxy: current-user HKCU settings plus a change broadcast; enabling and disabling are independent actions.

Cancellation, timeout and spawn failures have distinct error kinds and retain bounded partial diagnostics. Silent mode still hides captured output. Failed terminal actions never forward partial raw stdout. Cleanup covers the managed Unix process group or Windows Job; escaped descendants are outside that boundary, and UAC tracking requires the bundled script to join the Job.
