# Clashtui 使用指南

Clashtui 是一个终端工具，用于管理 Mihomo（Clash.Meta）和 sing-box 代理核心。你可以在一个简洁的界面里切换节点、更新订阅、查看连接状态和控制服务启停。

## 获取与安装

从 [GitHub Releases](https://github.com/JohanChane/clashtui/releases) 下载对应平台的二进制文件（如 `clashtui-linux-amd64`），解压后放到 `PATH` 目录即可：

```sh
chmod +x clashtui
sudo mv clashtui /usr/local/bin/
```

## 启动

```sh
clashtui
```

第一次运行会自动在 `~/.config/clashtui` 下创建配置目录和默认文件。

### 指定配置目录

可以用 `--config-dir` 参数或 `CLASHTUI_CONFIG_DIR` 环境变量指定其他目录：

```sh
clashtui --config-dir /my/config/path
```

如果可执行文件同目录下存在 `data/` 子目录，会自动使用它（便携模式，适合放 U 盘）。

## TUI 界面

进入 TUI 后，顶部会显示一排标签页，底部有提示栏。

### 标签页

| 按键 | 标签页 | 做什么 |
|------|--------|--------|
| `1` | Status | 查看核心状态：上行/下行速率、内存、运行时间 |
| `2` | Files | 管理订阅（Profile）和模板 |
| `3` | Proxies | 切换代理节点、查看延迟、管理代理组 |
| `4` | Connections | 查看当前所有连接，可关闭单个或全部连接 |
| `5` | Logs | 实时查看核心日志 |
| `6` | Settings | 修改 Clashtui 设置项 |
| `7` | CoreSrvCtl | 控制核心服务：启动、停止、重启、切换 Mihomo / sing-box |

### 快捷键

**全局（所有页面生效）：**

| 快捷键 | 功能 |
|--------|------|
| `1` ~ `7` | 跳到对应标签页 |
| `Tab` | 下一个标签页 |
| `q` 或 `Ctrl-c` | 退出 |
| `?` | 显示快捷键帮助 |
| `Ctrl-g` 再按 `c` | 在文件管理器中打开 Clashtui 配置目录 |
| `Ctrl-g` 再按 `m` | 打开核心配置目录 |
| `Ctrl-g` 再按 `f` | 启动核心服务 |
| `Ctrl-g` 再按 `t` | 关闭所有连接 |

> 其他页面内的快捷键在各标签页中按 `?` 查看。

### 自定义按键

在配置目录下创建 `keymap.yaml` 即可修改各页面的按键。比如把上下移动改成 `j` `k`：

```yaml
proxies:
  j: SelectDown
  k: SelectUp
  enter: ToggleExpand

connections:
  j: SelectDown
  k: SelectUp

settings:
  j: SelectDown
  k: SelectUp
  enter: Edit

file:
  profile:
    j: SelectDown
    k: SelectUp
  template:
    j: SelectDown
    k: SelectUp
```

也可以写成列表格式，自定义描述文字：

```yaml
proxies:
  - key: j
    action: SelectDown
    desc: 下移
  - keys: ["g", "g"]
    action: ToggleExpand
    desc: 展开/折叠
```

## 命令行模式

不进入 TUI 也可以用命令操作，适合写脚本或自动化。

### 管理订阅（Profile）

```sh
# 查看当前使用的订阅
clashtui profile select

# 列出所有订阅
clashtui profile list

# 只显示名称
clashtui profile list --name-only

# 按类型筛选：file / url / template / singbox
clashtui profile list --type url

# 切换到指定订阅
clashtui profile select --name 我的订阅

# 更新当前订阅
clashtui profile update --name 我的订阅

# 更新全部订阅（包含当前使用的）
clashtui profile update --all

# 更新时走代理
clashtui profile update --all --with-proxy
```

通过上面的命令, 可以结合 [cron](https://wiki.archlinuxcn.org/wiki/Cron) 定时更新 profiles。

### 切换模式

```sh
# 查看当前模式
clashtui mode

# 设置为规则模式
clashtui mode rule

# 设置为全局
clashtui mode global

# 设置为直连
clashtui mode direct
```

### 控制服务

```sh
# 硬重启（通过 systemd）
clashtui service restart

# 软重启（通过 API，不重启进程）
clashtui service restart --soft

# 停止服务
clashtui service stop
```

### 检查更新

```sh
# 检查 Clashtui 自身更新
clashtui update clashtui

# 检查 Mihomo 核心更新
clashtui update mihomo
```

## 订阅（Profile）类型

Clashtui 支持三种订阅方式：

### 1. File 订阅

直接选择一个本地的 YAML（Mihomo）或 JSON（sing-box）配置文件作为订阅。文件不会丢失，Clashtui 只在其基础上做覆盖。

### 2. URL 订阅

输入一个代理订阅链接，Clashtui 会自动下载并追踪更新。更新时会同步下载 proxy-provider 中的资源。

### 3. Template 订阅

最灵活的方式。你写一个模板文件，里面定义好 DNS、路由规则、inbounds 等骨架，然后用 `template_proxy_providers.yaml` 管理各组代理节点。

**Template 工作原理：**

- 模板放在 `mihomo/templates/`（或 `sing-box/templates/`）目录
- 节点分组定义在 `template_proxy_providers.yaml` 中
- 模板中用 `${PPG.分组名}` 等变量引用节点
- Clashtui 自动将变量展开为具体的代理，生成最终配置文件

一个简单例子——在 Files 标签页中添加一个 Template 类型的 Profile，选择对应的模板文件和 proxy-provider 分组，Clashtui 会自动合成最终可用的配置。

## 配置文件说明

### config.yaml

Clashtui 的主配置，定义核心路径和服务名称：

```yaml
mihomo:
  core:
    config_dir: /opt/clashtui/mihomo/config     # 核心配置目录
    bin_path: /opt/clashtui/mihomo/mihomo       # 核心程序路径
    config_path: /opt/clashtui/mihomo/config/config.yaml  # 最终配置文件路径
  core_service:
    service_name: clashtui_mihomo                # systemd 服务名
    is_user: false                               # 是否为用户服务
singbox:
  core:
    bin_path: /opt/clashtui/sing-box/sing-box
    config_dir: /opt/clashtui/sing-box/config
    config_path: /opt/clashtui/sing-box/config/config.json
  core_service:
    service_name: clashtui_singbox
    is_user: false
timeout: null
extra:
  edit_cmd: kitty -e nvim "%s"      # 编辑文件使用的命令，%s 替换为文件路径
  open_dir_cmd: kitty -e yazi "%s"  # 打开目录使用的命令
```

### core_override_config.yaml

该文件的**顶层字段**会在切换订阅时覆盖订阅配置的对应字段。

```yaml
mixed-port: 7890
allow-lan: false
mode: Rule
log-level: info
```

例如，无论你的订阅里写了什么端口，最终 `mixed-port` 都会是 `7890`。

> 这是 Mihomo 专属的配置。对 sing-box，请编辑 `sing-box/core_override_config.json`。

### core_override_config.json（sing-box）

sing-box 使用深度合并。你只需写要覆盖的部分，其他字段保留订阅的原样。

```json
{
  "experimental": {
    "clash_api": {
      "external_controller": "127.0.0.1:9090",
      "secret": ""
    }
  },
  "inbounds": [
    {
      "type": "mixed",
      "tag": "mixed-in",
      "listen": "127.0.0.1",
      "listen_port": 7890
    }
  ],
  "log": { "level": "info" }
}
```

覆盖规则：
- 对象：递归合并（你的字段覆盖订阅的，订阅独有的保留）
- 数组：整体替换（你写了哪些，最终就是哪些）
- 数字/字符串：直接覆盖

### 配置目录完整结构

```
~/.config/clashtui/
├── clashtui.db                     # 数据库：保存订阅列表、当前选择等
├── clashtui.log                    # 日志文件
├── config.yaml                     # 主配置
├── keymap.yaml                     # 自定义按键（可选）
├── theme.yaml                      # 自定义主题（可选）
├── mihomo/
│   ├── core_override_config.yaml   # 覆盖配置
│   ├── profiles/                   # 下载的订阅原始文件
│   ├── templates/                  # 模板文件
│   ├── provider-cache/             # provider 缓存
│   └── template_proxy_providers.yaml  # 模板的节点分组
└── sing-box/
    ├── core_override_config.json
    ├── profiles/
    ├── templates/
    ├── proxy-providers/
    └── template_proxy_providers.yaml
```

## 核心管理

### 切换核心

在 CoreSrvCtl 标签页（按 `7`）中，可以切换 Mihomo 和 sing-box 两种核心。切换后需要重启 Clashtui。

Clashtui 会自动检测当前运行的核心类型是否与设置匹配。如果不匹配，会弹窗提示并阻止显示错误数据。

### 文件权限（Linux）

Clashtui 使用系统组权限管理核心目录的访问。启动时会自动检查 `config_dir` 下的文件权限，如需修复会提示你确认。

## 日志

日志写在 `<配置目录>/clashtui.log`。需要排查问题时，可以调高日志级别：

```sh
clashtui -v     # 显示更多信息
clashtui -vv    # 显示调试信息
```

## 常见问题

**非 is-user 模式下核心启动后无法下载规则文件？**
非 `is-user` 安装默认会开启 TUN，可能导致核心无法下载启动时需要的文件（如 rule-set、geoip 等）。临时解决方法是：先将模式切换为 Direct（`clashtui mode direct`），等核心下载完需要的文件后，再切回原来的模式（`clashtui mode rule`）。

**启动后界面上没有数据？**
请确保 Mihomo 或 sing-box 核心已启动。到 CoreSrvCtl 标签页（按 `7`）可以启动服务。

**提示 "core mismatch"？**
说明正在运行的核心和 Clashtui 设置的不一致。到 CoreSrvCtl 标签页确认你实际想用的是哪个核心，切换后重启 Clashtui。

**怎么添加订阅？**
进入 Files 标签页（按 `2`），按 `?` 查看快捷键，通常用 `a` 添加新 Profile，输入订阅 URL 即可。Clashtui 会自动下载和更新。

**怎么创建 sing-box 模板订阅？**
1. 在 `sing-box/templates/` 下放一个 JSON 模板文件
2. 在 `template_proxy_providers.yaml` 里写好你的节点分组和 URL
3. 在 Files 标签页创建 Template 类型的 Profile，选择刚写的模板

**配置目录里的 profiles 目录是做什么的？**
存放的是你订阅的原始内容。这些文件不会被 override 配置影响——覆盖只在切换订阅、写入核心配置目录时才生效。
