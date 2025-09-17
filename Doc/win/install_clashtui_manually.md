# Install ClashTUI Manually

## Install the mihomo Program

[Install scoop](https://github.com/ScoopInstaller/Install) (Optional):

```powershell
irm get.scoop.sh -outfile 'install.ps1'
.\install.ps1 -ScoopDir 'D:\Scoop' -ScoopGlobalDir 'D:\ScoopGlobal' -NoProxy    # I chose to install it on the D drive.
```

Install mihomo via scoop:

```powershell
scoop install main/mihomo
```

Alternatively, manually download the version of mihomo suitable for your system. See [mihomo GitHub releases](https://github.com/MetaCubeX/mihomo/releases).

## Check if mihomo Can Run

Create the necessary files for mihomo to run:

```powershell
New-Item -ItemType Directory -Path "D:\ClashTUI\mihomo_config"            # Avoid spaces in the path
New-Item -ItemType File -Path "D:\ClashTUI\mihomo_config\config.yaml"     # Add your mihomo config
```

Run mihomo:

```powershell
<Path to the mihomo program> -d D:\ClashTUI\mihomo_config -f D:\ClashTUI\mihomo_config\config.yaml
```

Potential issues:
1.  If you can access the mihomo client (e.g., metacubexd) but cannot access websites that require a proxy, try allowing `mihomo.exe` through the firewall:
    -   If mihomo was installed via Scoop: Allow `D:\Scoop\apps\mihomo\<version>\mihomo.exe`, not the one in the current path. After upgrading mihomo, you may need to repeat this step.
    -   If mihomo was installed manually: Allow <Path to the mihomo program>.
2.  Slow download of geo files by mihomo:

    ```powershell
    Invoke-WebRequest -Uri "https://github.com/MetaCubeX/meta-rules-dat/releases/download/latest/geoip.metadb" -OutFile "D:\ClashTUI\mihomo_config\geoip.metadb"
    Invoke-WebRequest -Uri "https://github.com/MetaCubeX/meta-rules-dat/releases/download/latest/geosite.dat" -OutFile "D:\ClashTUI\mihomo_config\GeoSite.dat"
    ```

## Install ClashTUI

Install ClashTUI via scoop:

```powershell
scoop bucket add extras
scoop install clashtui
```

Alternatively, download it manually. [ClashTUI GitHub releases](https://github.com/JohanChane/clashtui/releases)

## Run ClashTUI

First, run ClashTUI. It will generate some default files in `%APPDATA%/clashtui`. Then modify `%APPDATA%/clashtui/config.yaml`. For configuration reference, see [ref](./clashtui_usage.md).

```yaml
# The parameters below correspond to the command <clash_core_path> -d <clash_cfg_dir> -f <clash_cfg_path>
clash_core_path: "D:/ClashTUI/mihomo.exe"
clash_cfg_dir: "D:/ClashTUI/mihomo_config"
clash_cfg_path: "D:/ClashTUI/mihomo_config/config.yaml"
clash_srv_name: "clashtui_mihomo"                           # nssm {install | remove | restart | stop | edit} <clash_srv_name>
```

1.  Install [nssm](https://nssm.cc/download):
    -   Download and rename it to nssm.
    -   Add the command to PATH.

    If you have scoop, you can install it directly:

    ```powershell
    scoop install nssm
    ```

2.  Install [Loopback Manager](https://github.com/tiagonmas/Windows-Loopback-Exemption-Manager) (Optional):
    -   Download and rename it to EnableLoopback.exe.
    -   Add the command to PATH.

3.  Install and start the clashtui_mihomo service via ClashTUI:
    -   Run ClashTUI. In the `ClashSrvCtl` tab, select `InstallSrv`. The program will install the `clashtui_mihomo` kernel service based on the above configuration.
    -   The service will start on boot. After installation, start the kernel service using the `StartClashService` option in the ClashSrvCtl tab to launch the mihomo service.

## Download Templates

See [ref](https://github.com/JohanChane/clashtui/blob/main/Doc/install_clashtui_manually.md#%E4%B8%8B%E8%BD%BD%E6%A8%A1%E6%9D%BF-%E5%8F%AF%E9%80%89)
