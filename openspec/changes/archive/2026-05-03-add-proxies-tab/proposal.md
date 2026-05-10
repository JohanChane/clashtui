## Why

demotui 目前只有 StatusTab 和 FileTab，用户无法查看和切换 Mihomo 代理节点。添加 ProxiesTab（Tab 位置 3）让用户以文件浏览器的方式浏览代理树——策略组就是文件夹，叶子节点就是文件——查看延迟、切换节点，这是 TUI 客户端最基本的功能之一。

## What Changes

- 新增 ProxiesTab 在 Tab 位置 3（数字键 `3` 切换）
- 新增 `src/functions/restful/proxies.rs` — 封装 `/proxies`、`/proxies/<name>` (GET/PUT)、`/proxies/<name>/delay`、`/group` API
- 新增 `Proxy`、`ProxyGroup` 等响应反序列化 struct
- 新增 `ProxiesTab`（`Tab<Proxies>`）— 单面板 Tab，以文件浏览器方式展示代理树
- 新增 `ProxyTree` 本地数据结构 — 从扁平 API 构建树形展示（文件夹/文件隐喻）
- 注册 ProxiesTab 到 Tab enum 和 app
- 支持按键：导航（↑↓jk）、展开/折叠文件夹（Enter/→/l）、选择节点（Enter on file）、折叠父文件夹（`u` on file）、测速（t）、搜索（/）
- 支持延迟内联显示（列表项前显示延迟 ms 或不显示）
- 注册到按键映射系统

## Capabilities

### New Capabilities

- `proxy-data`: 通过 Mihomo REST API 获取代理节点数据，构建本地树形结构用于 TUI 展示
- `proxy-selection`: 为 Selector 类型代理组切换选定节点
- `proxy-speedtest`: 对单个代理或策略组内所有代理进行延迟测试

### Modified Capabilities

（无）

## Impact

- 新增文件：`src/functions/restful/proxies.rs`（REST API 封装）、`src/tui/tab/proxies.rs`（ProxiesTab 实现）
- 修改文件：`src/functions/restful.rs`（注册 proxies 子模块）、`src/tui/tab/mod.rs`（注册 ProxiesTab + enum_dispatch）、`src/tui/app.rs`（添加 ProxiesTab 到 tabs）
- 依赖：仅使用已有的 `minreq`、`serde`、`ratatui`，无需新依赖
