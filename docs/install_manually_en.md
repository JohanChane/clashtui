# ClashTui Manual Installation Guide

Using Arch Linux, amd64, systemd as an example. Steps for both system mode and user mode are provided below.

---

# System Mode

## Directory Layout

After installation:

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

## 1. Download ClashTui

```sh
curl -LO "https://github.com/JohanChane/clashtui/releases/latest/download/clashtui-linux-amd64-$(curl -s https://api.github.com/repos/JohanChane/clashtui/releases/latest | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/').gz"
gunzip clashtui-linux-amd64-*.gz
chmod +x clashtui-linux-amd64-*
sudo mkdir -p /opt/clashtui/bin
sudo install -m 755 clashtui-linux-amd64-* /opt/clashtui/bin/clashtui
sudo ln -sf /opt/clashtui/bin/clashtui /usr/local/bin/clashtui
```

## 2. Download Mihomo

```sh
mihomo_ver=$(curl -s https://api.github.com/repos/MetaCubeX/mihomo/releases/latest | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
curl -L "https://github.com/MetaCubeX/mihomo/releases/latest/download/mihomo-linux-amd64-${mihomo_ver}.gz" -o /tmp/mihomo.gz
gunzip -c /tmp/mihomo.gz > /tmp/mihomo
sudo mkdir -p /opt/clashtui/mihomo
sudo install -m 755 /tmp/mihomo /opt/clashtui/mihomo/mihomo
```

## 3. Download sing-box

```sh
sb_ver=$(curl -s https://api.github.com/repos/SagerNet/sing-box/releases/latest | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
curl -L "https://github.com/SagerNet/sing-box/releases/latest/download/sing-box-${sb_ver}-linux-amd64.tar.gz" -o /tmp/sing-box.tar.gz
tar -xzf /tmp/sing-box.tar.gz -C /tmp/
sudo mkdir -p /opt/clashtui/sing-box
sudo install -m 755 /tmp/sing-box-*/sing-box /opt/clashtui/sing-box/sing-box
sudo ln -sf /opt/clashtui/sing-box/sing-box /usr/bin/sing-box
```

## 4. Create ClashTui Main Config

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

## 5. Create Core Configs

```sh
sudo mkdir -p /opt/clashtui/mihomo/config
sudo mkdir -p /opt/clashtui/sing-box/config
mkdir -p ~/.config/clashtui/mihomo
mkdir -p ~/.config/clashtui/sing-box
```

Download core override configs from GitHub:

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

## 6. Download Default Files

```sh
curl -o ~/.config/clashtui/default_keymap.yaml \
  "${base}/default_keymap.yaml"
curl -o ~/.config/clashtui/default_theme.yaml \
  "${base}/default_theme.yaml"
```

## 7. Create Users and Permissions

```sh
sudo groupadd --system mihomo
sudo useradd --system --no-create-home --gid mihomo --shell /bin/false mihomo
sudo groupadd --system sing-box
sudo useradd --system --no-create-home --gid sing-box --shell /bin/false sing-box

sudo gpasswd -a $USER mihomo
sudo gpasswd -a $USER sing-box

# After adding your user to the groups, re-login is required for the changes to take effect

sudo chown -R mihomo:mihomo /opt/clashtui/mihomo
sudo chmod g+rwxs /opt/clashtui/mihomo/config
sudo chown -R sing-box:sing-box /opt/clashtui/sing-box
sudo chmod g+rwxs /opt/clashtui/sing-box/config
```

## 8. Create systemd Services

**clashtui_mihomo** — write to `/opt/clashtui/mihomo/clashtui_mihomo.service`:

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

**clashtui_singbox** — write to `/opt/clashtui/sing-box/clashtui_singbox.service`:

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

Enable services:

```sh
sudo ln -sf /opt/clashtui/mihomo/clashtui_mihomo.service /usr/lib/systemd/system/
sudo ln -sf /opt/clashtui/sing-box/clashtui_singbox.service /usr/lib/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable clashtui_mihomo
sudo systemctl enable clashtui_singbox
```

## 9. Create Subscription Configs

`~/.config/clashtui/mihomo/template_proxy_providers.yaml`:

```yaml
# Define proxy-provider subscription URLs here
# pvd:
#   pvd0: "https://example.com/sub.yaml"
```

