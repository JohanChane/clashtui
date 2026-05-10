## Why

demotui 无法查看当前 Mihomo 的活动连接，用户无法了解流量经过哪个代理链、哪个规则匹配了请求，也无法主动关闭异常连接。连接面板是 Mihomo TUI 客户端的基本功能，让用户实时监控网络流量。

## What Changes

- 新增 ConnectionsTab 在 Tab 位置 4（数字键 `4` 切换）
- 扩展已有 `src/functions/restful.rs` 中的 `Conn`/`ConnMetaData` struct，补充 `rule`/`rulePayload`/`destinationIP`/`sniffHost` 字段
- 新增 `src/tui/tab/connections.rs` — 实现 `Connections` (TabContent)，以表格形式展示连接
- 表格列：Host（目标主机:端口）、Rule（匹配规则）、Chains（代理链）、Download/Upload（累计流量）、DL Speed/UL Speed（速率）
- 支持按键：导航（↑↓jk）、滚动（PgUp/PgDn/g/G）、关闭连接（d/D）、进入连接详情（Enter）
- 1 秒自动轮询刷新（仅 Tab 活跃时）
- 注册到 Tab enum 和 app

## Capabilities

### New Capabilities

- `connection-monitoring`: 通过 Mihomo REST API 获取活动连接列表，实时展示连接状态、流量和代理链
- `connection-management`: 通过 API 关闭单个或全部连接

### Modified Capabilities

（无）

## Impact

- 修改文件：`src/functions/restful.rs`（扩展 Conn/ConnMetaData 字段）、`src/tui/tab/mod.rs`（注册 ConnectionsTab）、`src/tui/app.rs`（添加 tab + 更新 TAB_COUNT）
- 新增文件：`src/tui/tab/connections.rs`（ConnectionsTab 实现）
- 依赖：仅使用已有的 `minreq`、`serde`、`ratatui`，无需新依赖
