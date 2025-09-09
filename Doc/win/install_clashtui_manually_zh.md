# Install ClashTUI Manually

## 安装 Mihomo 服务 (启用 Tun 模式)

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

手动下载安装 clashtui, 或者通过 `scoop bucket add extras; scoop install clashtui` 安装。这里有最新的 [clashtui.json](./PkgManagers/Scoop/clashtui.json)。

先运行 clashtui, 会在 `%APPDATA%/clashtui` 生成一些默认文件。

修改 `%APPDATA%/clashtui/config.yaml`:

```yaml
# 下面参数对应命令 <clash_core_path> -d <clash_cfg_dir> -f <clash_cfg_path>
#clash_core_path: "D:/PortableProgramFiles/mihomo/mihomo.exe"
clash_core_path: "D:/Scoop/shims/mihomo.exe"       # `Get-Command mihomo`
clash_cfg_dir: "D:/MyAppData/mihomo"
clash_cfg_path: "D:/MyAppData/mihomo/config.yaml"
clash_srv_name: "mihomo"       # nssm {install | remove | restart | stop | edit} <clash_srv_name>
```

改好之后, 将 clashtui, nssm 加入 PATH:
-   scoop 安装 clashtui 的: scoop install nssm
-   手动下载安装 clashtui 的: 将 `D:/PortableProgramFiles/clashtui` 加入 PATH。

运行 clashtui。在 `ClashSrvCtl` Tab 选择 `InstallSrv`, 程序会根据上面的配置安装 `mihomo` 内核服务。该服务会开机启动。安装之后启动内核服务, 使用 ClashSrvCtl Tab 的 StartClashService 启动 mihomo 服务。

如果不使用 scoop 安装 nssm, 可以手动下载 [nssm](https://nssm.cc/download), 将其改名为 `nssm.exe`, 并将其加入 PATH 或者放在 clashtui 所在的目录即可。

Loopback Manager 同理。下载 [Loopback Manager](https://github.com/tiagonmas/Windows-Loopback-Exemption-Manager), 将其改名为 `EnableLoopback.exe`, 然后将其加入 PATH 或者放在 clashtui 所在的目录下即可。
