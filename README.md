# ClashTui

**This demo is OUTDATED**
![Demo](./assets/clashtui_demo.gif)

Language: [English](./README.md) | [中文](./README_ZH.md)

<details>
<summary>Table of Contents</summary>
<!-- vim-markdown-toc GFM -->

* [Install Mihomo Service (Enable Tun Mode)](#install-mihomo-service-enable-tun-mode)
* [Install clashtui](#install-clashtui)
    * [Configure `basic_clash_config.yaml`](#configure-basic_clash_configyaml)
* [Launching](#launching)
    * [Windows](#windows)
* [Portable Mode](#portable-mode)
* [Usage Instructions](#usage-instructions)
    * [Importing Links](#importing-links)
    * [Automated Profile Updates with Windows Task Scheduler](#automated-profile-updates-with-windows-task-scheduler)
    * [Using Configuration Templates](#using-configuration-templates)
    * [Advanced Usage](#advanced-usage)
        * [Configuring Commands to Open Files and Directories](#configuring-commands-to-open-files-and-directories)
        * [Customizing Configuration Templates](#customizing-configuration-templates)
* [File Structure of clashtui](#file-structure-of-clashtui)
* [Disclaimer](#disclaimer)

<!-- vim-markdown-toc -->
</details>

## Install Mihomo Service (Enable Tun Mode)

[Install scoop](https://github.com/ScoopInstaller/Install) (Optional):

```powershell
irm get.scoop.sh -outfile 'install.ps1'
.\install.ps1 -ScoopDir 'D:\Scoop' -ScoopGlobalDir 'D:\ScoopGlobal' -NoProxy    # I chose to install it in the D drive.
```

For example:

-   Install mihomo via `scoop install mihomo`. Alternatively, download a suitable [mihomo release](https://github.com/MetaCubeX/mihomo/releases) for your system and place it at `D:/PortableProgramFiles/mihomo/mihomo.exe`.
-   Create the directory `D:/MyAppData/mihomo` and the file `D:/MyAppData/mihomo/config.yaml`.
-   Install clashtui after these steps.

If you can access the mihomo client (e.g., metacubexd) but cannot access websites that require a proxy, try allowing `mihomo.exe` through the firewall:
-   For mihomo installed via Scoop: Allow `D:\Scoop\apps\mihomo\1.17.0\mihomo.exe` instead of the current path. After updating mihomo to a newer version, you may need to perform this operation again.
-   For manually downloaded mihomo installations: Allow `D:/PortableProgramFiles/mihomo/mihomo.exe`.

## Install clashtui

Manually download and install clashtui, or install it via `scoop bucket add extras; scoop install clashtui`. You can find the latest [clashtui.json](./PkgManagers/Scoop/clashtui.json) here.

Run clashtui first, which will generate some default files in `%APPDATA%/clashtui`.

Modify `%APPDATA%/clashtui/config.yaml`:

```yaml
# The following parameters correspond to the command <clash_core_path> -d <clash_cfg_dir> -f <clash_cfg_path>
#clash_core_path: "D:/PortableProgramFiles/mihomo/mihomo.exe"
clash_core_path: "D:/Scoop/shims/mihomo.exe"       # `Get-Command mihomo`
clash_cfg_dir: "D:/MyAppData/mihomo"
clash_cfg_path: "D:/MyAppData/mihomo/config.yaml"
clash_srv_name: "mihomo"       # nssm {install | remove | restart | stop | edit} <clash_srv_name>
```

Once modified, add clashtui and nssm to PATH:
-   For clashtui installed via scoop: `scoop install nssm`
-   For manually downloaded clashtui installations: Add `D:/PortableProgramFiles/clashtui` to PATH.

Run clashtui. In the `ClashSrvCtl` tab, select `InstallSrv`. The program will install the `mihomo` core service according to the configuration above. This service will start automatically on boot. After installation, start the core service by using the `StartClashService` button in the ClashSrvCtl tab.

If you're not using scoop to install nssm, you can manually download [nssm](https://nssm.cc/download), rename it to `nssm.exe`, and add it to PATH or place it in the same directory as clashtui.

The same applies to Loopback Manager. Download [Loopback Manager](https://github.com/tiagonmas/Windows-Loopback-Exemption-Manager), rename it to `EnableLoopback.exe`, and add it to PATH or place it in the directory where clashtui is located.

### Configure `basic_clash_config.yaml`

Configure `%APPDATA%/clashtui/basic_clash_config.yaml` manually. Some basic fields in this file will be merged into `clash_cfg_path`. Refer to [here](./Example/basic_clash_config.yaml) for configuring tun mode.

## Launching

### Windows

Choose one of the following methods:

-   Add the clashtui command to PATH. Launch it via terminal, `win + r`, or by entering `clashtui` in the address bar of File Explorer.
-   Double-click on clashtui.

*clashtui uses [crossterm](https://docs.rs/crossterm/latest/crossterm/) and [ratatui](https://github.com/ratatui-org/ratatui) for implementation. It's recommended to use [Windows Terminal](https://github.com/microsoft/terminal) on Windows. Set the startup command to use `Windows Terminal` in Windows Terminal settings. Then, executing the clashtui command will automatically use Windows Terminal.*

## Portable Mode

Create a folder named `data` in the directory where clashtui is located. This will store data in the `data` folder instead of `%APPDATA%/clashtui`.

## Usage Instructions

Press `?` to display help.

### Importing Links

-   In the Profile area, press `i` to input Name (try to avoid suffixes) and Uri (URL or file path).
-   Press `a` to update all Profile dependencies. By default, it uses its own proxy for updates. If tun mode or system proxy is enabled and there are no available nodes, stop the mihomo service first (StopClashService in the ClashSrvCtl tab), then update.
-   Press `Enter` to select the Profile.
-   Enter `http://127.0.0.1:9090/ui` in your browser. Assuming your mihomo configuration has already set up the UI-related fields, [refer here](https://wiki.metacubex.one/config/general/#_7).

If the Windows platform fails to open `http://127.0.0.1:9090/ui`:
-   In `ClashSrvCtl`, select `TestClashConfig` to check syntax correctness and automatic downloading of geo files.
-   Press `L` to view logs. (`H` to open the clashtui config dir. `G` to open the clash config dir. Check if the related files are correct.)
-   You can use `netstat -aon | findstr "9090"` to check if the port exists. If it doesn't, you may need to use a compatible version of mihomo.
-   If you can open it but cannot access websites that require a proxy, allow `mihomo` through the firewall.

### Automated Profile Updates with Windows Task Scheduler

```powershell
clashtui -u         # Update all profiles in command-line mode. If profiles have proxy-providers, they will also be updated.
```

### Using Configuration Templates

-   Press `t` to switch to Templates area.
-   Select `template_proxy_providers`, press `e` to edit, and enter subscription links (copy the link directly without modification).

    For example:

    ```
    https://....
    https://....

    # Support comments
    #https://....
    ```

-   Press `Enter` to generate the configuration to `Profile`. Press `p` to switch back to Profile, then `Enter` to select the configuration.

The latest templates are available [here](./Example/templates).

### Advanced Usage

#### Configuring Commands to Open Files and Directories

Configure in `%APPDATA%/clashtui/config.yaml`. `%s` will be automatically replaced with the selected file path.

For example:

```yaml
edit_cmd = "notepad %s"
#opendir_cmd: "explorer %s"
```

#### Customizing Configuration Templates

The template function is unique to clashtui. Refer to the provided example templates for specific usage rules.

Define fields to be reused:

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
    type: http    # The type field must be placed here, not in pa_pp. clashtui detects network resources based on this field.
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

## File Structure of clashtui

-   basic_clash_config.yaml: Basic fields for mihomo configuration, merged into `clash_cfg_path`.
-   config.yaml: Configuration file for clashtui.

## Disclaimer

This project is for learning and reference purposes only. The author does not guarantee the accuracy, completeness, or applicability of the code in this project. Users should use the code in this project at their own risk.

The author is not responsible for any direct or indirect losses caused by the use of the code in this project, including but not limited to data loss, computer damage, or business interruption.

Before using the code in this project, users should fully understand its functionality and potential risks, and seek professional advice if necessary. The author assumes no responsibility for any consequences resulting from the use of the code in this project.

When using the code in this project, users must comply with relevant laws and regulations and refrain from using it for illegal activities or activities that infringe upon the rights of others.

The author reserves the right to interpret and modify this disclaimer at any time.
