# ClashTui 的功能设计

## 功能分类

-   与 core api 相关的功能:
    -   Status、Proxies、Connections 和 Settings tab
-   非 api 相关的功能 (也有可能用到 api):
    -   Files tab
        -   Profile panal
        -   template panel
    -   CoreSrvCtl tab

## ClashTui 的文件结构设计

ClashTui 配置的文件结构 (e.g. `~/.config/clashtui`):

```
.
├── clashtui.db                     # 存放 ClashTui 的持久化数据
├── clashtui.log                    # ClashTui 的日志
├── config.yaml                     # ClashTui 的配置
├── mihomo
│   ├── core_override_config.yaml   # 在生成 config_path 的配置文件时, 该文件的顶层 key 会覆盖 Profile 的顶层 key
│   ├── profiles                    # Profile 对应的 yaml 文件 (mihomo 的配置格式是 yaml)
│   ├── template_proxy_providers.yaml    # 存放生成 template type profile 时, 需要的 pvd_name-url(s)。
│   └── templates                   # template 存放的目录
└── sing-box
    ├── proxy-providers             # proxy-providers 文件的根目录
    ├── core_override_config.json
    ├── profiles                    # Profile 对应的 json 文件 (sing-box 的配置是 json 格式)
    ├── template_proxy_providers.yaml
    └── templates
```

ClashTui Core 的文件结构设计 (e.g. `/opt/clashtui`):

```
.
├── bin
│   └── clashtui -> /home/johan/.local/bin/clashtui
├── mihomo
│   ├── clashtui_mihomo.service       # Mihomo Core 的 systemd unit file
│   ├── config                        # Core Config Dir
│   │   └── config.yaml               # Core Config Path
│   └── mihomo -> /usr/bin/mihomo
└── sing-box
    ├── clashtui_singbox.service
    ├── config                        # Core Config Dir
    │   └── config.json               # Core Config Path
    └── sing-box -> /usr/bin/sing-box
```

## ServiceController (服务控制器)

ClashTui 支持多种服务管理器，在编译时根据平台自动选择默认值:

| Controller      | 平台          | 实现方式                      | 说明                        |
|-----------------|---------------|------------------------------|----------------------------|
| Systemd         | Linux (默认)   | `systemctl` CLI              | systemd 服务管理             |
| OpenRc          | Linux (可选)   | `rc-service` CLI             | OpenRC 服务管理              |
| WindowsService  | Windows (默认) | `windows-service` Rust crate | 直接调 Windows SCM API       |
| Launchd         | macOS (默认)   | `launchctl` CLI              | launchd 服务管理             |

各平台编译时默认:
- `cfg!(windows)` → WindowsService
- `cfg!(target_os = "macos")` → Launchd
- 其他 → Systemd

## ClashTui 的 clashtui.db 格式设计

```yaml
core_type: mihomo
mihomo:
  cur_profile:
  profiles:
sing-box:
  cur_profile:
  profiles:
```

设计原则: Mihomo 和 sing-box 不能共同使用的, 分别放在 mihomo 和 sing-box section。

## ClashTui 的配置设计

```
mihomo:
  core:
    config_dir: /opt/clashtui/mihomo/config
    bin_path: /opt/clashtui/mihomo/mihomo
    config_path: /opt/clashtui/mihomo/config/config.yaml
  core_service:
    service_name: clashtui_mihomo
    is_user: false
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
  edit_cmd: ghostty -e nvim "%s"
  open_dir_cmd: ghostty -e yazi "%s"
```

设计原则: Mihomo 和 sing-box 不能共同使用的, 分别放在 mihomo 和 sing-box section。

## ClashTui 管理 Core 文件的设计

ClashTui 使用 Linux 组文件权限管理 Core 的文件: User 加入每个 Core 的文件权限的组即可。

文件权限的检测与修复:
-   ClashTui 启动时, 取得 Core 目录 e.g. `/opt/clashtui/mihomo` 的 Group name
-   然后递归判断 Core 目录下的文件的 Group name 是否一致。
-   如果不一致则统一修复。否则不做什么。
-   同时确保 Core 目录设置 Group sticky bit。

为了使用户知道修改了什么, ClashTui 会转到 CLI 模式, 让用户输入密码。修复文件权限之后, ClashTui 重新启动。

## Mihomo 和 sing-box 配置合并设计

### Mihomo 配置的合并

使用 basic_core_config 的顶层 key 覆盖 profile 的顶层 key 即可。

我觉得 Mihomo 的合并规则比 sing-box 更加好, 不容易污染。因为 mihomo 的顶层字段 (Section) 耦合度不高。

### sing-box 配置的合并

因为 sing-box 的顶层字段 (Section) 耦合度比较高, 所以使用以下的合并方式。

sing-box 的合并由 clashtui 自行实现递归深合并，不再依赖外部 `sing-box merge` 命令。

合并算法：

- 对象 (object): 递归合并。override 中存在的 key 覆盖 profile 的对应值；profile 中独有的 key 保留 (没有交集的 key)。
- 数组 (array): 整体替换。override 中存在的数组完全替换 profile 的对应数组。可以防止出现多个 inbounds。
- 标量 (string, number, bool, null): 直接覆盖。

合并时机：用户选择 profile 时触发。流程为：

