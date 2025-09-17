# Install ClashTUI Manually

## 安装 mihomo 程序

[安装 scoop](https://github.com/ScoopInstaller/Install) (可选):

```powershell
irm get.scoop.sh -outfile 'install.ps1'
.\install.ps1 -ScoopDir 'D:\Scoop' -ScoopGlobalDir 'D:\ScoopGlobal' -NoProxy    # 我选择安装在 D 盘。
```

通过 scoop 安装 mihomo:

```powershell
scoop install main/mihomo
```

也可以手动下载适合自己系统的 mihomo。See [mihomo github releases](https://github.com/MetaCubeX/mihomo/releases)。

## 检测 mihomo 是否能运行

创建 mihomo 运行需要的文件:

```powershell
New-Item -ItemType Directory -Path "D:\ClashTUI\mihomo_config"            # 路径不要有空格
New-Item -ItemType File -Path "D:\ClashTUI\mihomo_config\config.yaml"     # 添加你的 mihomo 配置
```

运行 mihomo:

```powershell
<mihomo 程序的路径> -d D:\ClashTUI\mihomo_config -f D:\ClashTUI\mihomo_config\config.yaml
```

可能出现的问题:
1.  如果可以访问 mihomo 客户端 (比如: metacubexd) 而无法访问需要代理的网站, 则尝试允许 `mihomo.exe` 通过防火墙:
    -   如果通过 Scoop 安装 mihomo 的: 允许 `D:\Scoop\apps\mihomo\<version>\mihomo.exe`, 而不是 current 路径的。之后 mihomo 升级版本之后, 可能还要继续这样的操作。
    -   如果手动下载 mihomo 安装的: 允许 <mihomo 程序的路径>
2.  mihomo 下载 geo 文件比较慢:

    ```powershell
    Invoke-WebRequest -Uri "https://github.com/MetaCubeX/meta-rules-dat/releases/download/latest/geoip.metadb" -OutFile "D:\ClashTUI\mihomo_config\geoip.metadb"
    Invoke-WebRequest -Uri "https://github.com/MetaCubeX/meta-rules-dat/releases/download/latest/geosite.dat" -OutFile "D:\ClashTUI\mihomo_config\GeoSite.dat"
    ```

## 安装 clashtui

通过 scoop 安装 clashtui:

```powershell
scoop bucket add extras
scoop install clashtui
```

也可以手动下载。[clashtui github releases](https://github.com/JohanChane/clashtui/releases)

## 运行 clashtui

先运行 clashtui, 会在 `%APPDATA%/clashtui` 生成一些默认文件。然后修改 `%APPDATA%/clashtui/config.yaml`。配置参考 [ref](./clashtui_usage_zh.md)

```yaml
# 下面参数对应命令 <clash_core_path> -d <clash_cfg_dir> -f <clash_cfg_path>
clash_core_path: "D:/ClashTUI/mihomo.exe"
clash_cfg_dir: "D:/ClashTUI/mihomo_config"
clash_cfg_path: "D:/ClashTUI/mihomo_config/config.yaml"
clash_srv_name: "clashtui_mihomo"                           # nssm {install | remove | restart | stop | edit} <clash_srv_name>
```

1.  安装 [nssm](https://nssm.cc/download):
    -   下载并改名为 nssm。
    -   将命令加入 PATH

    如果有 scoop, 则可以直接安装:

    ```powershell
    scoop install nssm
    ```

2.  安装 [Loopback Manager](https://github.com/tiagonmas/Windows-Loopback-Exemption-Manager) (可选):
    -   下载并改名为 EnableLoopback.exe
    -   将命令加入 PATH

3.  通过 clashtui 安装和启动 clashtui_mihomo 服务:
    -   运行 clashtui。在 `ClashSrvCtl` Tab 选择 `InstallSrv`, 程序会根据上面的配置安装 `clashtui_mihomo` 内核服务。
    -   该服务会开机启动。安装之后启动内核服务, 使用 ClashSrvCtl Tab 的 `StartClashService` 启动 mihomo 服务。

## 下载模板

See [ref](https://github.com/JohanChane/clashtui/blob/main/Doc/install_clashtui_manually_zh.md#%E4%B8%8B%E8%BD%BD%E6%A8%A1%E6%9D%BF-%E5%8F%AF%E9%80%89)
