# ClashTui

![Demo](./assets/clashtui_demo.gif)

<details>
<summary>Table of Contents</summary>
<!-- vim-markdown-toc GFM -->

* [支持的平台](#支持的平台)
* [适用人群](#适用人群)
* [安装 Mihomo 服务 (启用 Tun 模式)](#安装-mihomo-服务-启用-tun-模式)
* [安装 clashtui](#安装-clashtui)
    * [配置 `basic_clash_config.yaml`](#配置-basic_clash_configyaml)
* [便携模式](#便携模式)
* [使用说明](#使用说明)
    * [导入链接](#导入链接)
    * [结合 cronie 定时更新 profiles](#结合-cronie-定时更新-profiles)
    * [使用配置模板](#使用配置模板)
    * [高级使用](#高级使用)
        * [配置打开文件和目录的命令](#配置打开文件和目录的命令)
        * [自定义配置模板](#自定义配置模板)
* [clashtui 的文件结构](#clashtui-的文件结构)
* [项目免责声明](#项目免责声明)

<!-- vim-markdown-toc -->
</details>

## 支持的平台

-   Linux
-   Windows. 请转到 [Windows README](https://github.com/JohanChane/clashtui/blob/win/README_ZH.md)

## 适用人群

-   对 clash 配置有一定了解。
-   喜欢 TUI 软件。

## 安装 Mihomo 服务 (启用 Tun 模式)

比如: [ArchLinux](https://aur.archlinux.org/packages/mihomo)。

```sh
# ## 安装 mihomo
paru -S mihomo

# ## 添加 mihomo hook
# cat /etc/pacman.d/hooks/mihomo.hook (没有类似于 hook 的系统可以使用 ClashSrvCtl Tab 的 SetPermission 或者使用 mihomo@root 服务)
[Trigger]
Operation = Install
Operation = Upgrade
Type = Path
Target = usr/bin/mihomo

[Action]
When = PostTransaction
Exec = /usr/bin/setcap 'cap_net_admin,cap_net_bind_service=+ep' /usr/bin/mihomo

# ## 编辑 mihomo service unit
# systemctl edit mihomo
[Service]
# 删除原先的 ExecStart
ExecStart=
ExecStart=/usr/bin/mihomo -d /srv/mihomo -f /srv/mihomo/config.yaml

# ## 创建 /srv/mihomo
mkdir /srv/mihomo
cd /srv/mihomo
chown -R mihomo:mihomo /srv/mihomo
usermod -a -G mihomo <user>
groups <user>       # 查看是否已经加入 mihomo group

# Optional. 0.1.0 之后版本的 clashtui 会自动修复文件的权限。
chmod g+w /srv/mihomo               # clashtui 要有创建文件的权限。
chmod g+s /srv/mihomo               # 使 clashtui 创建的文件的组为 mihomo。为了使 clashtui 对该目录的文件有组的读写权限。
chmod g+w /srv/mihomo/config.yaml   # clashtui 要有写的权限。

# ## 设置 mihomo service unit
systemctl enable mihomo  # 开机启动
systemctl restart mihomo  # 启动服务
```

建议先用一个可用的 mihomo 配置测试 mihomo 服务是否成功。检查是否缺少 [meta-rules-dat](https://github.com/MetaCubeX/meta-rules-dat) 文件。

## 安装 clashtui

比如: ArchLinux

```sh
# ## 安装 clashtui
# 有最新的 [PKGBUILD](./PkgManagers/PKGBUILD)。
paru -S clashtui。      # 其他 linux 发行版, 手动下载, 将 clashtui 放在 PATH 即可。

# ## 配置 clashtui
clashtui                # 先运行会在 ~/.config/clashtui 生成一些默认文件。

# nvim ~/.config/clashtui/config.toml
[default]
# 下面参数对应命令 <clash_core_path> -d <clash_cfg_dir> -f <clash_cfg_path>
clash_core_path = "mihomo"
clash_cfg_dir = "/srv/mihomo"
clash_cfg_path = "/srv/mihomo/config.yaml"
clash_srv_name = "mihomo"       # systemctl {restart | stop} <clash_srv_name>
```

如果有 cargo 则可以使用 `cargo install clashtui` 安装 clashtui。

### 配置 `basic_clash_config.yaml`

自行配置 `~/.config/clashtui/basic_clash_config.yaml`。该文件的一些基础字段会合并到 `clash_cfg_path`。可以参考[这里](./Example/basic_clash_config.yaml)配置 tun 模式。

## 便携模式

在 clashtui 程序所有的目录创建一个名为 `data` 的文件夹。则会将数据放在 `data` 内而不是 `~/.config/clashtui`。

## 使用说明

按 `?` 显示 help。

### 导入链接

-   在 Profile 区域, 按 `i` 输入 Name (尽量不使用后缀) 和 Uri (url or file path)
-   按 `a` 更新 Profile 的依赖的所有资源。默认使用自身代理更新, 如果开启 tun 模式或系统代理且没有可用节点的情况下, 先停止 mihomo 服务 (ClashSrvCtl Tab 的 StopClashService), 再更新即可。
-   按 `Enter` 选择该 Profile。
-   在浏览器输入 `http://127.0.0.1:9090/ui`。前提是你的 mihomo 配置已经配置了 ui 相关的字段, [参考](https://wiki.metacubex.one/config/general/#_7)。

### 结合 cronie 定时更新 profiles

```sh
clashtui -u         # 以命令行的模式更新所有 profiles。如果 profile 有 proxy-providers, 同时也会更新它们。
```

所以可以结合 cronie 来定时更新 profiles:

```sh
# crontab -e
@daily /usr/bin/env clashtui -u >> ~/cron.out 2>&1
# OR
@daily /usr/bin/env clashtui -u        # 不保存更新结果
```

cronie 的使用, See [ref](https://wiki.archlinuxcn.org/wiki/Cron)。

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

在[这里](./Example/templates)有最新的 templates。

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
proxy-anchor:
  - delay_test: &pa_dt {url: https://www.gstatic.com/generate_204, interval: 300}
  - proxy_provider: &pa_pp {interval: 3600, intehealth-check: {enable: true, url: https://www.gstatic.com/generate_204, interval: 300}}
```

为 `template_proxy_providers` 的每个链接生成一个 proxy-provider:

```yaml
proxy-providers:
  provider:
    tpl_param:
    type: http    # type 字段要放在此处, 不能放入 pa_pp。原因是 clashtui 根据这个字段检测是否是网络资源。
    <<: *pa_pp
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
    <<: *pa_dt
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
