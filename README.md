# ClashTui

![Demo](./assets/clashtui_demo.gif)

<details>
<summary>Table of Contents</summary>
<!-- vim-markdown-toc GFM -->

* [支持的平台](#支持的平台)
* [适用人群](#适用人群)
* [启动](#启动)
    * [Windows](#windows)
* [安装](#安装)
    * [Windows](#windows-1)
    * [Linux](#linux)
* [使用说明](#使用说明)
    * [导入链接](#导入链接)
    * [使用配置模板](#使用配置模板)
    * [高级使用](#高级使用)
        * [配置打开文件和目录的命令](#配置打开文件和目录的命令)
        * [自定义配置模板](#自定义配置模板)
* [clashtui 的文件结构](#clashtui-的文件结构)
* [项目免责声明](#项目免责声明)

<!-- vim-markdown-toc -->
</details>

## 支持的平台

-   Windows
-   Linux

## 适用人群

-   对 clash 配置有一定了解。
-   喜欢 TUI 软件。

## 启动

### Windows

选择其中一种方式:

-   将 clashtui 这个命令加入 PATH。在终端或 `win + r` 或在文件资源管理器的地址栏输入 clashtui 启动。
-   双击 clashtui。

*clashtui 使用 [crossterm](https://docs.rs/crossterm/latest/crossterm/), [ratatui](https://github.com/ratatui-org/ratatui) 实现, Windows 最好使用 [Windows Terminal](https://github.com/microsoft/terminal)。在 Windows Terminal 中设置命令的启动方式使用 `Windows Terminal`, 则执行 clashtui 命令会自动使用 Windows Teminal 打开。*

## 安装

### Windows

在 `ClashSrvCtl` Tab 选择 `InstallSrv` 安装 `clash-meta` 内核服务。该服务会开机启动。安装之后启动内核服务, 输入 `R` 即可。

### Linux

比如: ArchLinux

```sh
paru -S clash-meta。
systemctl edit clash-meta@root  # tun 模式需要管理员权限启动。
```

修改 clash-meta@root unit:

```
[Service]
# 删除原先的 ExecStart
ExecStart=
# 修改 `-d, -f` 参数。我的 clashtui 放在在 `/opt/clashtui`
ExecStart=/usr/bin/clash -d /opt/clashtui/clash_config -f /opt/clashtui/final_clash_config.yaml
```

```sh
systemctl enable clash-meta@root  # 开机启动
systemctl restart clash-meta@root  # 启动服务
```

## 使用说明

按 `?` 显示 help。

### 导入链接

-   在 Profile 区域, 按 `i` 输入 Name (尽量不使用后缀) 和 Uri
-   按 `U` 更新 Profile 的依赖的所有资源。默认使用自身代理更新, 如果开启 tun 模式或系统代理且没有可用节点的情况下, 先停止 clash-meta 服务 (按 `S`), 再更新即可。
-   按 `Enter` 选择该 Profile。
-   在浏览器输入 `http://127.0.0.1:9090/ui`。

如果 Windows 平台无法打开 `http://127.0.0.1:9090/ui`:
-   在 `ClashSrvCtl` 选择 `TestClashConfig` 检测配置语法是否正确和是否自动下载了 geo 文件。
-   按 `L` 查看日志。
-   可以使用 `netstat -aon | findstr "9090"` 查看端口是否存在, 如果不存在可以换一个 compatible 版本的 clash-meta。
-   如果可以打开, 但是无法访问需要代理的网站。可以允许 `clash-meta` 通过防火墙。

支持导入文件配置。`Uri` 输入是文件路径即可。

### 使用配置模板

-   按 `t` 切换到 Templates 区域。
-   选择 `template_proxy_providers`, 按 `e` 编辑, 输入订阅链接 (直接复制链接, 不用修改) 即可。

    比如:

    ```
    https://....
    https://....

    # 支持注释
    #https://....
    ```

-   按 `Enter` 生成配置到 `Profile`。按 `p` 切换回 Profile, `Enter` 选择该配置即可。

### 高级使用

#### 配置打开文件和目录的命令

在 `./data/config.toml` 中配置即可。`%s` 会自动替换为选择的文件的路径。

比如:

Linux:

```toml
[default]
edit_cmd = "alacritty -e nvim %s"
opendir_cmd = "alacritty -e ranger %s"
```

Windows:

```
[default]
edit_cmd = "notepad %s"
```

#### 自定义配置模板

模板功能是 clashtui 独有的。具体使用规则参考提供例子模板。

定义重复使用的字段:

```yaml
# 不添加 `interval: 3600` 的原因: clash-meta 重载配置时, 如果检测到要更新配置时会更新 url, 这样会导致加载速度慢。
pp: &pp {type: http, intehealth-check: {enable: true, url: https://cp.cloudflare.com/generate_204, interval: 300}}
delay_test: &delay_test {url: https://cp.cloudflare.com/generate_204, interval: 300}
```

为 `template_proxy_providers` 的每个链接生成一个 proxy-provider:

```yaml
proxy-providers:
  provider:
    tpl_param:
    <<: *pp
```

为每个 proxy-providers 生成一个 `Select, Auto` proxy-group。

```yaml
proxy-groups:
  - name: "Select"
    tpl_param:
      providers: ["provider"]
    type: select

  - name: "Auto"
    tpl_param:
      providers: ["provider"]
    type: url-test
    <<: *delay_test
```

使用 `Select, Auto` proxy-groups:

```yaml
proxy-groups:
  - name: "Entry"
    type: select
    proxies:
      - <Auto>
      - <Select>
```

## clashtui 的文件结构

-   data: 存放个人数据
    -   basic_clash_config.yaml: clash-meta 配置的基本字段, 会合并到 `final_clash_config`。
    -   config.yaml: clashtui 的配置。
-   final_clash_config.yaml: clash-meta 使用的配置。
-   clash_config: clash-meta config directory。


## 项目免责声明

此项目仅供学习和参考之用。作者并不保证项目中代码的准确性、完整性或适用性。使用者应当自行承担使用本项目代码所带来的风险。

作者对于因使用本项目代码而导致的任何直接或间接损失概不负责，包括但不限于数据丢失、计算机损坏、业务中断等。

使用者应在使用本项目代码前，充分了解其功能和潜在风险，并在必要时寻求专业建议。对于因对本项目代码的使用而导致的任何后果，作者不承担任何责任。

在使用本项目代码时，请遵守相关法律法规，不得用于非法活动或侵犯他人权益的行为。

作者保留对本免责声明的最终解释权，并可能随时对其进行修改和更新。
