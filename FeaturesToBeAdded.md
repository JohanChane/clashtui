# 将要添加的功能

## 开发约定

-   尽量不影响 clashtui 的启动速度。
-   从需求出发, 不添加多余的功能。追求精简高效。
-   这些功能在之后不一定会实现。只是记录想法。如果你喜欢某个功能并可以实现的话, 欢迎向我提交 pr。

## 一键更新所有过时的 profiles

需求:

-   有些 profile 存放备份订阅。在其他订阅无法使用的情况下。可以切换到该 profile 使用, 而不至于该 profile 的节点过时。

大概实现思路:

1.  clashtui 配置 (config.toml) 添加 `profile_update_interval` 字段记录 profile 的更新间隔。
2.  检查 profile 最近一次的更新时间 (使用文件修改时间, 不必在文件中记录更新时间, 减少读写文件的操作)。

## 显示汇总的 clashtui 和 mihomo 的信息

需求:

-   主要想查看 geo 文件是否是最新的。

比如以下信息:

-   geo 文件的最近一次更新时间和是否有更新 (refer https://api.github.com/repos/MetaCubeX/meta-rules-dat/releases/latest)。
-   profile 的最近一次的更新时间和是否需要更新 (配合配置中的 `profile_update_interval` 来判断是否需要更新)。
-   内存信息
-   Clash 的版本信息
-   重启 mihomo 服务或检查 profile 的命令。
