# ClashTUI Usage

## ClashTUI 的配置

配置文件的路径是 `~/.config/clashtui/config.yaml`.

```yaml
# 下面参数对应命令 <clash_core_path> -d <clash_cfg_dir> -f <clash_cfg_path>
basic:
  clash_config_dir: '/opt/clashtui/mihomo_config'
  clash_bin_path: '/usr/bin/mihomo'
  clash_config_path: '/opt/clashtui/config.yaml'
  timeout: null                     # 模拟 clash_ua 下载的超时时间。`null` 表示没有超时时间。单位是`秒`。
service:
  clash_srv_name: 'mihomo'          # systemctl {restart | stop} <clash_srv_name>
  is_user: false                    # true: systemctl --user ...
extra:
  edit_cmd: 'alacritty -e nvim "%s"'          # `%s` 会被替换为相应的文件路径。如果为空, 则使用 `xdg-open` 打开文件。
  open_dir_cmd: 'alacritty -e ranger "%s"'
```

## 快捷方式

按 `?` 显示 help。

## 便携模式

在 clashtui 程序所在的目录创建一个名为 `data` 的文件夹。则会将数据放在 `data` 内而不是 `~/.config/clashtui`。

## 结合 cronie 定时更新 profiles

```sh
clashtui -u         # 以命令行的模式更新所有 profiles。如果 profile 有 proxy-providers, 同时也会更新它们。
```

所以可以结合 cronie 来定时更新 profiles:

```sh
# crontab -e
0 10,14,16,22 * * * /usr/bin/env clashtui -u >> ~/cron.out 2>&1
```

cronie 的使用, See [ref](https://wiki.archlinuxcn.org/wiki/Cron)。

## ClashTUI 文件结构

`~/.config/clahstui`:
-   basic_clash_config.yaml: 存放 mihomo 配置的基础字段, 这些字段会合并到 `clash_cfg_path`。
-   config.yaml: clashtui 程序的配置。
-   templates/template_proxy_providers: 存放模板使用的代理订阅。

clash_config_path: mihomo 最终使用的配置。

mihomo 配置的基础字段: 除了这些字段 "proxy-groups"、"proxy-providers"、"proxies"、"sub-rules"、"rules" 和 "rule-providers" 都是基础字段。

## Template

前提已经掌握 [mihomo 的配置](https://wiki.metacubex.one/config/)和 yaml 的语法。

### Proxy-Providers Template

作用: 为 `template_proxy_providers` 中的每个订阅生成一个 proxy-provider。

For example:

```yaml
proxy-anchor:
  - delay_test: &pa_dt {url: https://www.gstatic.com/generate_204, interval: 300}
  - proxy_provider: &pa_pp {interval: 3600, health-check: {enable: true, url: https://www.gstatic.com/generate_204, interval: 300}}

proxy-providers:
  provider:
    tpl_param:
    type: http    # type 字段要放在此处, 不能放入 pa_pp。原因是 clashtui 根据这个字段检测是否是网络资源。
    <<: *pa_pp
```

### Proxy-Groups Template

作用: 为 Proxy-Providers template 生成的每个 proxy-provider 都生成一个 Proxy-Group.

```yaml
proxy-groups:
  - name: "Select"
    tpl_param:
      providers: ["provider"]
    type: select

  - name: "Auto"
    tpl_param:
      providers: ["provider"]
    type: url-test
    <<: *pa_dt
```

### Using Proxy-Groups Template

使用 `<>` 包含 Proxy-Group template 的名称即可使用 Proxy-Group template 生成的每个 proxy-group.

For example:

```yaml
proxy-groups:
  - name: "Entry"
    type: select
    proxies:
      - <Auto>
      - <Select>
```

---

你可以在此找到最新的模板。See [ref](https://github.com/JohanChane/clashtui/tree/main/InstallRes/templates).
