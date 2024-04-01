# ClashTui

**This demo is OUTDATED**
![Demo](./assets/clashtui_demo.gif)

Language: [English](./README.md) | [中文](./README_ZH.md)

<details>
<summary>Table of Contents</summary>
<!-- vim-markdown-toc GFM -->

* [Supported Platforms](#supported-platforms)
* [Target Audience](#target-audience)
* [Install Mihomo Service (Enable Tun Mode)](#install-mihomo-service-enable-tun-mode)
* [Install ClashTui](#install-clashtui)
    * [Configure `basic_clash_config.yaml`](#configure-basic_clash_configyaml)
* [Portable Mode](#portable-mode)
* [Usage Instructions](#usage-instructions)
    * [Importing Links](#importing-links)
    * [Scheduled Updates with cronie](#scheduled-updates-with-cronie)
    * [Using Configuration Templates](#using-configuration-templates)
    * [Advanced Usage](#advanced-usage)
        * [Configuring Open File and Open Directory Commands](#configuring-open-file-and-open-directory-commands)
        * [Customizing Configuration Templates](#customizing-configuration-templates)
* [ClashTui File Structure](#clashtui-file-structure)
* [Project Disclaimer](#project-disclaimer)

<!-- vim-markdown-toc -->
</details>

## Supported Platforms

- Linux
- Windows. Please refer to [Windows README](https://github.com/JohanChane/clashtui/blob/win/README.md)

## Target Audience

- Those with a certain understanding of Clash configurations.
- Fans of TUI software.

## Install Mihomo Service (Enable Tun Mode)

For example: [ArchLinux](https://aur.archlinux.org/packages/mihomo).

```sh
# ## Install mihomo
paru -S mihomo

# ## Add mihomo hook
# cat /etc/pacman.d/hooks/mihomo.hook (If there is no system similar to hook, use ClashSrvCtl Tab's SetPermission or use mihomo@root service)
[Trigger]
Operation = Install
Operation = Upgrade
Type = Path
Target = usr/bin/mihomo

[Action]
When = PostTransaction
Exec = /usr/bin/setcap 'cap_net_admin,cap_net_bind_service=+ep' /usr/bin/mihomo

# ## Edit mihomo service unit
# systemctl edit mihomo
[Service]
# Remove original ExecStart
ExecStart=
ExecStart=/usr/bin/mihomo -d /srv/mihomo -f /srv/mihomo/config.yaml

# ## Create /srv/mihomo
mkdir /srv/mihomo
cd /srv/mihomo
chown -R mihomo:mihomo /srv/mihomo
usermod -a -G mihomo <user>
groups <user>       # Check if already added to mihomo group

# Optional. After version 0.2.0, clashtui will automatically fix file permissions.
chmod g+w /srv/mihomo               # clashtui needs permission to create files.
chmod g+s /srv/mihomo               # Make the group of files created by clashtui mihomo. This is to give clashtui group read and write permissions to files in this directory.
chmod g+w /srv/mihomo/config.yaml   # clashtui needs write permission.

# ## Set mihomo service unit
systemctl enable mihomo  # Start on boot
systemctl restart mihomo  # Start service
```

It is recommended to test the mihomo service with a valid configuration to ensure its success. Check if [meta-rules-dat](https://github.com/MetaCubeX/meta-rules-dat) file is missing.

## Install ClashTui

For example: ArchLinux

```sh
# ## Install clashtui
# There is a latest [PKGBUILD](./PkgManagers/PKGBUILD).
paru -S clashtui.      # For other Linux distributions, manually download and place clashtui in PATH.

# ## Configure clashtui
clashtui                # Running this will generate some default files in ~/.config/clashtui.

# nvim ~/.config/clashtui/config.yaml
# The following parameters correspond to the command <clash_core_path> -d <clash_cfg_dir> -f <clash_cfg_path>
clash_core_path: "mihomo"
clash_cfg_dir: "/srv/mihomo"
clash_cfg_path: "/srv/mihomo/config.yaml"
clash_srv_name: "mihomo"       # systemctl {restart | stop} <clash_srv_name>
```

If you have cargo, you can use `cargo install clashtui` to install clashtui.

### Configure `basic_clash_config.yaml`

Configure `~/.config/clashtui/basic_clash_config.yaml` manually. Some basic fields in this file will be merged into `clash_cfg_path`. Refer to [here](./Example/basic_clash_config.yaml) for configuring tun mode.

## Portable Mode

Create a folder named `data` in the directory where clashtui program resides. Then the data will be placed in `data` instead of `~/.config/clashtui`.

## Usage Instructions

Press `?` to display help.

### Importing Links

- In the `Profile` area, press `i` to input Name (preferably without suffix) and Uri (url or file path).
- If it's the first time installing clashtui:
  - Press Enter to select an available profile so that the fields of `basic_clash_config` are merged into `clash_cfg_path`.
  - Restart clashtui to reparse changes such as `basic_clash_config`.
  - Restart the mihomo service (StartClashService in the ClashSrvCtl tab).
- Press `a` to update all resources dependent on the Profile. By default, it uses its own proxy for updates. If tun mode or system proxy is enabled and there are no available nodes, stop the mihomo service first (ClashSrvCtl Tab's StopClashService), then update.
- Press `Enter` to select the Profile.
- Enter `http://127.0.0.1:9090/ui` in the browser. Provided your mihomo configuration has already set up ui related fields, [reference](https://wiki.metacubex.one/config/general/#_7).

### Scheduled Updates with cronie

```sh
clashtui -u         # Updates all profiles in command-line mode. If the profile has proxy-providers, they will also be updated.
```

Thus, you can use cronie to schedule updates:

```sh
# crontab -e
@daily /usr/bin/env clashtui -u >> ~/cron.out 2>&1
# OR
@daily /usr/bin/env clashtui -u        # Do not save update results
```

For cronie usage, see [ref](https://wiki.archlinuxcn.org/wiki/Cron).

### Using Configuration Templates

- Press `t` to switch to Templates area.
- Select `template_proxy_providers`, press `e` to edit, and enter subscription links.

    For example:

    ```
    https://....
    https://....

    # Supports comments
    #https://....
    ```

- Press `Enter` to generate configuration to `Profile`. Press `p` to switch back to `Profile`, then `Enter` to select the configuration.

The latest templates can be found [here](./Example/templates).

### Advanced Usage

#### Configuring Open File and Open Directory Commands

Configure in `~/.config/clashtui/config.yaml`. `%s` will be automatically replaced with the path of the selected file.

For example:

```yaml
edit_cmd: "alacritty -e nvim %s"
opendir_cmd: "alacritty -e ranger %s"
```

#### Customizing Configuration Templates

The template feature is unique to clashtui. Refer to provided example templates for specific usage rules.

Define reusable fields:

```yaml
proxy-anchor:
  - delay_test: &pa_dt {url: https://www.gstatic.com/generate_204, interval: 300}
  - proxy_provider: &pa_pp {interval: 3600, intehealth-check: {enable: true, url: https://www.gstatic.com/generate_204, interval: 300}}
```

Generate a proxy-provider for each link in `template_proxy_providers`:

```yaml
proxy-providers:
  provider:
    tpl_param:
    type: http    # type field must be placed here, not within pa_pp. This is because clashtui detects if it is a network resource based on this field.
    <<: *pa_pp
```

Generate a `Select, Auto` proxy-group for each proxy-providers:

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

Use `Select, Auto` proxy-groups:

```yaml
proxy-groups:
  - name: "Entry"
    type: select
    proxies:
      - <Auto>
      - <Select>
```

## ClashTui File Structure

- basic_clash_config.yaml: Basic fields of mihomo configuration, which will be merged into `clash_cfg_path`.
- config.yaml: Configuration of clashtui.

## Project Disclaimer

This project is for learning and reference purposes only. The author does not guarantee the accuracy, completeness, or applicability of the code in the project. Users should use the code in this project at their own risk.

The author is not responsible for any direct or indirect losses caused by the use of the code in this project, including but not limited to data loss, computer damage, and business interruption.

Before using the code in this project, users should fully understand its functionality and potential risks, and seek professional advice if necessary. The author is not liable for any consequences resulting from the use of the code in this project.

When using the code in this project, please comply with relevant laws and regulations, and refrain from using it for illegal activities or activities that infringe upon the rights of others.

The author reserves the right of final interpretation of this disclaimer, and may modify and update it at any time.
