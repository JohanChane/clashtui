# ClashTui

<p>
  <a href="https://github.com/JohanChane/clashtui/releases"><img src="https://img.shields.io/github/v/release/JohanChane/clashtui" alt="Release"></a>
  <a href="https://github.com/JohanChane/clashtui/releases"><img src="https://img.shields.io/github/v/release/JohanChane/clashtui?include_prereleases&label=pre-release" alt="Pre-release"></a>
  <a href="https://github.com/JohanChane/clashtui/blob/main/LICENSE"><img src="https://img.shields.io/github/license/JohanChane/clashtui" alt="License"></a>
  <a href="https://github.com/JohanChane/clashtui/actions/workflows/ci.yml"><img src="https://github.com/JohanChane/clashtui/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/JohanChane/clashtui/actions/workflows/release.yml"><img src="https://github.com/JohanChane/clashtui/actions/workflows/release.yml/badge.svg" alt="Release CI"></a>
</p>

<video src="https://github.com/user-attachments/assets/7808534a-84bc-4967-a024-534487ab7aaf" controls width="100%"></video>

Language: [English](./README.md) | [中文](./README_ZH.md)

ClashTui 是一个终端用户界面（TUI）代理管理工具，支持 **Mihomo**（Clash.Meta）和 **sing-box** 两种代理核心。你可以在终端里完成切换节点、更新订阅、管理连接和控制服务启停等操作。

## 特性

- **双核心支持** — 同时兼容 Mihomo 和 sing-box，可在界面中随时切换
- **订阅管理** — 支持 File、URL、Template (ClashTui 的 template) 三种 profiles。
- **代理切换** — 按组/按节点切换，支持延迟测试
- **连接监控** — 实时查看所有活动连接，可关闭单个或全部连接
- **服务控制** — 通过 systemd 管理核心的启动、停止和重启
- **日志查看** — 在界面内实时查看核心日志
- **命令行模式** — 支持 `profile`、`mode`、`service`、`update` 等子命令，适合脚本和自动化
- **配置覆盖** — 通过 `core_override_config` 在不修改订阅原始文件的前提下改写最终配置
- **Template 模板** — 用模板 + 节点分组自动生成配置文件，支持变量展开 (对 singbox 建议使用该功能, 能解决烦人的配置的版本问题)
- **自定义按键** — 每个标签页的快捷键都可以通过 `keymap.yaml` 自行定义
- **自定义主题** - 通过 `theme.yaml` 自行定义
- **which panel** - 方便用户操作
- **支持 fzf** - 用户可以使用 fzf 进行选择

## 支持的平台

-   [x] Linux
-   [x] macOs
-   [x] Windows

## 安装

### 依赖

Linux and macOS:
-   sudo
-   fzf

Windows:
-   nssm

### Shell 安装脚本（Linux/macOS）

脚本默认开启询问。默认选择：系统安装、两个核心、对应模板和 mihomo 规则数据。服务管理器默认使用 `--service-controller auto-detect` 自动检测：Linux 使用 systemd 或 OpenRC，macOS 使用 launchd。`--no-prompt`（或 `-y` / `--yes`）跳过询问，使用相同默认值。

```sh
# 在本地仓库中交互安装
git clone https://github.com/JohanChane/clashtui.git
cd clashtui
bash installs/install

# 指定 contrib 目录
bash installs/install --contrib-dir /path/to/contrib

# 免询问，安装到当前用户目录
bash <(curl -fsSL https://raw.githubusercontent.com/JohanChane/clashtui/refs/heads/main/installs/install) --is-user --no-prompt
```

用 `--core mihomo` 或 `--core sing-box` 选择单个核心。`--templates no` 跳过模板，`--rules-dat no` 跳过 mihomo 规则数据。用户安装使用 `~/.local/clashtui`，不开启 TUN。完整参数见 `bash installs/install --help`。

脚本资源、默认配置和模板按以下顺序选择：`--contrib-dir PATH`、安装脚本同级或上一级的 `contrib/`、release 的 `contrib.tar.gz`。`PATH` 支持相对或绝对路径，目录内须包含 `scripts/`、`default_configs/` 和 `templates/`。显式指定的目录缺失或不完整时直接报错，不回退。此参数只选择资源目录，缺失的程序和已启用的规则数据仍可能需要下载。

脚本优先复用安装目录中的程序，再查找 `PATH`，只下载缺失的程序。下载的 clashtui 程序和 contrib 使用同一 release。`--version <tag>` 固定这两者的下载版本（默认 `latest`），本地 contrib 仍优先。缺失的核心始终下载各自的最新 release。用 `--repo <owner/repo>` 指定 ClashTui 下载仓库。

重装会用安装默认值覆盖 `config.yaml`、`scripts.json`、核心配置和 override，以及安装资源。原内容保存为 `<文件名>.bak.N`；内容相同的文件不重复备份。安装后不会自动启动服务或开启自启动。

免询问模式使用 `sudo -n`，权限不足会立即失败。系统安装前先运行 `sudo -v`，或以 root 身份运行。OpenRC 用户服务写入 `${XDG_CONFIG_HOME:-$HOME/.config}/rc/init.d`，无需 sudo。

