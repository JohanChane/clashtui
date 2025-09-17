# ClashTUI Usage

## ClashTUI 的配置

配置文件的路径是 `%APPDATA%/clashtui/config.yaml`.

```yaml
# 下面参数对应命令 <clash_core_path> -d <clash_cfg_dir> -f <clash_cfg_path>
clash_core_path: "D:/ClashTUI/mihomo.exe"
clash_cfg_dir: "D:/ClashTUI/mihomo_config"
clash_cfg_path: "D:/ClashTUI/mihomo_config/config.yaml"
clash_srv_name: "mihomo"                          # nssm {install | remove | restart | stop | edit} <clash_srv_name>
edit_cmd: 'notepad "%s"'                          # `%s` 会被替换为相应的文件路径。如果为空, 则使用默认的方式打开文件。
open_dir_cmd: 'explorer "%s"'                     # 与 `edit_cmd` 同理
```

## 其他

参考 [ref](../clashtui_usage_zh.md)
