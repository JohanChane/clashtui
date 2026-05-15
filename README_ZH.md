# Clashtui

Clashtui 是一个终端用户界面（TUI）代理管理工具，支持 **Mihomo**（Clash.Meta）和 **sing-box** 两种代理核心。你可以在终端里完成切换节点、更新订阅、管理连接和控制服务启停等操作。

## 特性

- **双核心支持** — 同时兼容 Mihomo 和 sing-box，可在界面中随时切换
- **订阅管理** — 支持 File、URL、Template (clashtui 的 template) 三种 profiles。
- **代理切换** — 按组/按节点切换，支持延迟测试
- **连接监控** — 实时查看所有活动连接，可关闭单个或全部连接
- **服务控制** — 通过 systemd 管理核心的启动、停止和重启
- **日志查看** — 在界面内实时查看核心日志
- **命令行模式** — 支持 `profile`、`mode`、`service`、`update` 等子命令，适合脚本和自动化
- **配置覆盖** — 通过 `core_override_config` 在不修改订阅原始文件的前提下改写最终配置
- **Template 模板** — 用模板 + 节点分组自动生成配置文件，支持变量展开 (对 singbox 建议使用该功能, 能解决烦人的配置的版本问题)
- **自定义按键** — 每个标签页的快捷键都可以通过 `keymap.yaml` 自行定义
- **支持 fzf** - 支持 fzf

## 安装

想开启 tun 并有 root 权限:

```sh
./install
```

没有 root 权限 (不开启 tun):

```sh
./install --is-user
```

## 文档

| 文档 | 说明 |
|------|------|
| [使用指南](docs/getting_started_zh.md) | 详细使用教程：界面操作、命令行、订阅管理、配置说明、常见问题 |
| [功能设计](docs/clashtui_feature_design_zh.md) | 功能设计文档：配置结构、订阅管理、Template 展开、sing-box 合并算法 |
| [架构](docs/architecture_zh.md) | 代码架构文档：模块结构、启动流程、TUI 事件循环、Tab 体系 |

## 参与开发

欢迎提交 Issue 和 Pull Request。

快速了解项目, 加入开发:
1.  docs/clashtui_feature_design_zh.md
2.  docs/architecture_zh.md