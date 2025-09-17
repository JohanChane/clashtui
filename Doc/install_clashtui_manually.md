# Install ClashTUI Manually

## Install mihomo

1.  Install the mihomo program

ArchLinux:

```sh
sudo pacman -S mihomo
```

2.  Create mihomo user and mihomo group, and add the user to the group

```sh
sudo groupadd --system mihomo
sudo useradd --system --no-create-home --gid mihomo --shell /bin/false mihomo
sudo gpasswd -a $USER mihomo  # Please log out and log back in for the group file permissions to take effect; this will be used later.
groups $USER                  # Check if you have been added to the mihomo group
```

*It is possible that the mihomo user and group were already created during the installation of mihomo.*

## Install ClashTUI

Install the ClashTUI program, e.g., on ArchLinux:

```sh
sudo pacman -S clashtui
```

It is not recommended to use `cargo install clashtui` for installation:
-   Because subsequent versions of ClashTUI have not been uploaded to `crates.io`, as ClashTUI is now split into multiple modules.
-   Uploading to `crates.io` would require uploading each dependent module, and some modules do not need to be uploaded to `crates.io`. See [ref](https://users.rust-lang.org/t/is-it-possible-to-publish-crates-with-path-specified/91497/2).

## Run mihomo

1.  Create necessary files

```sh
sudo mkdir -p /opt/clashtui/mihomo_config
cat > /opt/clashtui/mihomo_config/config.yaml << EOF
mixed-port: 7890
external-controller: 127.0.0.1:9090
EOF

# Optional. Download geo files in advance to make the first startup of the mihomo service respond faster.
sudo curl -o /opt/clashtui/mihomo_config/geoip.metadb https://github.com/MetaCubeX/meta-rules-dat/releases/download/latest/geoip.metadb
sudo curl -o /opt/clashtui/mihomo_config/GeoSite.dat https://github.com/MetaCubeX/meta-rules-dat/releases/download/latest/geosite.dat

sudo chown -R mihomo:mihomo /opt/clashtui/mihomo_config
```

2.  Create a systemd unit `clashtui_mihomo.service`

It is recommended to use the [file](https://wiki.metacubex.one/startup/service/) provided by the mihomo documentation rather than the one provided by the installation.

There may be differences that make unified modifications inconvenient. However, if you are familiar with it, you can use the one provided by the installation.

Create the systemd configuration file /etc/systemd/system/clashtui_mihomo.service: (Added User and Group)

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

3.  Link the mihomo program (Optional):

```sh
sudo ln -s $(which mihomo) /opt/clashtui/mihomo
```

4.  Enable startup on boot (Optional):

```sh
sudo systemctl enable clashtui_mihomo
```

It is recommended to start the clashtui_mihomo systemd unit first to check for any issues:

```sh
sudo systemctl start clashtui_mihomo
```

## Configure ClashTUI

First, run ClashTUI to generate necessary files. Then modify `$XDG_CONFIG_HOME/clashtui/config.yaml`. For configuration reference, see [ref](./clashtui_usage.md).

Use the repository's `basic_clash_config.yaml` (Optional):

```sh
curl -o $XDG_CONFIG_HOME/clashtui/basic_clash_config.yaml https://raw.githubusercontent.com/JohanChane/clashtui/refs/heads/main/InstallRes/basic_clash_config.yaml
```

## Download Templates (Optional)

```sh
cd $XDG_CONFIG_HOME/clashtui/templates

curl -O "https://raw.githubusercontent.com/JohanChane/clashtui/refs/heads/main/InstallRes/templates/common_tpl.yaml"
curl -O "https://raw.githubusercontent.com/JohanChane/clashtui/refs/heads/main/InstallRes/templates/generic_tpl.yaml"
curl -O "https://raw.githubusercontent.com/JohanChane/clashtui/refs/heads/main/InstallRes/templates/generic_tpl_with_all.yaml"
curl -O "https://raw.githubusercontent.com/JohanChane/clashtui/refs/heads/main/InstallRes/templates/generic_tpl_with_filter.yaml"
curl -O "https://raw.githubusercontent.com/JohanChane/clashtui/refs/heads/main/InstallRes/templates/generic_tpl_with_ruleset.yaml"
```
