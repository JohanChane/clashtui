# ClashTui

![Demo](https://github.com/user-attachments/assets/7a35f4a7-e400-4e73-b2ec-0d68f287b99c)

Language: [English](./README.md) | [中文](./README_ZH.md)

## Table of Contents

<details>
<summary>Table of Contents</summary>
<!-- vim-markdown-toc GFM -->

* [Install](#install)
* [ClashTUI Usage](#clashtui-usage)
* [Uninstall](#uninstall)
* [See more](#see-more)
* [项目免责声明](#项目免责声明)

<!-- vim-markdown-toc -->
</details>

## Install

```powershell
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
irm https://raw.githubusercontent.com/JohanChane/clashtui/refs/heads/win/install.ps1 -outfile 'install.ps1'
.\install.ps1 -InstallDir "D:\ClashTUI" # 安装路径不要有空格
```

如果你想手动安装, 请参考 [Install manually](https://github.com/JohanChane/clashtui/blob/main/Doc/win/install_clashtui_manually_zh.md)

## ClashTUI Usage

See [ClashTUI Usage](https://github.com/JohanChane/clashtui/blob/main/Doc/win/clashtui_usage.md)

## Uninstall

```powershell
irm https://raw.githubusercontent.com/JohanChane/clashtui/refs/heads/win/install.ps1 -outfile 'install.ps1'
.\install.ps1 -Uninstall
```

## See more

[Doc](https://github.com/JohanChane/clashtui/tree/main/Doc)

## 项目免责声明

此项目仅供学习和参考之用。作者并不保证项目中代码的准确性、完整性或适用性。使用者应当自行承担使用本项目代码所带来的风险。

作者对于因使用本项目代码而导致的任何直接或间接损失概不负责，包括但不限于数据丢失、计算机损坏、业务中断等。

使用者应在使用本项目代码前，充分了解其功能和潜在风险，并在必要时寻求专业建议。对于因对本项目代码的使用而导致的任何后果，作者不承担任何责任。

在使用本项目代码时，请遵守相关法律法规，不得用于非法活动或侵犯他人权益的行为。

作者保留对本免责声明的最终解释权，并可能随时对其进行修改和更新。
