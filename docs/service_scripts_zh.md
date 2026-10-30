# 服务控制脚本

CoreSrvCtl 从 `CONFIG_ROOT/scripts.json` 生成菜单。CLI 服务命令、全局重启、状态页和切核使用同一配置。

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

首次运行从二进制内嵌资源补齐文件，无需网络。已有文件不覆盖；符号链接写入被拒绝。目录不可写时显示路径，不自动提权。修改过的默认资源旁可生成 `.new` 样本；已有 `.new` 也保留。

仅在 `scripts.json` 缺失时，旧 `service_controller`、`is_user` 用于选择默认模板。未知后端报错。两核后端或权限不同，生成带 `cores` 的同 ID 动作。`config.yaml` 不改写；其中服务名和 core 路径仍提供动态参数。

## 配置

```json
{
  "root": "scripts/coresrvctrl",
  "actions": [
    {
      "id": "start_core",
      "label": "启动核心",
      "cmd": {
        "program": "sudo",
        "args": ["--", "/bin/sh", "${root}/systemd/service.sh", "start", "system", "${service_name}"]
      }
    },
    {
      "id": "status_core",
      "label": "查看状态",
      "cmd": {
        "program": "/bin/sh",
        "args": ["${root}/systemd/service.sh", "status", "system", "${service_name}"]
      }
    }
  ]
}
```

`root`、`actions`、`id`、`label`、`cmd.program`、`cmd.args` 必填。拒绝未知字段、空标签、空程序。`id` 格式为 `[a-z][a-z0-9_-]*`。

可选 `"cores": ["mihomo"]` 或 `["singbox"]`；省略表示全部 core。使用机器名 `singbox`，不是显示名 `sing-box`。数组不可为空或重复。同 ID 可分核定义不同命令，但作用域不可重叠；共用动作不能再用同 ID 定向覆盖。

按当前 core 筛选，保持数组顺序。自定义 ID 可直接显示、执行。重新进入 tab 读取文件，保留仍存在的选中 ID。文件无效则禁用动作，不运行上次命令；正在执行的操作使用原快照。

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

后台查询关闭 stdin，超时 5 秒，stdout/stderr 各限 64 KiB。状态脚本必须非交互；若需要 sudo，配置 `sudo -n`，不得弹密码或 UAC。脚本不得自行脱离受管理进程组/Job；真正由服务管理器启动的 core 不属于脚本清理对象。

前台动作独占终端。输入线程确认停读后交接，命令继承标准流，结束后回车一次返回；Ctrl-C 中断命令。CLI 不增加回车等待。

sudo 应直接作为 program，脚本内不要再套 sudo。同一 clashtui 进程复用系统 tty/ppid 时间戳。无密码存储、无保活、退出不清缓存；策略过期仍可能再询问。`sudo /bin/sh SCRIPT` 与原来的精确 NOPASSWD 规则可能不同，可直接配置原授权命令；clashtui 不修改 sudoers。

## 平台边界

- systemd：enable/disable 不带 `--now`；用户服务不自动设置 linger。
- OpenRC：用户模式所有查询与写操作保留 `--user`，需管理器支持及 `XDG_RUNTIME_DIR`；只修改 `default` runlevel。
- launchd：启停保留 load/unload；自启用 enable/disable。用户模式是登录自启。disable 也阻止后续加载，手动启动不会暗中 enable。
- Windows：NSSM 管理生命周期，SCM 查询状态；自启为 AUTO_START/DEMAND_START。写服务时按需 UAC，拒绝即失败。PowerShell 脚本 UTF-8 BOM，协议输出无 BOM；ExecutionPolicy Bypass 不覆盖组策略。
- Windows 代理：以当前用户修改 HKCU 并广播变更；启用、关闭各自独立。

Stop All 先预检两核 stop，再逐核执行并汇总错误。Switch Core 预检两核 stop 与目标 start；全部停止成功后启动目标，启动成功才保存选择并退出。失败保留原选择，报告已发生步骤，不自动回滚或重试。
