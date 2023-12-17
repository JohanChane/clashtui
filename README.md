# ClashTui

![Demo](./assets/clashtui_demo.gif)

<details>
<summary>Table of Contents</summary>
<!-- vim-markdown-toc GFM -->

* [支持的平台](#支持的平台)
* [适用人群](#适用人群)
* [安装 Mihomo 服务 (启用 Tun 模式)](#安装-mihomo-服务-启用-tun-模式)
    * [Linux](#linux)
    * [Windows](#windows)
* [安装 clashtui](#安装-clashtui)
    * [Linux](#linux-1)
    * [Windows](#windows-1)
    * [配置 `basic_clash_config.yaml`](#配置-basic_clash_configyaml)
* [启动](#启动)
    * [Windows](#windows-2)
* [便携模式](#便携模式)
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

## 安装 Mihomo 服务 (启用 Tun 模式)

### Linux

比如: [ArchLinux](https://aur.archlinux.org/packages/mihomo)。

```sh
# cat /etc/pacman.d/hooks/mihomo.hook (没有类似于 hook 的系统可能要手动 setcap 或者使用 mihomo@root 服务)
[Trigger]
Operation = Install
Operation = Upgrade
Type = Path
Target = usr/bin/mihomo

[Action]
When = PostTransaction
Exec = /usr/bin/setcap 'cap_net_admin,cap_net_bind_service=+ep' /usr/bin/mihomo
# ---

paru -S mihomo

# systemctl edit mihomo
[Service]
# 删除原先的 ExecStart
ExecStart=
ExecStart=/usr/bin/mihomo -d /srv/mihomo -f /srv/mihomo/config.yaml
# ---

mkdir /srv/mihomo
cd /srv/mihomo
chown -R mihomo:mihomo /srv/mihomo
usermod -a -G mihomo <user>
groups <user>       # 查看是否已经加入 mihomo group
chmod g+w /srv/mihomo               # clashtui 要有创建文件的权限。
chmod g+w /srv/mihomo/config.yaml   # clashtui 要有写的权限。

systemctl enable mihomo  # 开机启动
systemctl restart mihomo  # 启动服务
```

建议先用一个可用的 mihomo 配置测试 mihomo 服务是否成功。检查是否缺少 [meta-rules-dat](https://github.com/MetaCubeX/meta-rules-dat) 文件。

### Windows

[安装 scoop](https://github.com/ScoopInstaller/Install) (可选):

```powershell
irm get.scoop.sh -outfile 'install.ps1'
.\install.ps1 -ScoopDir 'D:\Scoop' -ScoopGlobalDir 'D:\ScoopGlobal' -NoProxy    # 我选择安装在 D 盘。
```

比如:

-   通过 `scoop install mihomo` 安装 mihomo。或者, 下载一个适合自己系统的 [mihomo](https://github.com/MetaCubeX/mihomo/releases), 将其放在 `D:/PortableProgramFiles/mihomo/mihomo.exe`。
-   创建目录 `D:/MyAppData/mihomo` 和文件 `D:/MyAppData/mihomo/config.yaml`
-   安装 clashtui 后, 再操作。

如果可以访问 mihomo 客户端 (比如: metacubexd) 而无法访问需要代理的网站, 则尝试允许 `mihomo.exe` 通过防火墙:
-   通过 Scoop 安装的 mihomo: 允许 `D:\Scoop\apps\mihomo\1.17.0\mihomo.exe`, 而不是 current 路径的。之后 mihomo 升级版本之后, 可能还要继续这样的操作。
-   手动下载 mihomo 安装的: 允许 `D:/PortableProgramFiles/mihomo/mihomo.exe`。

## 安装 clashtui

### Linux

比如: ArchLinux

```sh
# 有最新的 [PKGBUILD](https://github.com/JohanChane/clashtui/blob/main/PKGBUILD)。
paru -S clashtui。      # 其他 linux 发行版, 手动下载, 将 clashtui 放在 PATH 即可。
clashtui                # 先运行会在 ~/.config/clashtui 生成一些默认文件。

# nvim ~/.config/clashtui/config.toml
[default]
# 下面参数对应命令 <clash_core_path> -d <clash_cfg_dir> -f <clash_cfg_path>
clash_core_path = "mihomo"
clash_cfg_dir = "/srv/mihomo"
clash_cfg_path = "/srv/mihomo/config.yaml"
clash_srv_name = "mihomo"       # systemctl {restart | stop} <clash_srv_name>
# ---
```

### Windows

手动下载安装 clashtui, 或者通过 `scoop bucket add extras; scoop install clashtui` 安装。后续会将文件 [clashtui.json](./Scoop/clashtui.json) 加入scoop extras 仓库。如果没有加入 scoop extras, 你可以将 clashtui.json 文件添加到 `D:\Scoop\buckets\extras\bucket\clashtui.json`, 这样就可以使用 `scoop install clashtui` 了。

先运行 clashtui, 会在 `%APPDATA%/clashtui` 生成一些默认文件。

修改 `%APPDATA%/clashtui/config.toml`:

```toml
[default]
# 下面参数对应命令 <clash_core_path> -d <clash_cfg_dir> -f <clash_cfg_path>
#clash_core_path = "D:/PortableProgramFiles/mihomo/mihomo.exe"
clash_core_path = "D:/Scoop/shims/mihomo.exe"       # `Get-Command mihomo`
clash_cfg_dir = "D:/MyAppData/mihomo"
clash_cfg_path = "D:/MyAppData/mihomo/config.yaml"
clash_srv_name = "mihomo"       # nssm {install | remove | restart | stop | edit} <clash_srv_name>
```

改好之后, 将 clashtui, nssm 加入 PATH:
-   scoop 安装 clashtui 的: scoop install nssm
-   手动下载安装 clashtui 的: 将 `D:/PortableProgramFiles/clashtui` 加入 PATH。

运行 clashtui。在 `ClashSrvCtl` Tab 选择 `InstallSrv`, 程序会根据上面的配置安装 `mihomo` 内核服务。该服务会开机启动。安装之后启动内核服务, 输入 `R` 即可。

### 配置 `basic_clash_config.yaml`

自行配置 `{~/.config | %APPDATA%}/clashtui/basic_clash_config.yaml`。该文件的一些基础字段会合并到 `clash_cfg_path`。可以参考[这里](./App/basic_clash_config.yaml)配置 tun 模式。

## 启动

### Windows

选择其中一种方式:

-   将 clashtui 这个命令加入 PATH。在终端或 `win + r` 或在文件资源管理器的地址栏输入 clashtui 启动。
-   双击 clashtui。

*clashtui 使用 [crossterm](https://docs.rs/crossterm/latest/crossterm/), [ratatui](https://github.com/ratatui-org/ratatui) 实现, Windows 最好使用 [Windows Terminal](https://github.com/microsoft/terminal)。在 Windows Terminal 中设置命令的启动方式使用 `Windows Terminal`, 则执行 clashtui 命令会自动使用 Windows Teminal 打开。*

## 便携模式

在 clashtui 程序所有的目录创建一个名为 `data` 的文件夹。则会将数据放在 `data` 内而不是 `~/.config/clashtui` 或 `%APPDATA%/clashtui`。

## 使用说明

按 `?` 显示 help。

### 导入链接

-   在 Profile 区域, 按 `i` 输入 Name (尽量不使用后缀) 和 Uri
-   按 `U` 更新 Profile 的依赖的所有资源。默认使用自身代理更新, 如果开启 tun 模式或系统代理且没有可用节点的情况下, 先停止 mihomo 服务 (按 `S`), 再更新即可。
-   按 `Enter` 选择该 Profile。
-   在浏览器输入 `http://127.0.0.1:9090/ui`。

如果 Windows 平台无法打开 `http://127.0.0.1:9090/ui`:
-   在 `ClashSrvCtl` 选择 `TestClashConfig` 检测配置语法是否正确和是否自动下载了 geo 文件。
-   按 `L` 查看日志。(`H` 打开 clashtui config dir。`G` 打开 clash config dir。查看相关的文件是否正确。)
-   可以使用 `netstat -aon | findstr "9090"` 查看端口是否存在, 如果不存在可以换一个 compatible 版本的 mihomo。
-   如果可以打开, 但是无法访问需要代理的网站。可以允许 `mihomo` 通过防火墙。

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

下载的 clashtui 一般会附带 templates。如果没有, 在[这里](https://github.com/JohanChane/clashtui/tree/main/App/templates)有最新的 templates。

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
# 不添加 `interval: 3600` 的原因: mihomo 重载配置时, 如果检测到要更新配置时会更新 url, 这样会导致加载速度慢。
pp: &pp {intehealth-check: {enable: true, url: https://cp.cloudflare.com/generate_204, interval: 300}}
delay_test: &delay_test {url: https://cp.cloudflare.com/generate_204, interval: 300}
```

为 `template_proxy_providers` 的每个链接生成一个 proxy-provider:

```yaml
proxy-providers:
  provider:
    tpl_param:
    type: http    # type 字段要放在此处, 不能放入 pp。原因是要用于更新资源。
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

-   basic_clash_config.yaml: mihomo 配置的基本字段, 会合并到 `clash_cfg_path`。
-   config.yaml: clashtui 的配置。

## 项目免责声明

此项目仅供学习和参考之用。作者并不保证项目中代码的准确性、完整性或适用性。使用者应当自行承担使用本项目代码所带来的风险。

作者对于因使用本项目代码而导致的任何直接或间接损失概不负责，包括但不限于数据丢失、计算机损坏、业务中断等。

使用者应在使用本项目代码前，充分了解其功能和潜在风险，并在必要时寻求专业建议。对于因对本项目代码的使用而导致的任何后果，作者不承担任何责任。

在使用本项目代码时，请遵守相关法律法规，不得用于非法活动或侵犯他人权益的行为。

作者保留对本免责声明的最终解释权，并可能随时对其进行修改和更新。