1. 读取 `sing-box/profiles/<profile_name>.json` 作为 base
2. 读取 `sing-box/core_override_config.json` 作为 overlay（如果文件不存在则跳过合并，直接使用 profile 原样）
3. 将 overlay 递归深合并到 base 上
4. 将合并结果写入 core config path
5. Reload core service

`core_override_config.json` 使用标准 sing-box JSON 语法，字段与 sing-box 配置文档一致。
用户只需写需要覆盖的部分即可，例如只覆盖 inbounds + experimental + log：

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
    },
    {
      "type": "tun",
      "tag": "tun-in",
      "stack": "gvisor",
      "auto_route": true,
      "address": ["172.19.0.1/30"]
    }
  ],
  "log": {
    "level": "info"
  }
}
```

合并示例：

```
profile.json:                           core_override_config.json:
{                                       {
  "inbounds": [                           "inbounds": [
    {"type":"mixed","port":12345},          {"type":"mixed","port":20122},
    {"type":"http","port":8080}             {"type":"tun","stack":"gvisor"}
  ],                                      ],
  "route": { "rules": [...],              "log": { "level": "debug" }
    "final": "entry" },                  }
  "experimental": {
    "clash_api": {
      "external_controller": "0.0.0.0:9090"
    }
  }
}
                        ↓ 递归深合并 ↓
结果 (config.json):
{
  "inbounds": [                           ← 数组整体替换
    {"type":"mixed","port":20122},
    {"type":"tun","stack":"gvisor"}
  ],
  "route": { "rules": [...],              ← 对象保留（override 未涉及）
    "final": "entry"
  },
  "experimental": {                       ← 对象递归合并
    "clash_api": {
      "external_controller": "127.0.0.1:9090",  ← 标量覆盖
      "secret": ""                              ← 新增
    }
  },
  "log": { "level": "debug" }            ← 新增
}
```

设计理由：

- 与 Mihomo 的整顶层 key 替换不同，sing-box 需要深度合并，因为用户可能只想覆盖 inbounds 而不丢失 profile 的 route/dns/outbounds。
- 使用标准 sing-box JSON 语法可降低学习门槛，用户查阅 sing-box 文档即可。
- 不依赖 `sing-box merge` 可以避免外部命令的版本兼容问题，且合并逻辑完全由 clashtui 控制。
- 数组整体替换（而非元素级合并）是 GUI.for.SingBox 的一致行为，且语义明确：用户写了哪些 inbound 就是哪些。

## Profile 的管理设计

将 Profile 的信息存放到 clashtui.db, 格式如下:

```yaml
mihomo_cur_profile: my
singbox_cur_profile: johan
mihomo_profiles:
  my:
    dtype: !Url https://example.com
    no_pp: false
  file:
    dtype: !File
    no_pp: false
  template:
    dtype: !Template
    no_pp: false
  common_tpl.yaml.tpl:    # Template type profile name 会以 `.tpl` 作为后缀
    dtype: !Template
      template: common_tpl.yaml
        proxy_provider_group:
          pvd:
            foo_pvd: https://example.com
            bar_pvd: https://example.com
    no_pp: false
singbox_profiles:
  my:
    dtype: !Url https://example.com
    no_pp: false
  file:
    dtype: !File
    no_pp: false
  template:
    dtype: !Template
    no_pp: false
  common_tpl.json.tpl:    # Template type profile name 会以 `.tpl` 作为后缀
    dtype: !Template
      template: common_tpl.json
        proxy_provider_group:
          pvd:
            foo_pvd: https://example.com
            bar_pvd: https://example.com
    no_pp: false
