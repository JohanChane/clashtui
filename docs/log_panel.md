# Log Panel

## 不需要实现

demotui 不打算实现日志面板。

### 理由

Mihomo 本身作为一个 systemd 服务运行，所有日志通过 journald 管理。`journalctl` 已经提供了远超 TUI 能实现的日志查看体验：

| 能力 | journalctl | TUI 日志面板 |
|------|-----------|-------------|
| 实时跟踪 | `journalctl -fu mihomo` | 需要自己实现 follow |
| 时间范围过滤 | `--since` / `--until` | 需要 UI 组件 |
| 关键词搜索 | `grep` 管道 | 需要搜索框 |
| 分页/跳转 | 原生 less 滚动 | 需要自己实现 |
| 优先级过滤 | `-p err` / `-p warning` | 需要额外 UI |
| 导出 | `-o json` / `> file` | 需要额外功能 |
| 持久化 | 自动（journald 磁盘存储） | 内存缓冲，重启丢失 |
| 历史日志 | 开机以来的全部记录 | 只能看启动后的 |

### 结论

在 TUI 中实现一个日志面板，功能会被 `journalctl` 子集化，且交互体验远不如终端原生的 less 分页器。这属于多余的重复工作。

推荐操作：切到另一个终端窗口运行 `journalctl -fu mihomo`。
