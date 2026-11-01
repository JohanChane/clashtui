# Service scripts

CoreSrvCtl and clipboard commands share one `CONFIG_ROOT/scripts.json` action format. CLI service commands, global restart, status queries and core switching use the same configuration.

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
```

The installer selects the backend (Linux: systemd/OpenRC, macOS: launchd, Windows: NSSM), installs only its scripts, and writes `scripts.json`. Unix `--is-user` selects `user`; otherwise the scope is `system`. Reinstallation backs up the config directory with a numbered suffix, then replaces the selected default scripts and configuration. Other custom files remain in place.

Each Unix backend has one `default.json`. The installer replaces standalone `"${scope}"` arguments with `"system"` or `"user"`; other variables stay intact. Neither `${scope}` nor `${is_user}` is a runtime variable. Windows uses `windows/system.json` directly. There is no template index or runtime migration. `config.yaml` supplies service names and core paths; it contains no backend or scope selector.

For manual installation, also copy the platform's `contrib/scripts/clipboard/copy.sh` or `copy.ps1` to `CONFIG_ROOT/scripts/clipboard/`. Copy the chosen service backend's script(s) from `contrib/scripts/coresrvctrl/` into the matching directory above. Copy its JSON template to `CONFIG_ROOT/scripts.json`, replacing `"${scope}"` with the desired literal on Unix. The application only reads these files; missing or invalid configuration disables service actions and reports the path. Other tabs remain available.

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
        "output": "silent"
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

`output` is ignored for interactive execution. `popup` displays stdout/stderr as text. `silent` suppresses both streams; failed exits still report their status. `terminal` forwards successful stdout verbatim on the render thread and displays stderr separately; scripts construct any terminal control sequences themselves. Captured results remain available to internal callers in all modes.

Noninteractive execution waits for the command and has a five-second total deadline, including stdin writes. Interactive commands have no such timeout. Capture retains at most 64 KiB per stream and marks truncation; terminal stdout allows 2 MiB and fails without forwarding if exceeded. Failure, timeout, and cancellation clean up managed children even when preservation was requested. On success, the caller collects the parent's remaining output and closes capture pipes without waiting for descendants to close them. Retained children must detach their standard streams and manage their own lifetime. These rules apply on Unix and Windows.

The reserved `copy` action implements [clipboard copying](clipboard_en.md); it is never listed in CoreSrvCtl and does not by itself create service composite entries. Its stdin contract is documented separately.

Visibility does not affect execution or capabilities. Hidden actions still support background queries, shortcuts and composite operations. Default templates hide `status_core`; other actions can be hidden the same way. Built-in Switch Core and Stop All entries still use all configured actions to determine availability.

Optional `"cores": ["mihomo"]` or `["singbox"]` limits an action. Omission means all cores. Use the machine ID `singbox`, not the display name `sing-box`. Explicit scopes must be nonempty and contain no duplicates. Repeated IDs may have disjoint scopes only; a targeted action cannot override a shared action.

Filtering by core and label preserves array order. Custom IDs work without Rust changes. Entering the tab reloads the whole file and preserves the selected ID when applicable. Invalid edits disable actions instead of running stale commands. In-flight operations keep their original snapshot.

| Standard ID | Meaning |
| --- | --- |
| `start_core` | Start; already running succeeds, missing service fails |
| `stop_core` | Stop; already stopped or confirmed missing succeeds |
| `restart_core` | Restart; may start a stopped service, missing service fails |
| `status_core` | Query runtime state |
| `enable_autostart_core` | Enable persistent autostart, without starting now |
| `disable_autostart_core` | Disable persistent autostart, without stopping now |

Autostart actions require an installed service and are idempotent. Missing standard actions report unsupported. Windows defaults also provide `install_core`, `uninstall_core`, `enable_system_proxy` and `disable_system_proxy`.

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

A bare `${core_bin}` program name is resolved through PATH to an absolute path; a relative path with separators is based on `CONFIG_ROOT`.

`$${name}` passes literal `${name}`. Spaces, Unicode, quotes and `$()` remain argument data. Unknown variables fail validation; missing values block only commands that reference them. Proxy port queries are lazy and use the target core's authentication. Secrets are never passed to scripts; disabling the proxy works while the core is offline.

## Results and terminal

Ordinary actions return exit code `0` for success and nonzero for failure. stdout is optional explanation; stderr is diagnostic. Shell `$0` is the script name: return with `exit N`, inspect with `$?`.

`status_core` must exit `0` and print exactly one UTF-8 line without BOM: `active`, `inactive` or `missing`. A final LF or CRLF is allowed. An installed but stopped or failed service is inactive. Missing means confirmed uninstalled. Diagnostics belong on stderr.

Permission errors, missing tools, nonzero/signal exits, invalid output, timeout and excessive output mean unknown. An absent status action means unsupported. Runtime state does not describe autostart.

Internal status queries always capture and consume the result without popups or terminal forwarding, independently of manual invocation settings. Background queries close stdin, have a total five-second deadline and capture at most 64 KiB per stream. Status scripts must be noninteractive; use `sudo -n` if needed, without password or UAC prompts. Scripts must not detach from their managed process group/Job. Real core services started by the service manager are outside script cleanup.

Interactive commands exclusively own the terminal after the input reader acknowledges suspension. They inherit all standard streams. Press Enter once to return; Ctrl-C interrupts the command. CLI adds no Enter prompt.

Unix defaults invoke `/bin/sh`; scripts use sudo only for commands that require system privileges. User scope never elevates; root does not elevate again. Status queries use no sudo on systemd, OpenRC or launchd. Launchd reads the domain and plist as the current user; read failures remain query errors. Only service writes may authenticate. Custom scripts can call sudo directly without a special wrapper.

Sudo controls credential reuse. A valid tty timestamp can span scripts in the same terminal/session; ppid timestamps may require authentication for new script processes. Clashtui does not store passwords, keep credentials alive, clear caches on exit or modify sudoers. Command-specific authorization rules must match the commands actually invoked.

Command tracing is off by default. Use `/bin/sh -x service.sh restart system mihomo`, or `set -x` / `set +x` inside a script. When editing `scripts.json`, place `-x` before the script path in `/bin/sh` arguments. Tracing writes to stderr; status stdout still follows the protocol.

## Platform behavior

- systemd: enable/disable omit `--now`; user services do not enable linger automatically.
- OpenRC: user queries and writes preserve `--user`, requiring manager support and `XDG_RUNTIME_DIR`. Only the `default` runlevel is changed.
- launchd: lifecycle uses load/unload; persistent autostart uses enable/disable. User scope means login autostart. Manual start/restart uses `load -F` to run even when autostart is disabled, preserving the persistent enable/disable setting.
- Windows: NSSM manages lifecycle; SCM queries state. Autostart uses AUTO_START/DEMAND_START. Service writes request UAC when needed; cancellation fails. PowerShell files use UTF-8 BOM, protocol output does not. ExecutionPolicy Bypass cannot override group policy.
- Windows proxy: current-user HKCU settings plus a change broadcast; enabling and disabling are independent actions.

Stop All preflights both stop actions, attempts both and aggregates errors. Switch Core preflights both stops and target start, starts only after all stops succeed, and saves the selection only after target start succeeds. It then exits after queued terminal output is delivered and any output popups are dismissed. Failures preserve the previous selection and report completed steps; no automatic rollback or retry.
