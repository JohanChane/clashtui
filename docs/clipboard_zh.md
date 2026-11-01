# 复制到剪贴板

文件页仅提供两种复制：依次按 `y s` 复制订阅 URL，依次按 `y p` 复制文件路径。

| 页面 | 按键 | 动作 | 复制内容 | `CLASHTUI_COPY_SOURCE` | `CLASHTUI_COPY_FORMAT` |
| --- | --- | --- | --- | --- | --- |
| File / Profile | `y s` | `CopySubscription` | 当前配置保存的订阅 URL，保留完整查询参数 | `profile` | `text` |
| File / Profile | `y p` | `CopyPath` | 本地配置文件的完整路径 | `profile` | `text` |
| File / Template | `y p` | `CopyPath` | 模板文件的完整路径 | `template` | `text` |

复制使用过滤后的当前高亮条目。没有选择，或配置没有订阅 URL 时提示错误，不执行复制脚本。复制路径不读取文件内容；Windows 路径去除扩展前缀并使用正斜杠。连接、日志、代理页不提供复制动作。

单独按 `y` 只显示后续按键提示，按 Esc 取消。可在 `keymap.yaml` 中重绑，例如将 Profile 的路径复制改成 Ctrl-y：

```yaml
file:
  profile:
    - {on: [y, p], action: null}
    - {on: "<C-y>", action: CopyPath}
```

旧的自定义 `action: Copy` 绑定需删除或改为上述动作；文件页不能同时绑定单键 `y` 和以 `y` 开头的组合键。

## 配置

剪贴板使用 `CONFIG_ROOT/scripts.json` 中的 `copy` action，与服务命令共用结构。安装器会添加该动作，并安装 `scripts/clipboard/copy.sh`（Unix）或 `copy.ps1`（Windows）。已有安装需要复制脚本并添加动作；应用不会自动生成或覆盖脚本。

将下面对象加入现有 `actions` 数组，保留原有动作和 `root`：

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

Windows 使用以下 `cmd`：

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

执行字段适用于所有脚本动作；默认 copy 动作另设 `timeout_secs: 5`。其余默认值是 `interactive: true`、`output: "popup"`、`keep_children_on_success: false`。剪贴板配置明确覆盖它们。完整规则见[脚本配置](service_scripts_zh.md)。

`copy` 保留给文件页的复制动作，即使有 label 也不会出现在 CoreSrvCtl 菜单。可用 `cores` 限定核心。每次复制重新读取 `scripts.json`，修改无需重启；配置无效时报告错误，不执行旧命令。

**迁移：** 从 `config.yaml` 删除 `extra.copy_cmd`（包括 `null`）；旧字段现在会报告迁移错误。将自定义 program/args 移入 `copy` action。原来的 `osc52` 改为脚本参数 `osc52`。相对路径和 `${...}` 使用统一规则：工作目录为 `${root}`，配置目录下的文件应写 `${config_root}`；字面 `${name}` 写成 `$${name}`。

## 脚本协议

- **stdin**：完整 UTF-8 文本，随后 EOF。保留换行，不把文本拼入命令行。
- **argv**：仅配置的参数，使用统一的一次变量展开。
- **环境变量**：上表中的 `CLASHTUI_COPY_SOURCE`、`CLASHTUI_COPY_FORMAT`；其余环境继承自 clashtui。
- **退出码**：`0` 成功；启动、输入、非零退出、超时、取消均报告失败。
- **stdout**：本地复制保持为空；终端复制输出完整控制序列。`output: "terminal"` 将成功命令的 stdout 原样交主线程写入终端，clashtui 不构造或识别 OSC52。
- **stderr**：诊断信息。terminal 模式单独显示，不混入控制序列。popup 将两路输出作为文字弹窗；silent 隐藏两路输出，非零退出仍报告退出状态。
- **限制**：文本最多 1 MiB；默认 `timeout_secs: 5` 限制整个命令，包括 stdin 写入；省略或设为 `null` 表示不限时。terminal stdout 最多 2 MiB，容纳 base64 膨胀，超限时失败且不发送截断序列；其他捕获流各最多 64 KiB，超限标注截断。

非交互执行时界面保持响应；前一次结果送达前忽略重复复制。非空终端输出提示 `Copy output sent to terminal`，并不表示终端已经接受复制。

`keep_children_on_success: true` 在 Unix 和 Windows 都允许成功命令的后台子进程存活，供 `wl-copy`/`xclip` 等继续提供剪贴板数据。失败、超时、取消仍清理受管理进程组/Job。后台进程应停止使用捕获的 stdin/stdout/stderr：调用方收集主命令输出后关闭管道，不等待后台进程的 EOF。保留进程自行管理后续生命周期，clashtui 退出也不会再清理它们。

## 附带后端

```text
copy.sh  [auto|osc52|pbcopy|wl-copy|xclip|xsel]
copy.ps1 [auto|native|osc52]
```

省略参数等同 `auto`，选择逻辑位于脚本中：

- 存在非空 `SSH_TTY`、`SSH_CONNECTION` 或 `SSH_CLIENT`：OSC52。
- 本地 macOS：`pbcopy`；本地 Windows：`Set-Clipboard`。
- Linux：有会话和工具时依次选 Wayland 的 `wl-copy`、X11 的 `xclip`/`xsel`，否则 OSC52。

显式后端参数跳过自动选择。已选工具失败时报告错误，不改用其他后端。自定义脚本可读取元数据，并将 stdin 直接传给剪贴板工具；避免 `text=$(cat)`，它会丢失末尾换行。

OSC52 需要终端支持并允许剪贴板写入；tmux 等中间层也需要允许转发。SSH 下目标是本地终端的剪贴板。终端或复用器可能有更小的长度限制。
