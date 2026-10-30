# Service scripts

CoreSrvCtl builds its menu from one `CONFIG_ROOT/scripts.json`. CLI service commands, global restart, status queries and core switching use the same configuration.

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
```

The binary embeds all default resources. Startup creates missing files offline and never overwrites existing files. Extraction rejects symbolic links and reports unwritable paths without elevation. Updated defaults may appear as `.new` samples beside modified resources; existing samples are preserved too.

Only when `scripts.json` is absent, legacy `service_controller` and `is_user` select data templates. Different core backends or scopes produce separate actions with `cores`. Unknown backends fail migration. `config.yaml` remains unchanged; its service names and core paths still supply variables.

## Configuration

```json
{
  "root": "scripts/coresrvctrl",
  "actions": [
    {
      "id": "start_core",
      "label": "Start core",
      "cmd": {
        "program": "sudo",
        "args": ["--", "/bin/sh", "${root}/systemd/service.sh", "start", "system", "${service_name}"]
      }
    },
    {
      "id": "status_core",
      "label": "Service status",
      "cmd": {
        "program": "/bin/sh",
        "args": ["${root}/systemd/service.sh", "status", "system", "${service_name}"]
      }
    }
  ]
}
```

All shown fields are required. Unknown fields, empty labels and empty programs are rejected. IDs match `[a-z][a-z0-9_-]*`.

Optional `"cores": ["mihomo"]` or `["singbox"]` limits an action. Omission means all cores. Use the machine ID `singbox`, not the display name `sing-box`. Explicit scopes must be nonempty and contain no duplicates. Repeated IDs may have disjoint scopes only; a targeted action cannot override a shared action.

Filtering preserves array order. Custom IDs work without Rust changes. Entering the tab reloads the whole file and preserves the selected ID when applicable. Invalid edits disable actions instead of running stale commands. In-flight operations keep their original snapshot.

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

Background queries close stdin, have a total five-second deadline and capture at most 64 KiB per stream. Status scripts must be noninteractive; use `sudo -n` if needed, without password or UAC prompts. Scripts must not detach from their managed process group/Job. Real core services started by the service manager are outside script cleanup.

Foreground commands exclusively own the terminal after the input reader acknowledges suspension. They inherit all standard streams. Press Enter once to return; Ctrl-C interrupts the command. CLI adds no Enter prompt.

Configure sudo directly as program, rather than nesting it inside scripts. The same clashtui parent and controlling terminal preserve the system tty/ppid timestamp scope. There is no password storage, keepalive or cache invalidation on exit. Policy expiry can prompt again. `sudo /bin/sh SCRIPT` may differ from an existing exact NOPASSWD rule; configure the previously authorized command directly if needed. Clashtui does not modify sudoers.

## Platform behavior

- systemd: enable/disable omit `--now`; user services do not enable linger automatically.
- OpenRC: user queries and writes preserve `--user`, requiring manager support and `XDG_RUNTIME_DIR`. Only the `default` runlevel is changed.
- launchd: lifecycle uses load/unload; persistent autostart uses enable/disable. User scope means login autostart. Disable also blocks later loading; manual start does not silently re-enable it.
- Windows: NSSM manages lifecycle; SCM queries state. Autostart uses AUTO_START/DEMAND_START. Service writes request UAC when needed; cancellation fails. PowerShell files use UTF-8 BOM, protocol output does not. ExecutionPolicy Bypass cannot override group policy.
- Windows proxy: current-user HKCU settings plus a change broadcast; enabling and disabling are independent actions.

Stop All preflights both stop actions, attempts both and aggregates errors. Switch Core preflights both stops and target start, starts only after all stops succeed, and saves the selection only after target start succeeds. It then exits as before. Failures preserve the previous selection and report completed steps; no automatic rollback or retry.