Same for `~/.config/clashtui/sing-box/template_proxy_providers.yaml`.

Create required directories:

```sh
mkdir -p ~/.config/clashtui/mihomo/{profiles,templates}
mkdir -p ~/.config/clashtui/sing-box/{profiles,templates}
```

## 10. Download Rule Files (Optional)

```sh
curl -Lo /opt/clashtui/mihomo/config/geoip.metadb \
  https://github.com/MetaCubeX/meta-rules-dat/releases/latest/download/geoip.metadb
curl -Lo /opt/clashtui/mihomo/config/GeoSite.dat \
  https://github.com/MetaCubeX/meta-rules-dat/releases/latest/download/geosite.dat
```

## 11. Launch

```sh
clashtui
```

## Uninstall

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

---

# User Mode

Install to `~/.local/clashtui/` without root privileges. Services run as the current user.

## Directory Layout

```
~/.local/clashtui/
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

## 1. Download ClashTui

```sh
curl -LO "https://github.com/JohanChane/clashtui/releases/latest/download/clashtui-linux-amd64-$(curl -s https://api.github.com/repos/JohanChane/clashtui/releases/latest | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/').gz"
gunzip clashtui-linux-amd64-*.gz
chmod +x clashtui-linux-amd64-*
mkdir -p ~/.local/clashtui/bin
install -m 755 clashtui-linux-amd64-* ~/.local/clashtui/bin/clashtui
ln -sf ~/.local/clashtui/bin/clashtui ~/.local/bin/clashtui
```

## 2. Download Mihomo

```sh
mihomo_ver=$(curl -s https://api.github.com/repos/MetaCubeX/mihomo/releases/latest | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
curl -L "https://github.com/MetaCubeX/mihomo/releases/latest/download/mihomo-linux-amd64-${mihomo_ver}.gz" -o /tmp/mihomo.gz
gunzip -c /tmp/mihomo.gz > /tmp/mihomo
mkdir -p ~/.local/clashtui/mihomo
install -m 755 /tmp/mihomo ~/.local/clashtui/mihomo/mihomo
```

## 3. Download sing-box

```sh
sb_ver=$(curl -s https://api.github.com/repos/SagerNet/sing-box/releases/latest | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
curl -L "https://github.com/SagerNet/sing-box/releases/latest/download/sing-box-${sb_ver}-linux-amd64.tar.gz" -o /tmp/sing-box.tar.gz
tar -xzf /tmp/sing-box.tar.gz -C /tmp/
mkdir -p ~/.local/clashtui/sing-box
install -m 755 /tmp/sing-box-*/sing-box ~/.local/clashtui/sing-box/sing-box
ln -sf ~/.local/clashtui/sing-box/sing-box ~/.local/bin/sing-box
```

## 4. Create ClashTui Main Config

```sh
mkdir -p ~/.config/clashtui
```

`~/.config/clashtui/config.yaml`:

```yaml
mihomo:
  core:
    config_dir: <HOME_REPLACE>/.local/clashtui/mihomo/config
    bin_path: <HOME_REPLACE>/.local/clashtui/mihomo/mihomo
    config_path: <HOME_REPLACE>/.local/clashtui/mihomo/config/config.yaml
  core_service:
    service_name: clashtui_mihomo
    is_user: true
    service_controller: systemd
singbox:
  core:
    bin_path: <HOME_REPLACE>/.local/clashtui/sing-box/sing-box
    config_dir: <HOME_REPLACE>/.local/clashtui/sing-box/config
    config_path: <HOME_REPLACE>/.local/clashtui/sing-box/config/config.json
  core_service:
    service_name: clashtui_singbox
    is_user: true
    service_controller: systemd
timeout: null
extra:
  edit_cmd:
  open_dir_cmd:
```

> Replace `<HOME_REPLACE>` with your home directory path, e.g. `/home/johan`. Do not use `~` or `$HOME`.

## 5. Create Core Configs

```sh
mkdir -p ~/.local/clashtui/mihomo/config
mkdir -p ~/.local/clashtui/sing-box/config
mkdir -p ~/.config/clashtui/mihomo
mkdir -p ~/.config/clashtui/sing-box
```

Download core override configs from GitHub:

```sh
base="https://raw.githubusercontent.com/JohanChane/clashtui/main/contrib/default_configs"

# Mihomo
curl -o ~/.config/clashtui/mihomo/core_override_config.yaml \
  "${base}/mihomo/core_override_config.yaml"
cp ~/.config/clashtui/mihomo/core_override_config.yaml \
  ~/.local/clashtui/mihomo/config/config.yaml

# sing-box
curl -o ~/.config/clashtui/sing-box/core_override_config.json \
  "${base}/sing-box/core_override_config.json"
cp ~/.config/clashtui/sing-box/core_override_config.json \
  ~/.local/clashtui/sing-box/config/config.json
```

## 6. Download Default Files

```sh
curl -o ~/.config/clashtui/default_keymap.yaml \
  "${base}/default_keymap.yaml"
curl -o ~/.config/clashtui/default_theme.yaml \
  "${base}/default_theme.yaml"
```

> User mode does **not** require creating system users and groups. Services run as the current user.

## 7. Create systemd User Services

**clashtui_mihomo** — write to `~/.config/systemd/user/clashtui_mihomo.service`:

```sh
mkdir -p ~/.config/systemd/user
```

```ini
[Unit]
Description=mihomo Daemon, Another Clash Kernel.
After=network.target NetworkManager.service systemd-networkd.service iwd.service

[Service]
Type=simple
LimitNPROC=500
LimitNOFILE=1000000
Restart=always
ExecStartPre=/usr/bin/sleep 1s
ExecStart=<HOME_REPLACE>/.local/clashtui/mihomo/mihomo -d <HOME_REPLACE>/.local/clashtui/mihomo/config
ExecReload=/bin/kill -HUP $MAINPID

[Install]
WantedBy=default.target
```

**clashtui_singbox** — write to `~/.config/systemd/user/clashtui_singbox.service`:

```ini
[Unit]
Description=sing-box Daemon, The universal proxy platform.
After=network.target NetworkManager.service systemd-networkd.service iwd.service

[Service]
Type=simple
ExecStartPre=/usr/bin/sleep 1s
ExecStart=<HOME_REPLACE>/.local/clashtui/sing-box/sing-box -D <HOME_REPLACE>/.local/clashtui/sing-box/config -c <HOME_REPLACE>/.local/clashtui/sing-box/config/config.json run
ExecReload=/bin/kill -HUP $MAINPID
Restart=on-failure
RestartSec=10s
LimitNOFILE=infinity

[Install]
WantedBy=default.target
```

> Replace `<HOME_REPLACE>` with your home directory path. In user mode, omit `User=` and `Group=`, and skip `CapabilityBoundingSet` and `AmbientCapabilities` (these require root privileges to take effect).

Enable services:

```sh
systemctl --user daemon-reload
systemctl --user enable clashtui_mihomo
systemctl --user enable clashtui_singbox
```

> By default, user services stop on logout. To keep them running, run: `sudo loginctl enable-linger`

## 8. Create Subscription Configs

`~/.config/clashtui/mihomo/template_proxy_providers.yaml`:

```yaml
# Define proxy-provider subscription URLs here
# pvd:
#   pvd0: "https://example.com/sub.yaml"
```

Same for `~/.config/clashtui/sing-box/template_proxy_providers.yaml`.

Create required directories:

```sh
mkdir -p ~/.config/clashtui/mihomo/{profiles,templates}
mkdir -p ~/.config/clashtui/sing-box/{profiles,templates}
```

## 9. Download Rule Files (Optional)

```sh
curl -Lo ~/.local/clashtui/mihomo/config/geoip.metadb \
  https://github.com/MetaCubeX/meta-rules-dat/releases/latest/download/geoip.metadb
curl -Lo ~/.local/clashtui/mihomo/config/GeoSite.dat \
  https://github.com/MetaCubeX/meta-rules-dat/releases/latest/download/geosite.dat
```

## 10. Launch

```sh
clashtui
```

## Uninstall

```sh
systemctl --user stop clashtui_mihomo clashtui_singbox
rm -f ~/.config/systemd/user/clashtui_mihomo.service
rm -f ~/.config/systemd/user/clashtui_singbox.service
systemctl --user daemon-reload
rm -rf ~/.local/clashtui
rm -rf ~/.config/clashtui
rm -rf ~/.cache/clashtui
```
