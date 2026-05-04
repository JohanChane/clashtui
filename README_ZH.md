# ClashTui

![演示](https://github.com/user-attachments/assets/7a35f4a7-e400-4e73-b2ec-0d68f287b99c)

语言: [English](./README.md) | [中文](./README_ZH.md)

<details>
<summary>目录</summary>
<!-- vim-markdown-toc GFM -->

* [支持的平台](#支持的平台)
* [目标受众](#目标受众)
* [安装](#安装)
* [ClashTUI 使用](#clashtui-使用)
* [卸载](#卸载)
* [更多信息](#更多信息)
* [尝试新东西](#尝试新东西)
* [项目免责声明](#项目免责声明)

<!-- vim-markdown-toc -->
</details>

## 支持的平台

-   Linux
-   Windows (请查看 [Windows README](https://github.com/JohanChane/clashtui/blob/win/README_ZH.md))

## 目标受众

-   对 Clash 配置有一定了解
-   喜欢 TUI 软件

## 安装

1. \[可选\] 从仓库中安装 mihomo 和 clashtui:

```sh
sudo pacman -S mihomo clashtui  # ArchLinux
```

这一步的目的是保证当前环境中包含 mihomo 和 clashtui，这样安装脚本会跳过安装它们的步骤。你也可以手动下载这两个工具，然后运行 `which mihomo clashtui` 来检查是否已正确配置。

2. 运行安装脚本

```sh
bash -c "$(curl -fsSL https://raw.githubusercontent.com/JohanChane/clashtui/refs/heads/main/install )"
```

提示：由于安装脚本使用的资源是从 GitHub 上下载的，所以如果总是下载失败，可以先开启代理再运行脚本。

3. \[可选\] 将 `clashtui_mihomo.service` 设置为开机启动

```sh
sudo systemctl enable clashtui_mihomo.service
```

---

如果你想手动安装，请参考 [手动安装](./Doc/install_clashtui_manually_zh.md)

## ClashTUI 使用

查看 [clashtui_usage](./Doc/clashtui_usage.md)

## 卸载

```sh
curl -o /tmp/install https://raw.githubusercontent.com/JohanChane/clashtui/refs/heads/main/install
bash /tmp/install -u
```
## 更多信息

[文档](./Doc)

## 尝试新东西

- [demotui](https://github.com/JohanChane/demotui): clashtui 的新框架 (开发中)
- [dev](https://github.com/JohanChane/clashtui/tree/dev): 最终会被 demotui 取代

## 项目免责声明

此项目仅供学习和参考之用。作者并不保证项目中代码的准确性、完整性或适用性。使用者应当自行承担使用本项目代码所带来的风险。

作者对于因使用本项目代码而导致的任何直接或间接损失概不负责，包括但不限于数据丢失、计算机损坏、业务中断等。

使用者应在使用本项目代码前，充分了解其功能和潜在风险，并在必要时寻求专业建议。对于因对本项目代码的使用而导致的任何后果，作者不承担任何责任。

在使用本项目代码时，请遵守相关法律法规，不得用于非法活动或侵犯他人权益的行为。

作者保留对本免责声明的最终解释权，并可能随时对其进行修改和更新。