免询问卸载：`bash installs/install --uninstall --no-prompt`。卸载用户安装时加上 `--is-user`。卸载会保留配置、备份、日志和系统账号。

### 想开启 tun 并有 root 权限

#### Linux

1. \[可选\] 从仓库中安装 mihomo 和 clashtui:

```sh
sudo pacman -S mihomo sing-box clashtui  # ArchLinux
```

这一步的目的是保证当前环境中包含 mihomo, sing-box 和 clashtui，这样安装脚本会跳过安装它们的步骤。你也可以手动下载这两个工具，然后运行 which mihomo sing-box clashtui 来检查是否已正确配置。

2. 运行安装脚本

```sh
bash <(curl -fsSL https://raw.githubusercontent.com/JohanChane/clashtui/refs/heads/main/installs/install) --core all
```

提示：由于安装脚本使用的资源是从 GitHub 上下载的，所以如果总是下载失败，可以先开启代理再运行脚本。

3. \[可选\] 将 `clashtui_mihomo.service/clashtui_singbox.service` 设置为开机启动

```sh
sudo systemctl enable clashtui_mihomo.service
# OR
sudo systemctl enable clashtui_singbox.service
```

#### macOS

1. \[可选\] 从仓库中安装 mihomo 和 clashtui:

```sh
brew install mihomo sing-box # 目前 clashtui 还没有上传, 请手动安装 clashtui
```

2. 运行安装脚本

和 Linux 一样

3. \[可选\] 将 `clashtui_mihomo.service/clashtui_singbox.service` 设置为开机启动

```sh
sudo launchctl load -w /Library/LaunchDaemons/clashtui_mihomo.plist
# 或
sudo launchctl load -w /Library/LaunchDaemons/clashtui_singbox.plist
```

#### Windows

1. \[可选\] 从仓库中安装 mihomo 和 clashtui:

```powershell
scoop install mihomo sing-box clashtui
# 验证
Get-Command mihomo sing-box clashtui
```

2. 运行安装脚本

```powershell
# 默认安装到 D:\ClashTui
irm https://raw.githubusercontent.com/JohanChane/clashtui/refs/heads/main/installs/install.ps1 | iex

# 安装到自定义目录 (路径不能有空格)
iex "& {$(irm https://raw.githubusercontent.com/JohanChane/clashtui/refs/heads/main/installs/install.ps1)} -Core all -InstallDir 'D:\MyTools\ClashTui'"

# 只安装 mihomo core
iex "& {$(irm https://raw.githubusercontent.com/JohanChane/clashtui/refs/heads/main/installs/install.ps1)} -Core mihomo"
```

3. 启动 clashtui 安装 clashtui_mihomo/clashtui_singbox 服务

安装脚本不注册 Windows Service。启动 clashtui 后，使用 CoreSrvCtl 安装并启动 core 服务。
将其中一个服务设置为开机启动即可 (不要两个同时开机启动)

### 没有 root 权限 (不开启 tun)

#### Linux

```sh
bash <(curl -fsSL https://raw.githubusercontent.com/JohanChane/clashtui/refs/heads/main/installs/install) --core all --is-user
```

开机启动:

```sh
systemctl --user enable clashtui_mihomo.service
# 或
systemctl --user enable clashtui_singbox.service
```

#### macOS

```sh
bash <(curl -fsSL https://raw.githubusercontent.com/JohanChane/clashtui/refs/heads/main/installs/install) --core all --is-user
```

开机启动:

```sh
launchctl load -w ~/Library/LaunchAgents/clashtui_mihomo.plist
# 或
launchctl load -w ~/Library/LaunchAgents/clashtui_singbox.plist
```

## FAQ

- 文件权限之类的问题: 因为 clashtui 需要用到组权限, 所以请重新登录使组权限生效。
- connection refused 的问题: 比如: mihomo, 请使用 `netstat -utapln | grep 9090` 检查 clash api 端口有没有打开。

## 文档

| 文档 | 说明 |
|------|------|
| [使用指南](docs/getting_started_zh.md) | 详细使用教程：界面操作、命令行、订阅管理、配置说明、常见问题 |
| [手动安装](docs/install_manually_zh.md) | system/user 模式手动安装步骤 |
| [功能设计](docs/ClashTui_feature_design_zh.md) | 功能设计文档：配置结构、订阅管理、Template 展开、sing-box 合并算法 |
| [架构](docs/architecture_zh.md) | 代码架构文档：模块结构、启动流程、TUI 事件循环、Tab 体系 |
| [开发约定](docs/development_conventions.md) | 分支命名、提交规范、CHANGELOG 约定 |

## 参与开发

欢迎提交 Issue 和 Pull Request。参与开发前请先阅读[开发约定](docs/development_conventions.md)。

快速了解项目, 加入开发:
1. [功能设计](docs/ClashTui_feature_design_zh.md) — 了解功能设计
2. [架构](docs/architecture_zh.md) — 了解代码结构
