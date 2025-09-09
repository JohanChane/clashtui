# Install ClashTUI Manually

## Install Mihomo Service (Enable Tun Mode)

For example: [ArchLinux](https://aur.archlinux.org/packages/mihomo).

```sh
# ## Install mihomo
paru -S mihomo

# ## Add mihomo hook
# cat /etc/pacman.d/hooks/mihomo.hook (If there is no system similar to hook, use ClashSrvCtl Tab's SetPermission or use mihomo@root service)
[Trigger]
Operation = Install
Operation = Upgrade
Type = Path
Target = usr/bin/mihomo

[Action]
When = PostTransaction
Exec = /usr/bin/setcap 'cap_net_admin,cap_net_bind_service=+ep' /usr/bin/mihomo

# ## Edit mihomo service unit
# systemctl edit mihomo
[Service]
# Remove original ExecStart
ExecStart=
ExecStart=/usr/bin/mihomo -d /srv/mihomo -f /srv/mihomo/config.yaml

# ## Create /srv/mihomo
mkdir /srv/mihomo
cd /srv/mihomo
chown -R mihomo:mihomo /srv/mihomo
usermod -a -G mihomo <user>
groups <user>       # Check if already added to mihomo group

# Optional. After version 0.2.0, clashtui will automatically fix file permissions.
chmod g+w /srv/mihomo               # clashtui needs permission to create files.
chmod g+s /srv/mihomo               # Make the group of files created by clashtui mihomo. This is to give clashtui group read and write permissions to files in this directory.
chmod g+w /srv/mihomo/config.yaml   # clashtui needs write permission.

# ## Set mihomo service unit
systemctl enable mihomo  # Start on boot
systemctl restart mihomo  # Start service
```

It is recommended to test the mihomo service with a valid configuration to ensure its success. Check if [meta-rules-dat](https://github.com/MetaCubeX/meta-rules-dat) file is missing.

`mihomo.service` of `mihomo` package:

```
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
ExecStart=/usr/bin/mihomo -d /etc/mihomo

[Install]
WantedBy=multi-user.target
```

## Install ClashTui

For example: ArchLinux

```sh
# ## Install clashtui
# There is a latest [PKGBUILD](./PkgManagers/PKGBUILD).
paru -S clashtui.      # For other Linux distributions, manually download and place clashtui in PATH.

# ## Configure clashtui
clashtui                # Running this will generate some default files in ~/.config/clashtui.

# nvim ~/.config/clashtui/config.yaml
basic:
  clash_config_dir: '/srv/mihomo'
  clash_bin_path: '/usr/bin/mihomo'
  clash_config_path: '/srv/mihomo/config.yaml'
  timeout: null
service:
  clash_srv_name: 'mihomo'
  is_user: false
extra:
  edit_cmd: ''
  open_dir_cmd: ''
```

The subsequent versions of clashtui have not been uploaded to `crates.io` because clashtui is now separated into multiple modules. If uploaded to `crates.io`, it would require uploading each dependent module, and some modules do not need to be uploaded to `crates.io`. See [ref](https://users.rust-lang.org/t/is-it-possible-to-publish-crates-with-path-specified/91497/2). So, do not use `cargo install clashtui` for installation.
