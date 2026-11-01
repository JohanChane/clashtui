# Copy to clipboard

File pages offer two copy operations: press `y` then `s` for a subscription URL, or `y` then `p` for a file path.

| Page | Keys | Action | Content | `CLASHTUI_COPY_SOURCE` | `CLASHTUI_COPY_FORMAT` |
| --- | --- | --- | --- | --- | --- |
| File / Profile | `y s` | `CopySubscription` | Saved subscription URL, including its query parameters | `profile` | `text` |
| File / Profile | `y p` | `CopyPath` | Full local profile path | `profile` | `text` |
| File / Template | `y p` | `CopyPath` | Full template path | `template` | `text` |

Copy uses the highlighted item after filtering. Missing selections or profiles without a subscription URL report an error without executing the copy script. Path copying does not read file contents; Windows paths omit extended prefixes and use forward slashes. Connections, Logs and Proxies have no copy action.

Pressing `y` alone shows available continuations; Esc cancels. Rebind the actions in `keymap.yaml`, for example replacing Profile's path-copy chord with Ctrl-y:

```yaml
file:
  profile:
    - {on: [y, p], action: null}
    - {on: "<C-y>", action: CopyPath}
```

Remove old custom `action: Copy` bindings or replace them with the actions above. A single-key `y` binding cannot coexist with chords starting with `y` in the same file pane.

## Configuration

Clipboard uses the `copy` action in `CONFIG_ROOT/scripts.json`, alongside service actions. The installer includes this action and installs `scripts/clipboard/copy.sh` (Unix) or `copy.ps1` (Windows). Existing installations must install the script and add the action. The application does not create or overwrite scripts automatically.

Add this object to the existing `actions` array; keep the other actions and `root`:

```json
{
  "id": "copy",
  "cmd": {
    "program": "/bin/sh",
    "args": ["${config_root}/scripts/clipboard/copy.sh", "auto"],
    "interactive": false,
    "output": "terminal",
    "keep_children_on_success": true,
    "timeout_secs": 5
  }
}
```

Windows uses this `cmd` instead:

```json
{
  "program": "powershell.exe",
  "args": ["-NoProfile", "-NonInteractive", "-STA", "-ExecutionPolicy", "Bypass", "-File", "${config_root}/scripts/clipboard/copy.ps1", "auto"],
  "interactive": false,
  "output": "terminal",
  "keep_children_on_success": true,
  "timeout_secs": 5
}
```

Execution options are shared with every script action. The shipped copy action also sets `timeout_secs: 5`. Their defaults are `interactive: true`, `output: "popup"`, and `keep_children_on_success: false`. The clipboard defaults explicitly override them. See [script configuration](service_scripts_en.md).

`copy` is reserved for the file copy actions and is excluded from CoreSrvCtl even if it has a label. Optional `cores` scopes work as for other actions. Each copy reloads `scripts.json`, so changes need no restart. Invalid configuration fails rather than executing an old command.

**Migration:** remove `extra.copy_cmd` from `config.yaml` (including a `null` entry); it now reports a migration error. Move custom program/args into the `copy` action. To replace the old `osc52` value, change the script argument from `auto` to `osc52`. Relative paths and `${...}` now follow the shared script rules: cwd is `${root}`, so use `${config_root}` for config-relative files. Write `$${name}` for a literal `${name}`.

## Script contract

- **stdin:** complete UTF-8 text followed by EOF. Newlines are preserved; text is never interpolated into argv.
- **argv:** configured arguments only, with the shared one-pass variable expansion.
- **Environment:** `CLASHTUI_COPY_SOURCE` and `CLASHTUI_COPY_FORMAT` from the table above; other variables are inherited.
- **Exit:** zero means success. Spawn/input errors, nonzero exit, timeout and cancellation report failure.
- **stdout:** native copy leaves it empty; terminal copying writes the complete control sequence. With `output: "terminal"`, clashtui forwards successful output verbatim on the render thread. It neither constructs nor recognizes OSC52.
- **stderr:** diagnostics. Terminal mode displays these separately instead of forwarding them as control sequences. `popup` displays both streams as text; `silent` suppresses both streams (a failed exit still reports its status).
- **Limits:** text at most 1 MiB; the shipped `timeout_secs: 5` limits the whole command, including stdin writes (omit or set to `null` for no timeout). Terminal stdout is limited to 2 MiB, enough for base64 expansion; overflow fails without forwarding a partial sequence. Other captured streams are limited to 64 KiB with truncation indicated.

The UI remains responsive during noninteractive execution. Repeated copy requests are ignored until the previous result is delivered. A nonempty terminal output produces `Copy output sent to terminal`; this does not acknowledge acceptance by the terminal.

With `keep_children_on_success: true`, the executor releases successful commands' background children on both Unix and Windows. `wl-copy`/`xclip` can continue serving the clipboard. Failure, timeout and cancellation still clean up the managed process group/Job. Background children must stop using captured stdin/stdout/stderr: the caller closes its pipes after collecting the parent's output and does not wait for descendants' EOF. Retained children own their subsequent lifetime, including after clashtui exits.

## Bundled backends

```text
copy.sh  [auto|osc52|pbcopy|wl-copy|xclip|xsel]
copy.ps1 [auto|native|osc52]
```

Omitted arguments select `auto`. Backend selection lives in the scripts:

- Nonempty `SSH_TTY`, `SSH_CONNECTION` or `SSH_CLIENT`: OSC52.
- Local macOS: `pbcopy`; local Windows: `Set-Clipboard`.
- Linux: Wayland `wl-copy`, then X11 `xclip`/`xsel` when the relevant session and tool exist; otherwise OSC52.

Explicit backend arguments bypass auto selection. Failure of a selected tool is reported without falling back to another backend. Custom scripts can inspect the metadata and pipe stdin directly into a tool. Avoid `text=$(cat)`, which removes trailing newlines.

OSC52 requires terminal clipboard-write support and permission; multiplexers such as tmux must also allow forwarding. Over SSH it targets the local terminal clipboard. Terminal/multiplexer limits may be smaller than the application's input limit.
