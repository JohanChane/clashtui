> **重要通知**
> clashtui v0.3.0 已经 pre-release 了, 如果没有什么严重的问题, 则会在 5 月 23 号之后, 将发布 release。整个仓库会做如下改动:
> -   `main` 和 `dev` branch 会分别改名为 `archive/main` 和 `archive/dev`。它们已经 tag 为 `archive/v0.2.3` 和 `archive/v0.2.3-dev`
> -   `demotui` branch 会改名为 `main` 并基于它创建 `dev` branch

# ClashTui

<p>
  <a href="https://github.com/JohanChane/clashtui/releases"><img src="https://img.shields.io/github/v/release/JohanChane/clashtui" alt="Release"></a>
  <a href="https://github.com/JohanChane/clashtui/releases"><img src="https://img.shields.io/github/v/release/JohanChane/clashtui?include_prereleases&label=pre-release" alt="Pre-release"></a>
  <a href="https://github.com/JohanChane/clashtui/blob/main/LICENSE"><img src="https://img.shields.io/github/license/JohanChane/clashtui" alt="License"></a>
  <a href="https://github.com/JohanChane/clashtui/actions/workflows/build.yml"><img src="https://github.com/JohanChane/clashtui/actions/workflows/build.yml/badge.svg" alt="Build"></a>
  <a href="https://github.com/JohanChane/clashtui/actions/workflows/pr.yml"><img src="https://github.com/JohanChane/clashtui/actions/workflows/pr.yml/badge.svg" alt="PR"></a>
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
-   [ ] Windows

## 安装

### 依赖

-   sudo
-   fzf

### 想开启 tun 并有 root 权限

1. \[可选\] 从仓库中安装 mihomo 和 clashtui:

```sh
sudo pacman -S mihomo sing-box clashtui  # ArchLinux. (目前 clashtui 还没有上传最新的, 请手动编译安装 clashtui)
```

这一步的目的是保证当前环境中包含 mihomo, sing-box 和 clashtui，这样安装脚本会跳过安装它们的步骤。你也可以手动下载这两个工具，然后运行 which mihomo sing-box clashtui 来检查是否已正确配置。

2. 运行安装脚本

```sh
bash <(curl -fsSL https://raw.githubusercontent.com/JohanChane/clashtui/refs/heads/demotui/installs/install) --repo JohanChane/clashtui --branch demotui --core all
```

提示：由于安装脚本使用的资源是从 GitHub 上下载的，所以如果总是下载失败，可以先开启代理再运行脚本。

3. \[可选\] 将 `clashtui_mihomo.service/clashtui_singbox.service` 设置为开机启动

```sh
sudo systemctl enable clashtui_mihomo.service
# OR
sudo systemctl enable clashtui_singbox.service
```

### 没有 root 权限 (不开启 tun)

```sh
bash <(curl -fsSL https://raw.githubusercontent.com/JohanChane/clashtui/refs/heads/demotui/installs/install) --repo JohanChane/clashtui --branch demotui --core all --is-user
```

## 文档

| 文档 | 说明 |
|------|------|
| [使用指南](docs/getting_started_zh.md) | 详细使用教程：界面操作、命令行、订阅管理、配置说明、常见问题 |
| [功能设计](docs/ClashTui_feature_design_zh.md) | 功能设计文档：配置结构、订阅管理、Template 展开、sing-box 合并算法 |
| [架构](docs/architecture_zh.md) | 代码架构文档：模块结构、启动流程、TUI 事件循环、Tab 体系 |
| [开发约定](docs/development_conventions.md) | 分支命名、提交规范、CHANGELOG 约定 |

## 参与开发

欢迎提交 Issue 和 Pull Request。参与开发前请先阅读[开发约定](docs/development_conventions.md)。

快速了解项目, 加入开发:
1.  docs/ClashTui_feature_design_zh.md
2.  docs/architecture_zh.md
