# ClashTui

![Demo](./Doc/Assets/clashtui_demo.gif)

Language: [English](./README.md) | [中文](./Doc/README_ZH.md)

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
* [See more](#see-more)
* [Project Disclaimer](#project-disclaimer)

<!-- vim-markdown-toc -->
</details>

## Supported Platforms

- Linux(amd64, arm64)
- Windows

## Target Audience

- Those with a certain understanding of Clash configurations.
- Fans of TUI software.

## Installing Mihomo Service (With Tun Mode Avaliable)

 > It's advised to test the mihomo service with a functional mihomo configuration to verify its success. 
 >
 > Check for any missing [meta-rules-dat](https://github.com/MetaCubeX/meta-rules-dat) files.

### Linux

For example: [ArchLinux](https://aur.archlinux.org/packages/mihomo).

#### Set Up Trigger:
 > This will `setcap` for `mihomo` automatically every time you update via `pacman`
 - `vi /etc/pacman.d/hooks/mihomo.hook` 
```systemd
[Trigger]
Operation = Install
Operation = Upgrade
Type = Path
Target = usr/bin/mihomo

[Action]
When = PostTransaction
Exec = /usr/bin/setcap 'cap_net_admin,cap_net_bind_service=+ep' /usr/bin/mihomo
```
 - And then `paru -S mihomo`
#### Set Up Systemd User Service: 
 - Create service: `systemctl --user edit mihomo`
```systemd
[Unit]
Description=mihomo Daemon, Another Clash Kernel.
After=network.target NetworkManager.service systemd-networkd.service iwd.service

[Service]
Type=simple
LimitNPROC=4096
LimitNOFILE=1000000
# %h mean HOME
WorkingDirectory=%h/.local/proxy
Restart=always
ExecStartPre=/usr/bin/sleep 1s
ExecStart=%h/.local/proxy/mihomo -d %h/.local/proxy/config
ExecReload=/bin/kill -HUP $MAINPID
 
[Install]
WantedBy=default.target
```
 - To enable at boot: `systemctl --user enable mihomo`
 - To start the service: `systemctl --user start mihomo`

#### Set Up Systemd System Service
 > For those want to use system-wide service, please refer to [this](https://wiki.metacubex.one/startup/service/#systemd) 
  - Create /srv/mihomo
  ```bash
mkdir /srv/mihomo
cd /srv/mihomo
chown -R mihomo:mihomo /srv/mihomo
usermod -a -G mihomo <user>
groups <user>       # Check if already added to mihomo group

chmod g+w /srv/mihomo               # clashtui needs permission to create files.
chmod g+s /srv/mihomo               # Make the group of files created by clashtui mihomo. This is to give clashtui group read and write permissions to files in this directory.
chmod g+w /srv/mihomo/config.yaml   # clashtui needs write permission.
```
 - Create service: `systemctl --user edit mihomo`
```systemd
[Unit]
Description=Mihomo daemon
After=network.target NetworkManager.service systemd-networkd.service iwd.service

[Service]
Type=simple
User=mihomo
Group=mihomo
LimitNPROC=500
LimitNOFILE=1000000
CapabilityBoundingSet=CAP_NET_ADMIN CAP_NET_RAW CAP_NET_BIND_SERVICE
AmbientCapabilities=CAP_NET_ADMIN CAP_NET_RAW CAP_NET_BIND_SERVICE
Restart=always
RestartSec=5
ExecStart=/usr/bin/mihomo -d /srv/mihomo

[Install]
WantedBy=multi-user.target
```

### Windows

[Install scoop](https://github.com/ScoopInstaller/Install) (optional):

```powershell
irm get.scoop.sh -outfile 'install.ps1'
.\install.ps1 -ScoopDir 'D:\Scoop' -ScoopGlobalDir 'D:\ScoopGlobal' -NoProxy    
# I chose to install it in the D drive.
```

For instance:

- Use `scoop install mihomo` to install mihomo. Alternatively, download a suitable [mihomo](https://github.com/MetaCubeX/mihomo/releases) for your system and place it in `D:/PortableProgramFiles/mihomo/mihomo.exe`.
- Create directories `D:/MyAppData/mihomo` and a file `D:/MyAppData/mihomo/config.yaml`.
- Perform actions after installing clashtui.

If mihomo client (e.g. metacubexd) can access but requires a proxy for certain websites:
- For mihomo installed via Scoop: Allow `D:\Scoop\apps\mihomo\1.17.0\mihomo.exe` instead of the current path. After updating mihomo, repeat this process.
- For manually downloaded mihomo installations: Allow `D:/PortableProgramFiles/mihomo/mihomo.exe`.

## Install ClashTui

### Linux

For example: ArchLinux `paru -S clashtui.`      
> For other Linux distributions, manually download and place clashtui in PATH.

Clashtui's config file:
```yaml
basic:
# You can find the path by `Get-Command mihomo`
  clash_bin_path: /usr/bin/mihomo
  clash_config_dir: /srv/mihomo
  clash_config_path: /srv/mihomo/config.yaml
  timeout: null # default to be 5, '0' DOES NOT mean no timeout
service: 
  clash_srv_nam: mihomo 
  is_user: false
extra:
  edit_cmd: ''
  open_dir_cmd: ''
```

The subsequent versions of clashtui have not been uploaded to `crates.io` because clashtui is now separated into multiple modules. If uploaded to `crates.io`, it would require uploading each dependent module, and some modules do not need to be uploaded to `crates.io`. See [ref](https://users.rust-lang.org/t/is-it-possible-to-publish-crates-with-path-specified/91497/2). So, do not use `cargo install clashtui` for installation.

### Windows

 - Manually [download](https://github.com/Jackhr-arch/clashtui/releases) and install clashtui
 - Install via Scoop 
 `scoop bucket add extras; scoop install clashtui; scoop install nssm`.

Please modify `%APPDATA%/clashtui/config.yaml`:

```yaml
basic:
# You can find the path by `Get-Command mihomo`
  clash_bin_path: D:/Scoop/shims/mihomo.exe
  clash_config_dir: D:/MyAppData/mihomo
  clash_config_path: D:/MyAppData/mihomo/config.yaml
  timeout: null
service: # is_user is not used on Windows
  clash_srv_nam: mihomo 
extra:
  edit_cmd: ''
  open_dir_cmd: ''
```

 > After editing, add `nssm` and `clashtui`(Optional) to PATH

Run clashtui. In `ClashSrvCtl`, select `InstallSrv`. The program will install the `mihomo` core service based on the configured settings. This service starts automatically at boot. After that, press `R` to start the core service without rebooting.

### Configure `basic_clash_config.yaml`

Configure `~/.config/clashtui/basic_clash_config.yaml` manually. Some basic fields in this file will be merged into `clash_cfg_path`. Refer to [here](./Example/basic_clash_config.yaml) for configuring tun mode.

## Portable Mode

Create a folder named `data` in the directory where clashtui program resides. Then the data will be placed in `data` instead of `~/.config/clashtui`.

## Usage Instructions

Press `?` to display help.

### Importing Links

- Import profile: In the `Profile` area, press `i` to input Name (preferably without suffix) and Uri (url or file path).
- Update profile: Press `a` to update all resources dependent on the Profile. By default, it uses its own proxy for updates. If tun mode or system proxy is enabled and there are no available nodes, stop the mihomo service first (ClashSrvCtl Tab's StopClashService), then update.
- Select profile: Press `Enter` to select the Profile.
- Open mihomo ui: Enter `http://127.0.0.1:9090/ui` in the browser. Provided your mihomo configuration has already set up ui related fields, [reference](https://wiki.metacubex.one/config/general/#_7).

If it is the first time installing clashtui:
- If you have changed the `basic_clash_config` or other configurations, restart clashtui to reparse the changes in `basic_clash_config`, etc.
- Import a profile that does not require updating with a proxy.
- Press `a` to update all resources dependent on the profile.
- Press `Enter` to select this profile, merging the fields of `basic_clash_config` into `clash_cfg_path`.
- Restart the mihomo service (StartClashService in ClashSrvCtl Tab).

### Scheduled Updates with cronie

> Here is also `systemd` solution under [`/Doc/systemd`](/Doc/systemd/README.md)

```sh
clashtui profile update -a         # Updates all profiles in command-line mode. If the profile has proxy-providers, they will also be updated.
```

Thus, you can use cronie to schedule updates:

```sh
# crontab -e
0 10,14,16,22 * * * /usr/bin/env clashtui profile update -a >> ~/cron.out 2>&1
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

### For Command line user
clashtui now also provide command line interface, simply type `clashtui -h` to find out
> shell auto-compeltion can be generated by clashtui itself. use `clashtui --generate-shell-completion --shell [zsh/bash/powershell]` to do so.

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

## See more

[Doc](./Doc)

## Project Disclaimer

This project is for learning and reference purposes only. The author does not guarantee the accuracy, completeness, or applicability of the code in the project. Users should use the code in this project at their own risk.

The author is not responsible for any direct or indirect losses caused by the use of the code in this project, including but not limited to data loss, computer damage, and business interruption.

Before using the code in this project, users should fully understand its functionality and potential risks, and seek professional advice if necessary. The author is not liable for any consequences resulting from the use of the code in this project.

When using the code in this project, please comply with relevant laws and regulations, and refrain from using it for illegal activities or activities that infringe upon the rights of others.

The author reserves the right of final interpretation of this disclaimer, and may modify and update it at any time.
