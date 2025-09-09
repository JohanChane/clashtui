# Install ClashTUI Manually

## 安装 Mihomo 服务 (启用 Tun 模式)

比如: [ArchLinux](https://aur.archlinux.org/packages/mihomo)。

```sh
# ## 安装 mihomo
paru -S mihomo

# ## 添加 mihomo hook
# cat /etc/pacman.d/hooks/mihomo.hook (没有类似于 hook 的系统可以使用 ClashSrvCtl Tab 的 SetPermission 或者使用 mihomo@root 服务)
[Trigger]
Operation = Install
Operation = Upgrade
Type = Path
Target = usr/bin/mihomo

[Action]
When = PostTransaction
Exec = /usr/bin/setcap 'cap_net_admin,cap_net_bind_service=+ep' /usr/bin/mihomo

# ## 编辑 mihomo service unit
# systemctl edit mihomo
[Service]
# 删除原先的 ExecStart
ExecStart=
ExecStart=/usr/bin/mihomo -d /srv/mihomo -f /srv/mihomo/config.yaml

# ## 创建 /srv/mihomo
mkdir /srv/mihomo
cd /srv/mihomo
chown -R mihomo:mihomo /srv/mihomo
usermod -a -G mihomo <user>
groups <user>       # 查看是否已经加入 mihomo group

# Optional. 0.2.0 之后版本的 clashtui 会自动修复文件的权限。
chmod g+w /srv/mihomo               # clashtui 要有创建文件的权限。
chmod g+s /srv/mihomo               # 使 clashtui 创建的文件的组为 mihomo。为了使 clashtui 对该目录的文件有组的读写权限。
chmod g+w /srv/mihomo/config.yaml   # clashtui 要有写的权限。

# ## 设置 mihomo service unit
systemctl enable mihomo  # 开机启动
systemctl restart mihomo  # 启动服务
```

建议先用一个可用的 mihomo 配置测试 mihomo 服务是否成功。检查是否缺少 [meta-rules-dat](https://github.com/MetaCubeX/meta-rules-dat) 文件。

`mihomo` package 提供的 `mihomo.service`:

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

## 安装 clashtui

比如: ArchLinux

```sh
# ## 安装 clashtui
# 有最新的 [PKGBUILD](./PkgManagers/PKGBUILD)。
paru -S clashtui。      # 其他 linux 发行版, 手动下载, 将 clashtui 放在 PATH 即可。

# ## 配置 clashtui
clashtui                # 先运行会在 ~/.config/clashtui 生成一些默认文件。

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

clashtui 后续的版本没有上传到 `crates.io`, 因为现在 clashtui 分离为多个模块, 如果上传到 `crates.io`, 需要上传依赖的每个模块, 而有些模块没有必要上传到 `crates.io`。See [ref](https://users.rust-lang.org/t/is-it-possible-to-publish-crates-with-path-specified/91497/2)。所以不要使用 `cargo install clashtui` 来安装了。