```

根据 profile name 取得 profile_yamls/profile_jsons 内相应的 yaml/json profile:
-   `profiles/<profile_name>.{yaml | json}`

Profile 不能 rename, 用户想要 rename 只能 delete + import, 所以这样管理是可行的。

profiles 目录下的文件是 profile 的原始文件, 不受其他因素影响。比如: `no_pp` option

File/Url Profile 的导入:
-   如果用户输入是一个文件路径, 则 profile type 是 `File`
-   如果是一个 url, 则是 `Url`。

File/Url Profile 的更新:
-   如果是 Url Profile, 则先更新 profile 的内容。
-   确保 profile 的文件存放到了 profiles 目录
-   取得 profiles 的网络资源 (proxy-providers 和 rule-providers), 然后更新到 Core Config Dir 的相应目录。

File/Url Profile 的选择:
-   参考配置合并设计

为什么不使用 api 来更新 Profile:
-   因为通过 api 更新 Profile 并没有返回值 (不知道是否更新成功), 则不知道有哪些东西要更新。
-   所以自己实现更新 Profile 会有比较好的体验。

*Mihomo 的 proxy-providers 和 rule-providers 没有 path 时, 则 path 默认为 `proxies/<url 的 md5 的值>`。ClashTui 需要支持这个设定。*

## Template 的管理设计

概念定义:
-   raw profile: 接近 core config 的格式。比如: file/url profile 就是 raw_profile, 而 template profile 不是 raw profile, 通过它生成的文件才是 raw profile。

因为我比较喜欢将每个 proxy-providers 分组, 而不是混合在一起。所以设计了 Template 的功能。

Mihomo/sing-box template profile 的生成:
-   将 template 的内容和 template_proxy_providers (放在文件的前面) 直接合并。
-   然后将合并后的文件放到 profiles 目录。

    比如:

    ```yaml
    clashtui:
      proxy_provider_groups:
        pvd: # proxy-provider group name
          foo_pvd: https://example.com
          bar_pvd: https://example.com

    # template file content
    ...
    ```

-   clashtui.db 记录:

    ```yaml
      common_tpl.yaml.tpl:
        dtype: !Template
          template: common_tpl.yaml
    ```

Template 文件主要有下面几个信息:
-   生成 proxy-provider groups。比如: pvd {pvd0, pvd1, ...}
-   为每个 proxy-provider 生成一个 proxy-group:

    比如:

    ```yaml
    - name: "At"
      expand_group_with: ["${pvd}"] # 也可以写多个 proxy-provider name, e.g. ["${PPG.pvd.pvd0}", "${PPG.pvd.pvd2}"]
      type: url-test
      <<: *pa_dt
    ```

    会展开为 `At-pvd0, At-pvd1, ...`

-   在 proxy-groups 中使用 proxy-provider groups:
    -   比如: 用 `${pvd}`, 表示使用 proxy-provider group。它会被展开为 `pvd0, pvd1, ...`

Template 的一个关键点是, Template 文件内容不会包含 proxy-provider 的 proxy name, 
所以只需要写上 proxy-provider group name (pvd) 和 proxy-provider name (pvd0, pvd1, ...) 即可知道 Template 要生成什么样的文件了。

综上, 只要提供 proxy-provider name + proxy-provider urls, 则可以生成一个 Profile 文件。

同理, sing-box 也是如此。比如:

为 proxy-provider 扩展 outbounds:

```json
  "outbounds": [
    {
      "type": "urltest",
      "tag": "auto-proxy",
      "expand_outbound_with": ["${PPG.pvd}"], // 也可以写多个 proxy-provider name, e.g. ["${PPG.pvd.pvd0}", "${PPG.pvd.pvd2}"]
      "url": "https://www.gstatic.com/generate_204",
      "interval": "5m",
      "tolerance": 50
    },
  ]
```

proxy-provider 的展开:

```json
  "outbounds": [
    {
      "type": "selector",
      "tag": "select-proxy",
      "outbounds": ["auto-proxy", "${PPG.pvd.pvd0}"],
      "default": "auto-proxy"
    },
  ]
