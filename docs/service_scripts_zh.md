# 服务控制脚本

CoreSrvCtl 与剪贴板共用 `CONFIG_ROOT/scripts.json` 的 action 结构，CoreSrvCtl 从中生成菜单。CLI 服务命令、全局重启、状态页和切核使用同一配置。

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

安装器根据平台和选项选择后端（Linux：systemd/OpenRC；macOS：launchd；Windows：NSSM），只安装对应脚本并生成 `scripts.json`。Unix 的 `--is-user` 选择 `user`，否则为 `system`。重装先按编号后缀备份配置目录，再替换所选默认脚本和配置；其他自定义文件保留。

每个 Unix 后端只有一个 `default.json`。安装器将独立的 `"${scope}"` 参数替换为 `"system"` 或 `"user"`，保留其余变量。`${scope}`、`${is_user}` 均不是运行期变量。Windows 直接使用 `windows/system.json`。不设模板索引，不做运行期迁移。`config.yaml` 只提供服务名和 core 路径，不再保存后端或 scope。

手动安装时，将 `contrib/scripts/coresrvctrl/` 中所选后端的脚本复制到上图对应目录，再将其 JSON 模板复制为 `CONFIG_ROOT/scripts.json`；Unix 需把 `"${scope}"` 改为所需字面值。应用只读取文件；配置缺失或无效时禁用服务动作并显示路径，其他 tab 仍可用。

## 配置

