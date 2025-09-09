# Install ClashTUI Manually

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
