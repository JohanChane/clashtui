# ClashTUI Usage

## Configuration of ClashTUI

The configuration file path is `%APPDATA%/clashtui/config.yaml`.

```yaml
# The parameters below correspond to the command <clash_core_path> -d <clash_cfg_dir> -f <clash_cfg_path>
clash_core_path: "D:/ClashTUI/mihomo.exe"
clash_cfg_dir: "D:/ClashTUI/mihomo_config"
clash_cfg_path: "D:/ClashTUI/mihomo_config/config.yaml"
clash_srv_name: "mihomo"                          # nssm {install | remove | restart | stop | edit} <clash_srv_name>
edit_cmd: 'notepad "%s"'                          # `%s` will be replaced with the corresponding file path. If empty, the file will be opened using the default method.
open_dir_cmd: 'explorer "%s"'                     # Same principle as `edit_cmd`
```

## Others

Reference [ref](../clashtui_usage.md)
