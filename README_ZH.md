# ClashTUI

## 安装与更新ClashTui

* 从[Github Release][GhRelease]下载预编译文件
  **注意**：Linux用户可能需要为其赋予可执行权限 (`chmod +x /path/to/clashtui`)
* 从[Crates.io](https://crates.io/crates/clashtui)安装（即 `cargo install clashtui` ）
  **注意**：目前crates.io上的版本为`v0.1.0`，不建议使用此方式
* 运行 `clashtui update clashtui` 从[Github Release][GhRelease]下载更新

[GhRelease]: https://github.com/JohanChane/clashtui/releases

## 配置ClashTui

### 配置文件夹

* 默认情况下，clashtui会在 `~/.config/clashtui`(linux)或 `C:\Users\你的用户名\AppData\clashtui`(windows)下储存配置文件
* 在clashtui可执行文件同目录下创建 `data` 文件夹会使clashtui在其下储存配置文件
* 也可以在运行时通过 `--config-dir=` 或 `CLASHTUI_CONFIG_DIR=` 指定

```
ls
---
clashtui
data/
```

### 初始配置

第一次运行clashtui，或clashtui没能检测到配置文件时，clashtui会请求初始化默认配置。

如果您是从旧版本升级而来，请根据原版本选择 `clashtui migrate <VERSION>`，
您可以通过 `clashtui help migrate` 获取帮助

### 配置目录结构

```yaml
basic_clash_config.yaml # mihomo基础配置文件，clashtui会自动合并此文件到你选择的mihomo配置中
clashtui.db             # clashtui数据库，yaml格式，不建议手动编辑
clashtui.log            # clashtui日志
config.yaml             # clashtui配置文件, yaml格式
profiles/               # mihomo配置文件夹，来自订阅链接或本地生成
templates/              # clashtui模板文件夹，用于生成mihomo配置文件
```

#### basic_clash_config.yaml
clashtui会自动覆盖所有选项，你可以在此设置 `log-level, secret, external-controller` 等，
对 `rules, rule-providers, proxys`等 的更改也会自动合并

``` yaml
# This shows how it works
# basic_clash_config.yaml
log-level: error
tun:
  enable: true
rules:
- IP-CIDR,127.0.0.1/8,DIRECT
rule-providers:  
  AWAvenue:
    type: http

# The origin profile
log-level: info
rules:
- IP-CIDR,192.168.0.1/8,DIRECT
- IP-CIDR,192.168.0.2/8,DIRECT

# Merged profile
log-level: error # Overwrite
tun: # Overwrite
  enable: true
rules: # Extend to original sequence
- IP-CIDR,127.0.0.1/8,DIRECT
- IP-CIDR,192.168.0.1/8,DIRECT
- IP-CIDR,192.168.0.2/8,DIRECT
rule-providers: # Add if not exist
  AWAvenue:
    type: http
```

#### config.yaml
``` yaml
basic:
  clash_config_dir: /path/to/mihomo_config_dir # mihomo配置文件夹路径
  clash_bin_path: /path/to/mihomo # mihomo可执行文件位置
  clash_config_path: /path/to/mihomo_config_file # mihomo配置文件路径
service:
  clash_srv_name: mihomo # mihomo服务名称
  is_user: true # Windows 用户可忽略此项
timeout: 10 # 等待超时，若网络情况不好可适当增大此值
edit_cmd: code %s # 调用外部编辑器，%s为文件路径
open_dir_cmd: open %s
```

## 使用ClashTui

运行 `clashtui help` 获取帮助

运行 `clashtui` 会自动进入TUI，您可以使用`?`获取帮助

## 配置定时更新

### 使用systemd

将这两个文件 [`clashtui.service`](Doc/systemd/clashtui.service) 和 [`clashtui.timer`](Doc/systemd/clashtui.timer) 复制到`~/.config/systemd/user/`下，随后使用`systemctl --user enable clashtui.timer`(可能需要先运行`systemctl --user daemon-reload`)启用即可。

现在，clashtui将在开机成功后1分钟后以1天一次的频率尝试更新所有配置文件，可通过`journalctl --user -u clashtui`查看所有日志。

### 使用cronie

使用 `crontab -e` 打开编辑器并追加 `0 10,14,16,22 * * * /usr/bin/env clashtui profile update -a >> ~/cron.out 2>&1`。[Cronie相关使用方式]((https://wiki.archlinuxcn.org/wiki/Cron))


# Mihomo

## 安装或更新mihomo

* 你可以运行 `clashtui update mihomo` 下载或更新mihomo
* 或从[官方发布页](https://github.com/MetaCubeX/mihomo/releases)下载
* 对于Windows用户，您也可以使用[scoop](https://github.com/ScoopInstaller/Install)安装

## 配置mihomo运行服务

### 对于Linux用户

#### 系统级服务

请参考[官方示例](https://wiki.metacubex.one/startup/service/)

#### 用户级服务（推荐）

> **注意**：对于Linux用户，为使用Tun，需要赋予mihomo特殊权限。你可以通过clashtui tui界面下`ServiceCTL`下`SetPermission`选项，或运行 `sudo setcap 'cap_net_admin,cap_net_bind_service=+ep' /path/to/mihomo` 为其授予权限。

运行 `systemctl --user edit mihomo`，并复制以下内容

**注意**: /path/to/请使用实际路径替代。systemd使用 `%h` 表示用户目录

```systemd
[Unit]
Description=mihomo Daemon, Another Clash Kernel.
After=network.target NetworkManager.service systemd-networkd.service iwd.service

[Service]
Type=simple
LimitNPROC=4096
LimitNOFILE=1000000
WorkingDirectory=/path/to/mihomo
Restart=always
ExecStartPre=/usr/bin/sleep 1s
ExecStart=/path/to/mihomo -d /path/to/mihomo_config_dir
ExecReload=/bin/kill -HUP $MAINPID
 
[Install]
WantedBy=default.target
```

随后您便可以使用 `systemctl --user` 管理服务，或使用 `journalctl --user` 检查日志了(相关使用方式可以参考[官方示例](https://wiki.metacubex.one/startup/service/))

### 对于Windows用户

安装`nssm`并将其添加到环境变量中。
在完成[clashtui配置](#配置clashtui)后，启动clashtui，在`ServiceCTL`界面选择`InstallSrv`，clashtui将自动为您完成剩下的工作。

初次配置完成后，您可能需要重启系统或者选择`RestartClashService`以启动服务。

# ClashTui 模板
这是clashtui独有功能。

TODO

# 其他
您可以查看[Doc](./Doc/)获取其他说明文件。

# 项目免责声明

此项目仅供学习和参考之用。作者并不保证项目中代码的准确性、完整性或适用性。使用者应当自行承担使用本项目代码所带来的风险。

作者对于因使用本项目代码而导致的任何直接或间接损失概不负责，包括但不限于数据丢失、计算机损坏、业务中断等。

使用者应在使用本项目代码前，充分了解其功能和潜在风险，并在必要时寻求专业建议。对于因对本项目代码的使用而导致的任何后果，作者不承担任何责任。

在使用本项目代码时，请遵守相关法律法规，不得用于非法活动或侵犯他人权益的行为。

作者保留对本免责声明的最终解释权，并可能随时对其进行修改和更新。