```

因为 sing-box 不支持 proxy-providers, 但是可以用 Template 的功能来替代它:
-   生成 Tempate type profile 时, 将 urls 存放到 profile 中
-   proxy-providers 还有 url 的文件的路径信息, 比如: 放在 `~/.config/clashtui/sing-box/proxy-providers/<url 的 md5 的值>.json`。 
-   有了上面的信息就可以替代 proxy-providers 的功能了。

Template type profile 的生成:
-   前提 proxy-providers 的内容已经更新了, 如果没有内容则更新, 否则不更新。
-   上面的 "template 的生成" 可以知道 Profile 的内容是如何生成的, 将它存放到 profiles 目录 (同理 sing-box 亦如此)
-   生成 clashtui.db 的 profile 信息

Template type profile 的更新:
-   下载 yaml profiles 的 proxy_provider_urls 到 proxy-providers 目录 (选择 profile 就是用这里的文件了)
-   更新 proxy_provider_urls 到相应的路径 (sing-box 是更新到 proxy-providers 目录)
-   不重新生成 template profile。只有 enter template 时, 才重新生成。但是如果用户的当前 profile 是这个 profile, 则要进行选择操作。

Mihomo/sing-box template type pofile 的选择:
-   如果 proxy_provider_urls 有一个没有相应的文件的, 则不用 template profile 生成 raw profile (防止生成格式不正常的 raw profile)
-   根据模板的生成规则使用 template profile 生成的一个 raw profile (这个文件相当于 Url/File 的 profile)。
-   和 File/Url profile 的选择是一样的, 只不过操作的对象是通过 template profile 生成的 raw profile。

*防止写入坏的文件格式, profile 和 proxy-provider 写到文件之前, 需要用 core 测试一下, 成功才写入。(template profile 是测试是使用 template profile 生成的 raw profile)*

## sing-box 的模板例子

```json
{
  "log": {
    "level": "info",
    "timestamp": true
  },
  "dns": {
    "servers": [
      {
        "tag": "dns-remote",
        "address": "https://1.1.1.1/dns-query",
        "address_resolver": "dns-direct",
        "detour": "entry",
        "strategy": "prefer_ipv4"
      },
      {
        "tag": "dns-direct",
        "address": "https://dns.alidns.com/dns-query",
        "address_resolver": "dns-direct",
        "detour": "direct"
      },
      {
        "tag": "dns-local",
        "address": "local",
        "detour": "direct"
      },
      {
        "tag": "dns-fake",
        "address": "fakeip"
      }
    ],
    "rules": [
      {
        "rule_set": ["geosite-geolocation-cn"],
        "server": "dns-direct"
      },
      {
        "rule_set": ["geosite-google"],
        "server": "dns-remote"
      },
      {
        "query_type": ["A", "AAAA"],
        "server": "dns-fake"
      },
      {
        "server": "dns-direct"
      }
    ],
    "final": "dns-direct",
    "strategy": "prefer_ipv4"
  },
  "inbounds": [
    {
      "type": "mixed",
      "tag": "mixed-in",
      "listen": "127.0.0.1",
      "listen_port": 7890
    },
    {
      "type": "tun",
      "tag": "tun-in",
      "address": ["172.19.0.1/30"],
      "mtu": 9000,
      "auto_route": true,
      "strict_route": true,
      "auto_redirect": true,
      "stack": "system"
    }
  ],
  "outbounds": [
    {
      "type": "selector",
      "tag": "entry",
      "outbounds": ["${PGG.auto}", "${PGG.select}", "${PPG.pvd}"] // OR `"outbounds": ["${PGG.auto}", "${PGG.select}", "${PPG.pvd.pvd0}", "${PPG.pvd.pvd2}"],`
    },
    // `"${PG.auto}"` 会扩展为 `auto-pvd0, auto-pvd1, ...`
    {
      "type": "urltest",
      "tag": "auto",
      "expand_group_with": ["${PPG.pvd}"], // OR `"expand_group_with": ["${PPG.pvd.pvd0}", "${PPG.pvd.pvd2}"],`
      "url": "https://www.gstatic.com/generate_204",
      "interval": "5m",
      "tolerance": 50
    },
    // 与上面一组类似
    {
      "type": "urltest",
      "tag": "select",
      "expand_group_with": ["${PPG.pvd}"], // OR `"expand_group_with": ["${PPG.pvd.pvd0}", "${PPG.pvd.pvd2}"],`
      "url": "https://www.gstatic.com/generate_204",
      "interval": "5m",
      "tolerance": 50
    },
    {
      "type": "direct",
      "tag": "direct"
    },
    {
      "type": "block",
      "tag": "block"
    },
    {
      "type": "dns",
      "tag": "dns-out"
    },
    // ===
    // 这里放 proxy-providers 的 type 不为 selector, urltest 之类的 proxies
    // ===
  ],
  "route": {
    "rule_set": [
      {
        "type": "remote",
        "tag": "geoip-cn",
        "format": "binary",
        "url": "https://github.com/SagerNet/sing-geoip/raw/rule-set/geoip-cn.srs",
        "download_detour": "direct",
        "update_interval": "7d"
      },
      {
        "type": "remote",
        "tag": "geosite-geolocation-cn",
        "format": "binary",
        "url": "https://github.com/SagerNet/sing-geosite/raw/rule-set/geosite-geolocation-cn.srs",
        "download_detour": "direct",
        "update_interval": "7d"
      },
      {
        "type": "remote",
        "tag": "geosite-google",
        "format": "binary",
        "url": "https://github.com/SagerNet/sing-geosite/raw/rule-set/geosite-google.srs",
        "download_detour": "direct",
        "update_interval": "7d"
      },
      {
        "type": "remote",
        "tag": "geosite-category-ads-all",
        "format": "binary",
        "url": "https://github.com/SagerNet/sing-geosite/raw/rule-set/geosite-category-ads-all.srs",
        "download_detour": "direct",
        "update_interval": "7d"
      }
    ],
    "rules": [
      {
        "rule_set": ["geosite-category-ads-all"],
        "outbound": "block"
      },
      {
        "rule_set": ["geoip-cn"],
        "outbound": "direct"
      },
      {
        "rule_set": ["geosite-geolocation-cn"],
        "outbound": "direct"
      },
      {
        "rule_set": ["geosite-google"],
        "outbound": "entry"
      },
      {
        "ip_is_private": true,
        "outbound": "direct"
      },
      {
        "protocol": ["bittorrent"],
        "outbound": "direct"
      },
      {
        "outbound": "entry"
      }
    ],
    "auto_detect_interface": true,
    "final": "entry"
  },
  "experimental": {
    "clash_api": {
      "external_controller": "127.0.0.1:9090",
      "external_ui": "dashboard",
      "secret": "",
      "default_mode": "Rule"
    },
    "cache_file": {
      "enabled": true,
      "path": "cache.db",
      "store_fakeip": true
    }
  }
}
```

template_proxy_providers.yaml:
```yaml
pvd:  # proxy-provider group name
  pvd0: https://example.com
  pvd1: https://example.com
