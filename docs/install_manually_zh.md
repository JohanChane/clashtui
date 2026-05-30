# ClashTui 手动安装指南

以 Arch Linux, amd64, systemd (system 模式) 为例。

## 目录布局

安装后的目录结构:

```
/opt/clashtui/
├── bin/
│   └── clashtui
├── mihomo/
│   ├── mihomo
│   └── config/
│       └── config.yaml
└── sing-box/
    ├── sing-box
    └── config/
        └── config.json

~/.config/clashtui/
├── config.yaml
├── default_keymap.yaml
├── default_theme.yaml
├── mihomo/
│   ├── core_override_config.yaml
│   ├── template_proxy_providers.yaml
│   ├── profiles/
│   └── templates/
└── sing-box/
    ├── core_override_config.json
    ├── template_proxy_providers.yaml
    ├── profiles/
    └── templates/
```

## 1. 下载 ClashTui

```sh
curl -LO "https://github.com/JohanChane/clashtui/releases/latest/download/clashtui-linux-amd64-$(curl -s https://api.github.com/repos/JohanChane/clashtui/releases/latest | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/').gz"
gunzip clashtui-linux-amd64-*.gz
chmod +x clashtui-linux-amd64-*
sudo mkdir -p /opt/clashtui/bin
sudo install -m 755 clashtui-linux-amd64-* /opt/clashtui/bin/clashtui
sudo ln -sf /opt/clashtui/bin/clashtui /usr/local/bin/clashtui
```

## 2. 下载 Mihomo

```sh
mihomo_ver=$(curl -s https://api.github.com/repos/MetaCubeX/mihomo/releases/latest | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
curl -L "https://github.com/MetaCubeX/mihomo/releases/latest/download/mihomo-linux-amd64-${mihomo_ver}.gz" -o /tmp/mihomo.gz
gunzip -c /tmp/mihomo.gz > /tmp/mihomo
sudo mkdir -p /opt/clashtui/mihomo
sudo install -m 755 /tmp/mihomo /opt/clashtui/mihomo/mihomo
```

## 3. 下载 sing-box

```sh
sb_ver=$(curl -s https://api.github.com/repos/SagerNet/sing-box/releases/latest | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
curl -L "https://github.com/SagerNet/sing-box/releases/latest/download/sing-box-${sb_ver}-linux-amd64.tar.gz" -o /tmp/sing-box.tar.gz
tar -xzf /tmp/sing-box.tar.gz -C /tmp/
sudo mkdir -p /opt/clashtui/sing-box
sudo install -m 755 /tmp/sing-box-*/sing-box /opt/clashtui/sing-box/sing-box
sudo ln -sf /opt/clashtui/sing-box/sing-box /usr/bin/sing-box
```

## 4. 创建 ClashTui 主配置

```sh
mkdir -p ~/.config/clashtui
```

`~/.config/clashtui/config.yaml`:

```yaml
mihomo:
  core:
    config_dir: /opt/clashtui/mihomo/config
    bin_path: /opt/clashtui/mihomo/mihomo
    config_path: /opt/clashtui/mihomo/config/config.yaml
  core_service:
    service_name: clashtui_mihomo
    is_user: false
    service_controller: systemd
singbox:
  core:
    bin_path: /opt/clashtui/sing-box/sing-box
    config_dir: /opt/clashtui/sing-box/config
    config_path: /opt/clashtui/sing-box/config/config.json
  core_service:
    service_name: clashtui_singbox
    is_user: false
    service_controller: systemd
timeout: null
extra:
  edit_cmd:
  open_dir_cmd:
```

## 5. 创建 Core 配置

```sh
sudo mkdir -p /opt/clashtui/mihomo/config
sudo mkdir -p /opt/clashtui/sing-box/config
mkdir -p ~/.config/clashtui/mihomo
mkdir -p ~/.config/clashtui/sing-box
```

从 GitHub 下载 core override 配置:

```sh
base="https://raw.githubusercontent.com/JohanChane/clashtui/main/contrib/default_configs"

# Mihomo
curl -o ~/.config/clashtui/mihomo/core_override_config.yaml \
  "${base}/mihomo/core_override_config.yaml"
sudo cp ~/.config/clashtui/mihomo/core_override_config.yaml \
  /opt/clashtui/mihomo/config/config.yaml

# sing-box
curl -o ~/.config/clashtui/sing-box/core_override_config.json \
  "${base}/sing-box/core_override_config.json"
sudo cp ~/.config/clashtui/sing-box/core_override_config.json \
  /opt/clashtui/sing-box/config/config.json
```

