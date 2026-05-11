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

ClashTui 配置的文件结构:

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

ClashTui Core 的文件结构设计:

```
.
├── mihomo
│   ├── clashtui_mihomo.service       # Mihomo Core 的 systemd unit file
│   ├── config                        # Core Config Dir
│   │   ├── config.yaml               # Core Config Path
│   └── mihomo -> /usr/bin/mihomo
└── sing-box
    ├── clashtui_singbox.service
    ├── config                        # Core Config Dir
    │   ├── config.json               # Core Config Path
    └── sing-box -> /usr/bin/sing-box
```

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
  edit_cmd: kitty -e nvim "%s"
  open_dir_cmd: kitty -e yazi "%s"
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

sing-box 的合并由 demotui 自行实现递归深合并，不再依赖外部 `sing-box merge` 命令。

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
- 不依赖 `sing-box merge` 可以避免外部命令的版本兼容问题，且合并逻辑完全由 demotui 控制。
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

*Mihomo 的 proxy-providers 和 rule-providers 没有 path 时, 则 path 会被设置为 `<url 的 md5 的值>.yaml`。ClashTui 需要支持这个设定。*

## Template 的管理设计

因为我比较喜欢将每个 proxy-providers 分组, 而不是混合在一起。所以设计了 Template 的功能。

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
-   生成 Tempate type profile 时, 将 urls 存放到 clashtui.db 的 profile 字段中:
    
    比如:
    ```yaml
    mihomo_profiles:
      common_tpl.json.tpl:
        dtype: !Template
          template: singbox_common_tpl.json
          proxy_provider_group:
            pvd:
              foo_pvd: https://example.com
              bar_pvd: https://example.com
    ```

    template_proxy_providers.yaml:
    ```yaml
    pvd:  # proxy-provider group name
      foo_pvd: https://example.com
      bar_pvd: https://example.com
    ```

-   proxy-providers 还有 url 的文件的路径信息, 比如: 放在 `~/.config/clashtui/sing-box/proxy-providers/<url 的 md5 的值>.yaml`。 
-   有了上面的信息就可以替代 proxy-providers 的功能了。

Template type profile 的生成:
-   前提 proxy-providers 的内容已经更新了, 如果没有内容则更新, 否则不更新。
-   上面的描述可以知道 Profile 的内容是如何生成的, 将它存放到 profiles 目录 (同理 sing-box 亦如此)
-   生成 clashtui.db 的 profile 信息

Template type profile 的更新:
-   下载 clashtui.db 的 proxy_provider_urls 到 proxy-providers 目录 (选择 profile 就是用这里的文件了)
-   如果 proxy_provider_urls 有一个没有更新成功, 则不生成 profile 文件 (防止生成格式不正常的 profile)
-   为了 mihomo 和 sing-box 的统一, 重新生成 profile。

Mihomo template type pofile 的选择:
-   和 File/Url profile 的选择是一样的

sing-box template type pofile 的选择:
-   和 File/Url profile 的选择是一样的

*防止定入坏的文件格式, profile 和 proxy-provider 写到文件之前, 需要用 core 测试一下, 成功才写入。*

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