```

域:
-   PPG: proxy-provider group
-   PGG: proxy-group group

展开规则:
-   PPG: 展开为 proxies
-   PGG: 展开为 proxy-group(s)

For example: 展开规则
-   "${PPG.pvd}": 展开是 proxies
-   "${PPG.pvd.pvd0}": 展开是 `pvd0` proxy-provider 的 proxies
-   "${PGG.auto}": 展开是 proxy-group groups。比如: `auto-pvd0, auto-pvd1, ...`
-   "${PGG.auto.pvd0}": 代表是一个 proxy-group。e.g. `auto-pvd0`

## Mihomo 的模板例子

Prerequisite: Familiarity with [mihomo configuration](https://wiki.metacubex.one/config/) and YAML syntax.

### Proxy-Providers Template

Purpose: Generates a proxy-provider for each subscription in `template_proxy_providers`.

For example:

```yaml
proxy-anchor:
  - delay_test: &pa_dt {url: https://www.gstatic.com/generate_204, interval: 300}
  - proxy_provider: &pa_pp {interval: 3600, health-check: {enable: true, url: https://www.gstatic.com/generate_204, interval: 300}}

proxy-providers:
  pvd:
    tpl_param:
    type: http    # The type field must be placed here, not in pa_pp. The reason is that ClashTUI uses this field to detect if it is a network resource.
    <<: *pa_pp
```

### Proxy-Groups Template

Purpose: Generates a Proxy-Group for each proxy-provider created by the Proxy-Providers template.

```yaml
proxy-groups:
  - name: "select"
    expand_group_with: ["${PPG.pvd}"]
    type: select

  - name: "auto"
    expand_group_with: ["${PPG.pvd}"]
    type: url-test
    <<: *pa_dt
```

### Using Proxy-Groups Template

Use `${auto}` to enclose the name of the Proxy-Group template to utilize each proxy-group generated by the Proxy-Group template.

For example:

```yaml
proxy-groups:
  - name: "entry"
    type: select
    proxies:
      - ${PGG.auto}
      - ${PGG.select}
```

---

template_proxy_providers.yaml:
```yaml
pvd:  # proxy-provider group name
  pvd0: https://example.com
  pvd1: https://example.com
```

## 解决 Mihomo/sing-box proxy-providers 的 proxy name 同名的方法

步骤:
-   放各个 proxy-provider 放到集合 (Set) 中
-   创建一个临时的集合, 然后将各个 proxy-provider 的 proxies 依次加入
-   如果同名, 则将其改名为 `<origin_name>-<proxy_provider_name>`
-   同时记录一条改名信息: `Set name: [{origin_name, new_name}, ...]`

## 按键冲突检测设计

目前有两层检验：
1. 运行时 — 加载 keymap.yaml 时检测同一 section 内是否有重复 key，有的话只打 log::warn!，不会拒绝配置。
2. 编译期测试 — 验证 mod_agent! 宏定义的默认按键中每个 tab 内部没有重复 key。

所以如果你自定义 keymap.yaml，同一个 section 内写重复 key 会有 warn 日志，但不会阻止启动。

按键冲突了, 则谁排前面就谁有效。keymap.yaml 定义的按键组合优先级比默认的高 。

按键歧义:
-   在同一作用域内, 一个按键组合与另一个按键组合相同, 或者其中一个为另一个的子集。

## api 数据与当前 core 不匹配时

以 `clashtui.db` 中的 `core_type` 为准。若 api 返回的内核数据与配置不符，该数据无效，不得使用——否则用户会因面板展示的数据来源不明而困惑。

### 不匹配场景

- 用户在 clashtui 外部启动了另一内核的 service
- 两个内核恰好监听同一端口，API 请求返回的数据来自错误内核
- CoreSrvCtl 切换内核后未重启 clashtui（当前设计）

### 检测方案

**通过 `/version` 的 `version` 字段识别内核**：

- sing-box 返回 `"version": "sing-box 1.13.11"` → 含 `"sing-box"` 子串
- mihomo 返回 `"version": "v1.18.10"` → 不含

> 注意：sing-box ≥ 1.13 在 clash API 模拟中也返回 `"meta": true`，该字段不可靠。

### 架构

**两层防护**：

| 层 | 位置 | 机制 |
|----|------|------|
| 面板层 | 各 Tab 的 `on_enter` / `after_sync` | 阻止 spawn async task 或清空已有数据 |
| API 层 | `request()` 函数入口 | 统一拒绝非 `/version` 的请求，返回 `"core mismatch"` |

**全局标志**：

`config.rs` 中维护 `static CORE_MISMATCH: AtomicBool`：

- `set_core_mismatch(bool)` — 写入（仅 StatusTab）
- `is_core_mismatch() -> bool` — 读取（所有面板 + `request()`）

**检测时机**：

`StatusTab.on_enter()` 中**同步**调用 `detect_core_type()`（localhost HTTP，<10ms），
在其他面板首次取数据前设置好 `CORE_MISMATCH`。`after_sync` 中继续异步轮询检测（`or_set` 静默）。

**弹窗**：

首次检测到 mismatch 时（`detected_core_type` 从 `None` 变为非匹配值），弹出 Confirm 提示用户。后续不再弹窗。

### 恢复正常

`after_sync` 持续检测，当 `detected == configured` 时自动清空 `CORE_MISMATCH` 标志。用户切换到各面板时正常发起 API 请求，数据恢复展示。

## Support macOS

### macOS Core 文件结构 (launchd)

ClashTui Core 的文件结构设计 (e.g. `/usr/local/opt/clashtui`):

```
.
├── bin
│   └── clashtui -> /usr/local/bin/clashtui
├── mihomo
│   ├── config                        # Core Config Dir
│   │   └── config.yaml               # Core Config Path
│   └── mihomo -> /usr/local/bin/mihomo
└── sing-box
    ├── config                        # Core Config Dir
    │   └── config.json               # Core Config Path
    └── sing-box -> /usr/local/bin/sing-box

launchd plist (独立存放):
  User Mode:   ~/Library/LaunchAgents/clashtui_mihomo.plist
               ~/Library/LaunchAgents/clashtui_singbox.plist
  System Mode: /Library/LaunchDaemons/clashtui_mihomo.plist
               /Library/LaunchDaemons/clashtui_singbox.plist
```

user mode 的 ClashTui Core 的默认路径是 `~/.local/clashtui`。和 Linux 一样。

### systemd vs launchd 对比

| 操作          | Linux (systemd)                          | macOS (launchd)                                    |
|---------------|------------------------------------------|----------------------------------------------------|
| **User Mode** |                                          |                                                    |
| unit 位置     | `~/.config/systemd/user/<name>.service`  | `~/Library/LaunchAgents/<name>.plist`              |
| 启动服务      | `systemctl --user start <name>`          | `launchctl load <plist>` (RunAtLoad 自动启动)       |
| 停止服务      | `systemctl --user stop <name>`           | `launchctl unload <plist>`                         |
| 查看状态      | `systemctl --user is-active <name>`      | `launchctl print gui/$UID/<name>`                  |
| 开机自启      | `systemctl --user enable <name>`         | `launchctl load -w <plist>` (跨重启持久化)          |
| 登出后存活    | `loginctl enable-linger` (支持)           | 不支持 (登出即停止)                                  |
| 崩溃重启      | `systemd service Restart=always`         | plist `KeepAlive=true`                             |
| **System Mode** |                                        |                                                    |
| unit 位置     | `/usr/lib/systemd/system/<name>.service` | `/Library/LaunchDaemons/<name>.plist`              |
| 启动服务      | `sudo systemctl start <name>`            | `sudo launchctl load <plist>` (RunAtLoad 自动启动)  |
| 停止服务      | `sudo systemctl stop <name>`             | `sudo launchctl unload <plist>`                    |
| 查看状态      | `systemctl is-active <name>`             | `sudo launchctl print system/<name>`               |
| 开机自启      | `sudo systemctl enable <name>`           | `sudo launchctl load -w <plist>` (跨重启持久化)     |
| 运行身份      | 专用用户 (mihomo / sing-box)              | root (launchd system daemon)                       |
| TUN 权限      | Linux capabilities (setcap)              | sudo / root 直接运行                                |

关键差异:
- **enable/disable 概念**: systemd 的 `enable` 只设开机自启，`start` 立即启动。launchd 的 `load` 启动服务 (因 plist 中 `RunAtLoad=true`)，`load -w` 同时持久化开机自启。`unload` 停止并从 launchd 移除。`unload -w` 同时标记为禁用。
- **登出行为**: launchd 的 `LaunchAgents` 在用户登出时全部停止，无法通过配置改变。`LaunchDaemons` (system mode) 在 boot 时启动，不受登入/登出影响。
- **TUN 权限**: Linux 用 `setcap` 给二进制加 capability，以非 root 用户运行 TUN。macOS 无此机制，system mode 以 root 运行即可使用 utun 设备。

所以 ClashTui services 在 macOS 下的命令如下:

```sh
# 立即启动 (system mode)
sudo launchctl load /Library/LaunchDaemons/clashtui_mihomo.plist
sudo launchctl load /Library/LaunchDaemons/clashtui_singbox.plist

# 启动 + 开机自启
sudo launchctl load -w /Library/LaunchDaemons/clashtui_mihomo.plist
sudo launchctl load -w /Library/LaunchDaemons/clashtui_singbox.plist

# 停止
sudo launchctl unload /Library/LaunchDaemons/clashtui_mihomo.plist
sudo launchctl unload /Library/LaunchDaemons/clashtui_singbox.plist

# 查看状态
sudo launchctl list | grep clashtui

# User mode (无需 sudo)
launchctl load ~/Library/LaunchAgents/clashtui_mihomo.plist
launchctl unload ~/Library/LaunchAgents/clashtui_mihomo.plist
launchctl load -w ~/Library/LaunchAgents/clashtui_mihomo.plist
```

> `load` 因 plist 中 `RunAtLoad=true`，加载即立即启动服务。
> `load -w` 额外持久化开机自启，重启后自动运行。

### macOS 文件权限

macOS 与 Linux 统一使用 Unix 组权限管理 Core 文件:

| 项 | Linux | macOS |
|---|---|---|
| Core 目录所有者 | `mihomo:mihomo` / `sing-box:sing-box` | `root:admin` |
| 添加用户到组 | `gpasswd -a $USER mihomo` | 不需要 (macOS 用户默认在 `admin` 组) |
| 目录 SGID + group rwx | `chmod g+rwxs` | `chmod g+rwxs` (相同) |
| 配置文件权限 | `chown mihomo:mihomo` + `chmod g+r` | `chmod g+rw` |
| 启动时权限检测修复 | ✅ | ✅ (从 `macos.rs` 桩函数改为真实实现) |

原理: macOS system mode 下 Core 服务以 root 运行 (TUN 需要), 普通用户通过 `admin` 组获取文件读写权限。启动时 ClashTui 检测 Core 目录的 SGID、group 一致性、group 可写性, 不一致则通过 `sudo chmod g+s` / `sudo chown :<group>` / `sudo chmod g+w` 修复。

### 1. User Mode — clashtui_mihomo.plist

路径: `~/Library/LaunchAgents/clashtui_mihomo.plist`

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>clashtui_mihomo</string>
    <key>ProgramArguments</key>
    <array>
        <string>~/.local/clashtui/mihomo/mihomo</string>
        <string>-d</string>
        <string>~/.local/clashtui/mihomo/config</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>~/Library/Logs/clashtui_mihomo.log</string>
    <key>StandardErrorPath</key>
    <string>~/Library/Logs/clashtui_mihomo.log</string>
    <key>WorkingDirectory</key>
    <string>~/.local/clashtui/mihomo/config</string>
</dict>
</plist>
```

### 2. User Mode — clashtui_singbox.plist

路径: `~/Library/LaunchAgents/clashtui_singbox.plist`

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>clashtui_singbox</string>
    <key>ProgramArguments</key>
    <array>
        <string>~/.local/clashtui/sing-box/sing-box</string>
        <string>-D</string>
        <string>~/.local/clashtui/sing-box/config</string>
        <string>-c</string>
        <string>~/.local/clashtui/sing-box/config/config.json</string>
        <string>run</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>~/Library/Logs/clashtui_singbox.log</string>
    <key>StandardErrorPath</key>
    <string>~/Library/Logs/clashtui_singbox.log</string>
    <key>WorkingDirectory</key>
    <string>~/.local/clashtui/sing-box/config</string>
</dict>
</plist>
```

### 3. System Mode — clashtui_mihomo.plist

路径: `/Library/LaunchDaemons/clashtui_mihomo.plist`

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>clashtui_mihomo</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/opt/clashtui/mihomo/mihomo</string>
        <string>-d</string>
        <string>/usr/local/opt/clashtui/mihomo/config</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>/usr/local/var/log/clashtui_mihomo.log</string>
    <key>StandardErrorPath</key>
    <string>/usr/local/var/log/clashtui_mihomo.log</string>
    <key>WorkingDirectory</key>
    <string>/usr/local/opt/clashtui/mihomo/config</string>
</dict>
</plist>
```

### 4. System Mode — clashtui_singbox.plist

路径: `/Library/LaunchDaemons/clashtui_singbox.plist`

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>clashtui_singbox</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/opt/clashtui/sing-box/sing-box</string>
        <string>-D</string>
        <string>/usr/local/opt/clashtui/sing-box/config</string>
        <string>-c</string>
        <string>/usr/local/opt/clashtui/sing-box/config/config.json</string>
        <string>run</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>/usr/local/var/log/clashtui_singbox.log</string>
    <key>StandardErrorPath</key>
    <string>/usr/local/var/log/clashtui_singbox.log</string>
    <key>WorkingDirectory</key>
    <string>/usr/local/opt/clashtui/sing-box/config</string>
</dict>
</plist>
```

## Support Windows

### Windows Core 文件结构

ClashTui Core 的文件结构设计 (System mode, e.g. `C:\Program Files\clashtui`):

```
.
├── bin
│   └── clashtui.exe
├── mihomo
│   ├── config                          # Core Config Dir
│   │   └── config.yaml                 # Core Config Path
│   └── mihomo.exe    # OR a link
└── sing-box
    ├── config                          # Core Config Dir
    │   └── config.json                 # Core Config Path
    └── sing-box.exe  # OR a link
```

User mode 的默认路径是 `%LOCALAPPDATA%\clashtui` (e.g. `C:\Users\<User>\AppData\Local\clashtui`)。

System mode 的默认路径是 `D:\ClashTui`

ClashTui 的配置文件结构同 Linux/macOS, 存放在 `%APPDATA%\clashtui` (e.g. `C:\Users\<User>\AppData\Roaming\clashtui`).

和 Linux/macOS 类似, Windows 也可使用 symlink 指向二进制路径 (`mklink` / `mklink /D`), 但需要 Administrator 权限。如果用户没有管理员权限, 可以直接放置 `.exe` 文件。

### 文件权限

ClashTui core 的文件不要安装在 `C:/Program Files` 和  `C:/Program Files (x86)` 等, 以避免一些文件权限问题。

### Core services 管理 (nssm)

因为一个普通的二进制文件是无法做成一个 Windows service 的, 所以 clashtui 使用一个轻量的方案：[nssm](https://nssm.cc/) (Non-Sucking Service Manager) 来管理 Windows 服务。

**依赖**: 用户需安装 nssm（通过 `scoop install nssm` 或 [nssm.cc/download](https://nssm.cc/download)）。

#### systemd vs launchd vs nssm 对比

| 操作            | Linux (systemd)                          | macOS (launchd)                               | Windows (nssm CLI)                                     |
|-----------------|------------------------------------------|-----------------------------------------------|--------------------------------------------------------|
| **System Mode** |                                          |                                               |                                                        |
| 安装服务        | `sudo systemctl link <unit>`             | `sudo launchctl bootstrap system <plist>`      | `nssm install <name> <exe> <args...>` (需 Administrator)|
| 卸载服务        | `sudo systemctl disable <name>`          | `sudo launchctl bootout system/<name>`         | `nssm remove <name> confirm` (需 Administrator)         |
| 启动服务        | `sudo systemctl start <name>`            | `sudo launchctl bootstrap system <plist>`      | `nssm start <name>` (需 Administrator)                  |
| 停止服务        | `sudo systemctl stop <name>`             | `sudo launchctl bootout system/<name>`         | `nssm stop <name>` (需 Administrator)                   |
| 重启服务        | `sudo systemctl restart <name>`          | 先 bootout 再 bootstrap                         | `nssm restart <name>`                                  |
| 查看状态        | `systemctl is-active <name>`             | `sudo launchctl print system/<name>`           | `nssm status <name>` → 解析 SERVICE_RUNNING/STOPPED     |
| TUN 权限        | `setcap` (Linux capabilities)            | root 运行 (无 setcap)                           | Administrator 运行即可 (LocalSystem 默认)                |

关键差异:
- **nssm 包装器**: nssm 将 mihomo/sing-box 包装为合法 Windows 服务，正确响应 SCM 事件。
- **CLI 调用**: clashtui 通过 `std::process::Command` 调用 `nssm.exe`，与调用 `systemctl` 的方式一致。
- **崩溃重启**: nssm 内置崩溃重启支持，无需额外配置。

ClashTui 会用的 nssm 命令 (下面的需要的信息从 ClashTui config 中取得即可):

```powershell
nssm install <servicename> <app> [<args> ...]
nssm {remove | stop | restart | status} [<servicename>]
```

clashtui 调用 nssm CLI (与 systemctl 类似):

```powershell
# mihomo
nssm install clashtui_mihomo "C:\Program Files\clashtui\mihomo\mihomo.exe" -d "C:\Program Files\clashtui\mihomo\config"

# sing-box
nssm install clashtui_singbox "C:\Program Files\clashtui\sing-box\sing-box.exe" -D "C:\Program Files\clashtui\sing-box\config" -c "C:\Program Files\clashtui\sing-box\config\config.json" run
```

#### CoreSrvCtl 新增操作

由于 nssm 不像 Linux/macOS 那样有 Service 文件, 它只是命令。为了用户方便操作, ClashTui CoreSrvCtl tab 在 Windows 上额外提供以下操作:

**1. Install Srv (安装服务)**

- 调用 nssm CLI: `nssm install <name> <bin_path> <launch_args...>`
- 安装后服务状态变为 `installed`
- nssm 内置崩溃重启和日志管理

**2. Uninstall Srv (卸载服务)**

- 调用 nssm CLI: `nssm remove <name> confirm`
- nssm 自动停止运行中的服务再卸载
- 卸载后服务状态变为 `uninstalled`

**3. Toggle System Proxy (切换系统代理)**

参考 clashtui v0.2.3 的实现, 通过修改 Windows 注册表实现系统代理的开关:

| 接口                  | 操作                                                                |
|-----------------------|--------------------------------------------------------------------|
| 检查系统代理状态       | 读取 `HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings\ProxyEnable` (REG_DWORD): `0` = disabled, `1` = enabled |
| Enable system proxy   | `ProxyEnable` → `1`; `ProxyServer` → `127.0.0.1:<port>`; `ProxyOverride` → `<-loopback>`; 广播 `WM_SETTINGCHANGE` |
| Disable system proxy  | `ProxyEnable` → `0`; 广播 `WM_SETTINGCHANGE`                        |

代理端口从 core 的 mixed 端口获取 (通常是 `7890`, 实际上还是要通过配置取得实际的 external controler), 通过 REST API `GET /configs` 读取 mixed inbound 的 `listen_port`。

实现方式: 通过 `winreg` crate 直接操作注册表, 或调用 `reg.exe` 命令行 (推荐 `winreg` crate 以获得更好的错误处理)。修改注册表后需调用 `SendMessageTimeoutW(HWND_BROADCAST, WM_SETTINGCHANGE, ...)` 通知系统刷新代理设置。

#### CoreSrvCtl 操作列表 (Windows)

CoreSrvCtl tab 的操作列表:

| 操作             | 说明                                           |
|------------------|------------------------------------------------|
| Stop Service     | 停止当前 core 的 service                       |
| Start Service    | 启动当前 core 的 service                       |
| Switch Core      | 切换到另一个 core (mihomo ↔ sing-box)           |
| Stop All         | 停止所有 core services                          |
| Install Srv      | 通过 nssm 安装当前 core 为 Windows Service      |
| Uninstall Srv    | 通过 nssm 卸载当前 core 的 Windows Service       |
| Toggle SysProxy  | 切换系统代理 (enable/disable)                   |

#### 服务状态

| 状态           | 含义                                |
|----------------|-------------------------------------|
| `active`       | 服务正在运行                         |
| `inactive`     | 服务已安装但未运行                    |
| `installed`    | Windows Service 已注册 (但未启动)        |
| `uninstalled`  | 未找到 Windows Service (需要 Install)    |
| `?`            | 无法确定状态 (如权限不足)         |

状态检测优先级:
1. 调用 `nssm status <service_name>` 获取状态字符串
2. `SERVICE_RUNNING` → `"active"`, `SERVICE_STOPPED` / `SERVICE_PAUSED` → `"inactive"`
3. nssm 返回错误 (服务不存在) → `"uninstalled"`

### Install Script

为了降低 Windows 用户的部署门槛, 提供一个 PowerShell 安装脚本 (`install.ps1`) 完成以下操作:

#### 使用方式

```powershell
# 默认安装到 D:\ClashTui
.\install.ps1

# 安装到自定义目录 (Core Directory)
.\install.ps1 -InstallDir "D:\MyTools\ClashTui"  # 检查一下 InstallDir, 路径不允许有空格。同时检查这个目录 ClashTui 是否有权限 (e.g. `C:/Program Files`)
```

**因为 ClashTui 已经支持 install core service 了, 所以 install.ps1 不需要安装 Core service。**
