# Install Clashtui Manually

## Installing Clashtui

For example: ArchLinux

1. Install mihomo and clashtui

```sh
sudo pacman -S mihomo clashtui
```

Later versions of clashtui have not been uploaded to `crates.io` because clashtui has been split into multiple modules.  
If uploaded to `crates.io`, each dependent module would need to be uploaded as well, and some modules do not necessarily need to be on `crates.io`.  
See [ref](https://users.rust-lang.org/t/is-it-possible-to-publish-crates-with-path-specified/91497/2).  
Therefore, do not use `cargo install clashtui` for installation.

2. Create the mihomo user and mihomo group, then add the user to the group

*Note: These may have already been created during mihomo installation.*

```sh
sudo groupadd --system mihomo
sudo useradd --system --no-create-home --gid mihomo --shell /bin/false mihomo
sudo gpasswd -a $USER mihomo  # Please log out and back in for the group file permissions to take effect; this will be used later.
groups $USER                  # Check if you have been added to the mihomo group
```

3. Create necessary files

```sh
sudo mkdir -p /opt/clashtui/mihomo_config
cat > /opt/clashtui/mihomo_config/config.yaml << EOF
mixed-port: 7890
external-controller: 127.0.0.1:9090
EOF

# Optional. Pre-download geo files to allow the mihomo service to respond faster on first startup
sudo curl -o /opt/clashtui/mihomo_config/geoip.metadb https://github.com/MetaCubeX/meta-rules-dat/releases/download/latest/geoip.metadb
sudo curl -o /opt/clashtui/mihomo_config/GeoSite.dat https://github.com/MetaCubeX/meta-rules-dat/releases/download/latest/geosite.dat

sudo chown -R mihomo:mihomo /opt/clashtui/mihomo_config
```

4. Create the systemd unit `clashtui_mihomo.service`

It is recommended to use the [file](https://wiki.metacubex.one/startup/service/) provided by the mihomo documentation rather than the one included with the installation, as there may be differences that make unified modifications inconvenient. Of course, if you are familiar with it, you may use the installation-provided file.

Create the systemd configuration file at /etc/systemd/system/clashtui_mihomo.service: (with User and Group added)

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

Link the mihomo executable:

```sh
sudo ln -s $(which mihomo) /opt/clashtui/mihomo
```

Optional. Enable startup on boot:

```sh
sudo systemctl enable clashtui_mihomo
```

It is recommended to start the clashtui_mihomo systemd unit first to check for any issues:

```sh
sudo systemctl start clashtui_mihomo
```

## Configuring Clashtui

First, run clashtui to generate necessary files in `$XDG_CONFIG_HOME/clashtui`.

Modify `$XDG_CONFIG_HOME/clashtui/config.yaml`. For configuration reference, see [ref](./clashtui_usage_zh.md).

Optional. Use the repository's `basic_clash_config.yaml`:

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