```json
{
  "root": "scripts/coresrvctrl",
  "actions": [
    {
      "id": "start_core",
      "label": "启动核心",
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

`root`、`actions`、`id`、`cmd.program`、`cmd.args` 必填。`label` 可选：省略或设为 `null` 时，动作不显示在菜单中；填写时必须是非空白字符串。拒绝未知字段和空程序。`id` 格式为 `[a-z][a-z0-9_-]*`。

隐藏只影响显示，不影响执行和能力判断。隐藏动作仍可供后台查询、快捷键和组合操作调用。默认模板隐藏 `status_core`，其他动作也可用相同方式隐藏。内置 Switch Core、Stop All 仍根据全部已配置动作判断可用性。

所有 `cmd` 均支持以下可选字段：

| 字段 | 默认值 | 含义 |
| --- | --- | --- |
| `interactive` | `true` | 交接终端并允许键盘输入；`false` 时保持 TUI，捕获输出 |
| `output` | `"popup"` | 非交互输出去向：`popup`、`silent`、`terminal` |
| `keep_children_on_success` | `false` | 仅在命令成功后允许后台子进程继续存活 |

交互执行时忽略 `output`。popup 将 stdout/stderr 作为文字弹窗；silent 隐藏两路输出，失败仍报告退出状态；terminal 将成功命令的 stdout 原样交主线程写入终端，stderr 单独显示。控制序列由脚本构造，clashtui 不识别 OSC52。所有模式的捕获结果仍可供内部调用方读取。

非交互执行等待主命令完成，总超时 5 秒，包含 stdin 写入；交互命令没有此超时。两路捕获各限 64 KiB，超限标注截断；terminal 的 stdout 上限为 2 MiB，超限失败且不发送部分序列。失败、超时、取消始终清理受管理子进程。成功后收集主命令的剩余输出并关闭捕获管道，不等待后台子进程关闭管道；保留的后台进程应脱离标准流并自行管理生命周期。Unix 和 Windows 使用相同规则。

保留的 `copy` action 用于[剪贴板复制](clipboard_zh.md)，不出现在 CoreSrvCtl，也不会单独生成服务组合项；其 stdin 协议另行定义。手动安装还需将 `contrib/scripts/clipboard/copy.sh` 或 `copy.ps1` 复制到 `CONFIG_ROOT/scripts/clipboard/`。

可选 `"cores": ["mihomo"]` 或 `["singbox"]`；省略表示全部 core。使用机器名 `singbox`，不是显示名 `sing-box`。数组不可为空或重复。同 ID 可分核定义不同命令，但作用域不可重叠；共用动作不能再用同 ID 定向覆盖。

按当前 core 和标签筛选菜单，保持数组顺序。自定义 ID 可直接执行，提供 `label` 即可显示。重新进入 tab 读取文件，保留仍存在的选中 ID。文件无效则禁用动作，不运行上次命令；正在执行的操作使用原快照。

| 标准 ID | 语义 |
| --- | --- |
| `start_core` | 启动；已运行成功，未安装失败 |
| `stop_core` | 停止；已停止或已确认未安装成功 |
| `restart_core` | 重启；已停止可启动，未安装失败 |
| `status_core` | 查询运行状态 |
| `enable_autostart_core` | 开启持久自启；不立即启动 |
| `disable_autostart_core` | 关闭持久自启；不立即停止 |

自启动作要求已安装，重复设置成功。缺少标准 ID 时对应功能报告不支持。Windows 默认另有 `install_core`、`uninstall_core`、`enable_system_proxy`、`disable_system_proxy`。

## 路径与参数

- 相对 `root` 基于 `CONFIG_ROOT`；也接受绝对目录。子进程 cwd 固定为 root。
- `program: "sudo"` 走 PATH；`"systemd/service.sh"`、`"./service.sh"` 基于 root；绝对路径直接使用。
- Windows 支持两种路径分隔符，拒绝 `C:foo`、`\foo` 等不完整路径。
- 不展开 `~`、环境变量或 shell 表达式；普通 args 不自动拼 root。
- 不隐式调用 shell。脚本需显式解释器，或 Unix shebang 与执行权限。

仅在 program 和每个 arg 中替换：

| 变量 | 值 |
| --- | --- |
| `${root}`、`${config_root}` | 绝对目录 |
| `${core_type}` | `mihomo` / `singbox` |
| `${service_name}` | 目标 core 的实际服务名 |
| `${core_bin}`、`${core_config_dir}`、`${core_config_path}` | 目标 core 的绝对路径 |
| `${proxy_port}` | 目标 API `/configs` 的有效 mixed-port，否则 port |

`${core_bin}` 的裸程序名先按 PATH 查到绝对路径；带分隔符的相对路径基于 `CONFIG_ROOT`。

只展开一次；`$${name}` 传入字面 `${name}`。空格、中文、引号、`$()` 保持参数数据。未知变量报错；仅被引用的缺值阻止执行。`${proxy_port}` 按需读取，使用目标 core 认证，secret 不传脚本；关闭代理无需 API 在线。

## 返回值与终端

普通动作：退出码 `0` 成功，非零失败。stdout 可写说明，stderr 写诊断。`$0` 是 shell 脚本名称，退出码用 `exit N` 返回，调用方用 `$?` 读取。

`status_core`：退出 `0`，stdout 只含一行 `active`、`inactive` 或 `missing`，接受末尾 LF/CRLF；UTF-8、无 BOM。已安装但停止或自身失败为 inactive；仅确认未安装才返回 missing。诊断写 stderr。

权限错误、工具缺失、非零退出、信号、格式错误、超时和输出超限均为 unknown。未配置状态动作显示 unsupported。状态表示当前运行状态，不表示是否自启。

内部状态查询始终捕获并消费结果，不弹窗、不转发终端输出，不受手动调用设置影响。后台查询关闭 stdin，超时 5 秒，stdout/stderr 各限 64 KiB。状态脚本必须非交互；若需要 sudo，配置 `sudo -n`，不得弹密码或 UAC。脚本不得自行脱离受管理进程组/Job；真正由服务管理器启动的 core 不属于脚本清理对象。

交互动作独占终端。输入线程确认停读后交接，命令继承标准流，结束后回车一次返回；Ctrl-C 中断命令。CLI 不增加回车等待。

Unix 默认配置通过 `/bin/sh` 调用脚本；脚本仅给需要 system 权限的具体命令加 sudo。user 不提权，root 不重复提权。systemd、OpenRC、launchd 状态查询均不使用 sudo。launchd 以当前用户读取域和 plist；读取失败仍报告查询错误。只有服务写操作可能要求认证。自定义脚本可直接调用 sudo，无需特殊包装。

凭据复用由 sudo 管理：有效 tty 缓存可在同一终端与会话内跨脚本使用；ppid 策略下，新脚本进程可能需要重新认证。clashtui 不存密码、不保活、退出不清缓存、不修改 sudoers。命令专属授权规则需匹配实际调用的命令。

默认不打印命令。调试时用 `/bin/sh -x service.sh restart system mihomo`，或在脚本内用 `set -x` / `set +x`。修改 `scripts.json` 时，将 `-x` 放在 `/bin/sh` 参数的脚本路径之前。跟踪输出写 stderr，状态 stdout 仍遵守协议。

## 平台边界

- systemd：enable/disable 不带 `--now`；用户服务不自动设置 linger。
- OpenRC：用户模式所有查询与写操作保留 `--user`，需管理器支持及 `XDG_RUNTIME_DIR`；只修改 `default` runlevel。
- launchd：启停保留 load/unload；自启用 enable/disable。用户模式是登录自启。手动启动/重启使用 `load -F`，允许关闭自启后仍运行，并保留持久 enable/disable 设置。
- Windows：NSSM 管理生命周期，SCM 查询状态；自启为 AUTO_START/DEMAND_START。写服务时按需 UAC，拒绝即失败。PowerShell 脚本 UTF-8 BOM，协议输出无 BOM；ExecutionPolicy Bypass 不覆盖组策略。
- Windows 代理：以当前用户修改 HKCU 并广播变更；启用、关闭各自独立。

Stop All 先预检两核 stop，再逐核执行并汇总错误。Switch Core 预检两核 stop 与目标 start；全部停止成功后启动目标，启动成功才保存选择，待终端输出送达、输出弹窗关闭后退出。失败保留原选择，报告已发生步骤，不自动回滚或重试。
