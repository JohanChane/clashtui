# ClashTui

![Demo](./assets/clashtui_demo.gif)

Language: [English](./README.md) | [中文](./README_ZH.md)

<details>
<summary>Table of Contents</summary>
<!-- vim-markdown-toc GFM -->

* [Supported Platforms](#supported-platforms)
* [Target Audience](#target-audience)
* [Installing Mihomo Service (Enabling Tun Mode)](#installing-mihomo-service-enabling-tun-mode)
    * [Linux](#linux)
    * [Windows](#windows)
* [Installing clashtui](#installing-clashtui)
    * [Linux](#linux-1)
    * [Windows](#windows-1)
    * [Configuring `basic_clash_config.yaml`](#configuring-basic_clash_configyaml)
* [Starting](#starting)
    * [Windows](#windows-2)
* [Portable Mode](#portable-mode)
* [Usage](#usage)
    * [Importing Links](#importing-links)
    * [Using Configuration Templates](#using-configuration-templates)
    * [Advanced Usage](#advanced-usage)
        * [Configuring Commands to Open Files and Directories](#configuring-commands-to-open-files-and-directories)
        * [Customizing Configuration Templates](#customizing-configuration-templates)
* [Features to be added](#features-to-be-added)
* [File Structure of clashtui](#file-structure-of-clashtui)
* [Project Disclaimer](#project-disclaimer)

<!-- vim-markdown-toc -->
</details>

## Supported Platforms

- Windows
- Linux

## Target Audience

- Familiarity with clash configurations
- Preference for TUI software

## Installing Mihomo Service (Enabling Tun Mode)

### Linux

For example: [ArchLinux](https://aur.archlinux.org/packages/mihomo).

```sh
# cat /etc/pacman.d/hooks/mihomo.hook (Systems without hooks may need manual setcap or use mihomo@root service)
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
# Remove the original ExecStart
ExecStart=
ExecStart=/usr/bin/mihomo -d /srv/mihomo -f /srv/mihomo/config.yaml
# ---

mkdir /srv/mihomo
cd /srv/mihomo
chown -R mihomo:mihomo /srv/mihomo
usermod -a -G mihomo <user>
groups <user>       # Check if the user is in the mihomo group
chmod g+w /srv/mihomo               # Required for clashtui to create files.
chmod g+w /srv/mihomo/config.yaml   # Required for clashtui to have write permissions.

systemctl enable mihomo  # Enable at boot
systemctl restart mihomo  # Start the service
```

It's advisable to test the mihomo service with a functional mihomo configuration to verify its success. Check for any missing [meta-rules-dat](https://github.com/MetaCubeX/meta-rules-dat) files.

### Windows

[Install scoop](https://github.com/ScoopInstaller/Install) (optional):

```powershell
irm get.scoop.sh -outfile 'install.ps1'
.\install.ps1 -ScoopDir 'D:\Scoop' -ScoopGlobalDir 'D:\ScoopGlobal' -NoProxy    # I chose to install it in the D drive.
```

For instance:

- Use `scoop install mihomo` to install mihomo. Alternatively, download a suitable [mihomo](https://github.com/MetaCubeX/mihomo/releases) for your system and place it in `D:/PortableProgramFiles/mihomo/mihomo.exe`.
- Create directories `D:/MyAppData/mihomo` and a file `D:/MyAppData/mihomo/config.yaml`.
- Perform actions after installing clashtui.

If mihomo client (e.g., metacubexd) can access but requires a proxy for certain websites:
- For mihomo installed via Scoop: Allow `D:\Scoop\apps\mihomo\1.17.0\mihomo.exe` instead of the current path. After updating mihomo, repeat this process.
- For manually downloaded mihomo installations: Allow `D:/PortableProgramFiles/mihomo/mihomo.exe`.

## Installing clashtui

### Linux

For instance: ArchLinux

```sh
# Check for the latest [PKGBUILD](https://github.com/JohanChane/clashtui/blob/main/PKGBUILD).
paru -S clashtui        # For other Linux distributions, manually download and place clashtui in the PATH.
clashtui                # Initial run generates default files in ~/.config/clashtui.

# nvim ~/.config/clashtui/config.toml
[default]
# Parameters correspond to the command <clash_core_path> -d <clash_cfg_dir> -f <clash_cfg_path>
clash_core_path = "mihomo"
clash_cfg_dir = "/srv/mihomo"
clash_cfg_path = "/srv/mihomo/config.yaml"
clash_srv_name = "mihomo"       # systemctl {restart | stop} <clash_srv_name>
# ---
```

If you have cargo, you can use `cargo install clashtui` to install clashtui.

### Windows

Manually download and install clashtui or install via `scoop bucket add extras; scoop install clashtui`. Later, the file [clashtui.json](./Scoop/clashtui.json) will be added to the scoop extras repository. If not added to scoop extras, you can place the clashtui.json file in `D:\Scoop\buckets\extras\bucket\clashtui.json` for `scoop install clashtui` to work.

Running clashtui for the first time generates default files in `%APPDATA%/clashtui`.

Modify `%APPDATA%/clashtui/config.toml`:

```toml
[default]
# Parameters correspond to the command <clash_core_path> -d <clash_cfg_dir> -f <clash_cfg_path>
#clash_core_path = "D:/PortableProgramFiles/mihomo/mihomo.exe"
clash_core_path = "D:/Scoop/shims/mihomo.exe"       # `Get-Command mihomo`
clash_cfg_dir = "D:/MyAppData/mihomo"
clash_cfg_path = "D:/MyAppData/mihomo/config.yaml"
clash_srv_name = "mihomo"       # nssm {install | remove | restart | stop | edit} <clash_srv_name>
```

After editing, add clashtui and nssm to PATH:
- For clashtui installed via scoop: `scoop install nssm`
- For manually downloaded clashtui: Add `D:/PortableProgramFiles/clashtui` to PATH.

Run clashtui. In `ClashSrvCtl`, select `InstallSrv`. The program will install the `mihomo` core service based on the configured settings. This service starts automatically at boot. After installation, input `E` to start the core service.

### Configuring `basic_clash_config.yaml`

Manually configure `{~/.config | %APPDATA%}/clashtui/basic_clash_config.yaml`. Some basic fields in this file merge into `clash_cfg_path`. Refer to [here](./App/basic_clash_config.yaml) to configure tun mode.

## Starting

### Windows

Choose one of the following methods:

- Add `clashtui` command to PATH. Start by typing `clashtui` in the terminal, `win + r`, or in File Explorer's address bar.
- Double-click `clashtui`.

*clashtui uses [crossterm](https://docs.rs/crossterm/latest/crossterm/) and [ratatui](https://github.com/ratatui-org/ratatui) for implementation. For optimal usage on Windows, consider using [Windows Terminal](https://github.com/microsoft/terminal). Set the command startup method in Windows Terminal to use `Windows Terminal` for executing the `clashtui` command.*

## Portable Mode

Create a folder named `data` in the clashtui program directory. This stores data in `data` instead of `~/.config/clashtui` OR `%APPDATA%/clashtui`.

## Usage

Press `?` to display help.

### Importing Links

- In the Profile area, press `i` to input Name (try avoiding suffixes) and Uri.
- Press `U` to update dependencies for the Profile. It defaults to using its proxy for updates. If using tun mode or system proxy with no available nodes, stop the mihomo service (press `S`) before updating.
- Press `Enter` to select the Profile.
- Enter `http://127.0.0.1:9090/ui` in the browser.

If the Windows platform can't open `http://127.0.0.1:9090/ui`:
- In `ClashSrvCtl`, choose `TestClashConfig` to check syntax and auto-downloaded geo files.
- Press `L` to view logs. (`H` opens clashtui config dir. `G` opens clash config dir. Check if the related files are correct.)
- Use `netstat -aon | findstr "9090"` to check if the port exists. If not, consider using a compatible version of mihomo.
- If it opens but can't access proxy-required sites, allow `mihomo` through the firewall.

Supports importing file configurations. Input the file path as `Uri`.

### Using Configuration Templates

- Press `t` to switch to Templates.
- Choose `template_proxy_providers`, press `e` to edit, and input subscription links (copy without modification) as needed.

For example:

```
https://....
https://....

# Supports comments
#https://....
```

- Press `Enter` to generate configurations in `Profile`. Press `p` to switch back to Profile, and `Enter` to select the configuration.

Downloaded clashtui versions usually include templates. If not, the latest templates are available [here](https://github.com/JohanChane/clashtui/tree/main/App/templates).

### Advanced Usage

#### Configuring Commands to Open Files and Directories

Configure in `./data/config.toml`. `%s` will automatically replace with the selected file's path.

For Linux:

```toml
[default]
edit_cmd = "alacritty -e nvim %s"
opendir_cmd = "alacritty -e ranger %s"
```

For Windows:

```
[default]
edit_cmd = "notepad %s"
```

#### Customizing Configuration Templates

The template feature is unique to clashtui. Refer to provided sample templates for usage instructions.

Define repeatedly used fields:

```yaml
pp: &pp {interval: 3600, intehealth-check: {enable: true, url: https://www.gstatic.com/generate_204, interval: 300}}
delay_test: &delay_test {url: https://www.gstatic.com/generate_204, interval: 300}
```

Generate a `proxy-provider` for each link in `template_proxy_providers`:

```yaml
proxy-providers:
  provider:
    tpl_param:
    type: http    # The type field should be placed here, not in pp, as it's used for updating resources.
    <<: *pp
```

Generate a `Select, Auto` `proxy-group` for each proxy-provider:

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

Use `Select, Auto` proxy-groups:

```yaml
proxy-groups:
  - name: "Entry"
    type: select
    proxies:
      - <Auto>
      - <Select>
```

## Features to be added

See [here](./FeaturesToBeAdded.md)

## File Structure of clashtui

- `basic_clash_config.yaml`: Contains basic fields for mihomo configuration, which will be merged into `clash_cfg_path`.
- `config.yaml`: Configuration file for clashtui.

## Project Disclaimer

This project is for educational and reference purposes only. The author doesn't guarantee the accuracy, completeness, or applicability of the code in this project. Users should use the code at their own risk.

The author is not responsible for any direct or indirect losses caused by the use of this project's code, including but not limited to data loss, computer damage, or business interruption.

Before using the code in this project, users should fully understand its functionality and potential risks. Seek professional advice if necessary. The author bears no responsibility for any consequences resulting from the use of this project's code.

When using the code in this project, abide by relevant laws and regulations. Do not use it for illegal activities or actions that infringe upon others' rights.

The author reserves the right to interpret this disclaimer and may modify and update it at any time.
