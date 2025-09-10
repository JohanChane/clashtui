# Install ClashTUI Manually

## 安装 ClashTUI

比如: ArchLinux

1. 安装 mihomo, clashtui

```sh
sudo pacman -S mihomo clashtui
```

clashtui 后续的版本没有上传到 `crates.io`, 因为现在 clashtui 分离为多个模块,
如果上传到 `crates.io`, 需要上传依赖的每个模块, 而有些模块没有必要上传到 `crates.io`。
See [ref](https://users.rust-lang.org/t/is-it-possible-to-publish-crates-with-path-specified/91497/2)。
所以不要使用 `cargo install clashtui` 来安装了。

2.  创建 mihomo user 和 mihomo group, 同时加入该组

*有可能安装 mihomo 的时候已经创建了。*

```sh
sudo groupadd --system mihomo
sudo useradd --system --no-create-home --gid mihomo --shell /bin/false mihomo
sudo gpasswd -a $USER mihomo  # 请重新登录使得组的文件权限生效, 后续会用到。
groups $USER                  # 查看是否已经加入 mihomo group
```

3.  创建一些必要的文件

```sh
sudo mkdir -p /opt/clashtui/mihomo_config
cat > /opt/clashtui/mihomo_config/config.yaml << EOF
mixed-port: 7890
external-controller: 127.0.0.1:9090
EOF

# Optional. 提前下载 geo 文件, mihomo 服务第一次启动会更加快地响应
sudo curl -o /opt/clashtui/mihomo_config/geoip.metadb https://github.com/MetaCubeX/meta-rules-dat/releases/download/latest/geoip.metadb
sudo curl -o /opt/clashtui/mihomo_config/GeoSite.dat https://github.com/MetaCubeX/meta-rules-dat/releases/download/latest/geosite.dat

sudo chown -R mihomo:mihomo /opt/clashtui/mihomo_config
```

4.  创建 systemd unit `clashtui_mihomo.service`

建议使用 mihomo doc 提供的[文件](https://wiki.metacubex.one/startup/service/), 不建议使用安装提供的。
因为可能存在差异不方便统一修改, 当然你了解的话, 可以使用安装提供的。

创建 systemd 配置文件 /etc/systemd/system/clashtui_mihomo.service: (加了 User 和 Group)

```
[Unit]
Description=mihomo Daemon, Another Clash Kernel.
After=network.target NetworkManager.service systemd-networkd.service iwd.service

[Service]
Type=simple
User=mihomo
Group=mihomo
LimitNPROC=500
LimitNOFILE=1000000
CapabilityBoundingSet=CAP_NET_ADMIN CAP_NET_RAW CAP_NET_BIND_SERVICE CAP_SYS_TIME CAP_SYS_PTRACE CAP_DAC_READ_SEARCH CAP_DAC_OVERRIDE
AmbientCapabilities=CAP_NET_ADMIN CAP_NET_RAW CAP_NET_BIND_SERVICE CAP_SYS_TIME CAP_SYS_PTRACE CAP_DAC_READ_SEARCH CAP_DAC_OVERRIDE
Restart=always
ExecStartPre=/usr/bin/sleep 1s
ExecStart=/opt/clashtui/mihomo -d /opt/clashtui/mihomo_config
ExecReload=/bin/kill -HUP $MAINPID

[Install]
WantedBy=multi-user.target
```

链接 mihomo 程序:

```sh
sudo ln -s $(which mihomo) /opt/clashtui/mihomo
```

可选。设置开机启动:

```sh
sudo systemctl enable clashtui_mihomo
```

建议先启动 clashtui_mihomo systemd unit 检查是否存在什么问题:

```sh
sudo systemctl start clashtui_mihomo
```

## 配置 clashtui

先运行 clashtui, 使其他生成一些必要文件。`$XDG_CONFIG_HOME/clashtui`

修改 `$XDG_CONFIG_HOME/clashtui/config.yaml`。配置参考 [ref](./clashtui_usage_zh.md)

可选。使用仓库的 `basic_clash_config.yaml`:

```sh
curl -o $XDG_CONFIG_HOME/clashtui/basic_clash_config.yaml https://raw.githubusercontent.com/JohanChane/clashtui/refs/heads/main/InstallRes/basic_clash_config.yaml
```

## 下载模板 (可选)

```sh
cd $XDG_CONFIG_HOME/clashtui/templates

curl -O "https://raw.githubusercontent.com/JohanChane/clashtui/refs/heads/main/InstallRes/templates/common_tpl.yaml"
curl -O "https://raw.githubusercontent.com/JohanChane/clashtui/refs/heads/main/InstallRes/templates/generic_tpl.yaml"
curl -O "https://raw.githubusercontent.com/JohanChane/clashtui/refs/heads/main/InstallRes/templates/generic_tpl_with_all.yaml"
curl -O "https://raw.githubusercontent.com/JohanChane/clashtui/refs/heads/main/InstallRes/templates/generic_tpl_with_filter.yaml"
curl -O "https://raw.githubusercontent.com/JohanChane/clashtui/refs/heads/main/InstallRes/templates/generic_tpl_with_ruleset.yaml"
```