## 6. 下载默认文件

```sh
curl -o ~/.config/clashtui/default_keymap.yaml \
  "${base}/default_keymap.yaml"
curl -o ~/.config/clashtui/default_theme.yaml \
  "${base}/default_theme.yaml"
```

## 7. 创建用户、权限

```sh
sudo groupadd --system mihomo
sudo useradd --system --no-create-home --gid mihomo --shell /bin/false mihomo
sudo groupadd --system sing-box
sudo useradd --system --no-create-home --gid sing-box --shell /bin/false sing-box

sudo gpasswd -a $USER mihomo
sudo gpasswd -a $USER sing-box

# 将用户加入组后, 需要重新登录才能生效

sudo chown -R mihomo:mihomo /opt/clashtui/mihomo
sudo chmod g+rwxs /opt/clashtui/mihomo/config
sudo chown -R sing-box:sing-box /opt/clashtui/sing-box
sudo chmod g+rwxs /opt/clashtui/sing-box/config
```

## 8. 创建 systemd 服务

**clashtui_mihomo** — 写入 `/opt/clashtui/mihomo/clashtui_mihomo.service`:

```ini
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
ExecStart=/opt/clashtui/mihomo/mihomo -d /opt/clashtui/mihomo/config
ExecReload=/bin/kill -HUP $MAINPID

[Install]
WantedBy=multi-user.target
```

**clashtui_singbox** — 写入 `/opt/clashtui/sing-box/clashtui_singbox.service`:

```ini
[Unit]
Description=sing-box Daemon, The universal proxy platform.
After=network.target NetworkManager.service systemd-networkd.service iwd.service

[Service]
Type=simple
User=sing-box
Group=sing-box
StateDirectory=sing-box
CapabilityBoundingSet=CAP_NET_ADMIN CAP_NET_RAW CAP_NET_BIND_SERVICE CAP_SYS_PTRACE CAP_DAC_READ_SEARCH
AmbientCapabilities=CAP_NET_ADMIN CAP_NET_RAW CAP_NET_BIND_SERVICE CAP_SYS_PTRACE CAP_DAC_READ_SEARCH
ExecStartPre=/usr/bin/sleep 1s
ExecStart=/usr/bin/sing-box -D /opt/clashtui/sing-box/config -c /opt/clashtui/sing-box/config/config.json run
ExecReload=/bin/kill -HUP $MAINPID
Restart=on-failure
RestartSec=10s
LimitNOFILE=infinity

[Install]
WantedBy=multi-user.target
```

启用服务:

```sh
sudo ln -sf /opt/clashtui/mihomo/clashtui_mihomo.service /usr/lib/systemd/system/
sudo ln -sf /opt/clashtui/sing-box/clashtui_singbox.service /usr/lib/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable clashtui_mihomo
sudo systemctl enable clashtui_singbox
```

## 9. 创建订阅配置

`~/.config/clashtui/mihomo/template_proxy_providers.yaml`:

```yaml
# 定义代理提供者的订阅 URL
# pvd:
#   pvd0: "https://example.com/sub.yaml"
```

`~/.config/clashtui/sing-box/template_proxy_providers.yaml` 同理。

创建必要的目录:

```sh
mkdir -p ~/.config/clashtui/mihomo/{profiles,templates}
mkdir -p ~/.config/clashtui/sing-box/{profiles,templates}
```

## 10. 下载规则文件 (可选)

```sh
curl -Lo /opt/clashtui/mihomo/config/geoip.metadb \
  https://github.com/MetaCubeX/meta-rules-dat/releases/latest/download/geoip.metadb
curl -Lo /opt/clashtui/mihomo/config/GeoSite.dat \
  https://github.com/MetaCubeX/meta-rules-dat/releases/latest/download/geosite.dat
```

## 11. 启动

```sh
clashtui
```

## 卸载

```sh
sudo systemctl stop clashtui_mihomo clashtui_singbox
sudo rm -f /usr/lib/systemd/system/clashtui_mihomo.service
sudo rm -f /usr/lib/systemd/system/clashtui_singbox.service
sudo systemctl daemon-reload
sudo rm -rf /opt/clashtui
rm -rf ~/.config/clashtui
rm -rf ~/.cache/clashtui
sudo userdel mihomo
sudo userdel sing-box
```
