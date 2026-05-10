## Context

demotui 已有一套成熟的 Tab 添加模式：`Tab<C: TabContent>` 泛型容器 + `FutureSet` 事务系统 + `tri!` 错误处理宏 + `mod_agent!` 按键映射宏。已有 StatusTab（状态轮询）、ProxiesTab（代理树）、FileTab（双栏文件管理）。

REST API 层已有 `restful::connection` 模块，实现了 `ConnInfo`/`Conn`/`ConnMetaData` 反序列化 struct 以及 `get_connections()` (HTTP GET) 和 `terminate_connection()` (HTTP DELETE) 函数。

本次新增 ConnectionsTab，需要用表格形式展示实时连接数据，支持关闭连接操作。复用现有 `Tab<C>` 容器 + `FutureSet` 事务模式。

## Goals / Non-Goals

**Goals:**
- 实现 ConnectionsTab（Tab 位置 4，数字键 `4`），以表格展示 Mihomo 活动连接
- 显示连接关键信息：目标主机、匹配规则、代理链、上行/下行流量和速率
- 支持关闭单个连接和全部连接（DELETE /connections/:id 和 DELETE /connections）
- 1 秒周期自动刷新（仅 Tab 活跃时），因为连接变化比代理更频繁
- 复用/扩展现有 `restful::connection` 模块，补充缺失字段
- 遵循现有 `TabContent` + `FutureSet` + `tri!` + `mod_agent!` 模式

**Non-Goals:**
- 不使用 WebSocket 流式推送（v1 使用 HTTP 轮询，足够满足需求）
- 不支持连接筛选/搜索（v1 省略，后续可扩展）
- 不支持分页（v1 依赖终端滚动，后续视量级扩展）
- 不支持连接捕获模式（keep closed connections visible）
- 不实现连接详情弹窗（v1 信息已在表格列中展示）

## Decisions

### 1. Tab 类型：单面板 Tab<Connections>，位置 4

**决策**：使用 `Tab<Connections>` 单面板，放置于 Tab 数组索引 3（数字键 `4` 切换），即 Status(1) → File(2) → Proxies(3) → Connections(4)。

连接数据天然适合表格展示——多列字段（主机、规则、代理链、流量）需要对齐排列。使用 ratatui Table 组件。

### 2. 数据获取：HTTP 轮询，1 秒间隔

**决策**：HTTP GET `/connections` 每 1 秒轮询一次。

**理由**：
- WebSocket 需要额外的 `tokio-tungstenite` 依赖，增加复杂度
- `ConnectionsTab` 不需要与 UI 帧同步；1 秒刷新对 TUI 表格来说足够实时
- 复用已有 `get_connections()` HTTP 函数
- 参考：ProxiesTab 使用 5 秒轮询；连接需要更频繁（1 秒）因为变化更快

**替代方案**：WebSocket 推送。更实时但需要额外依赖管理、连接生命周期管理、背压处理。v1 保留 HTTP 轮询，后续若用户反馈需要更实时可迁移。

### 3. 数据模型：直接使用 API struct

**决策**：直接复用 `restful::connection` 中的 `ConnInfo`/`Conn`，额外补充缺失字段。

现有 struct 缺少 `rule`/`rulePayload`（规则匹配信息）和 metadata 中的 `destinationIP`/`sniffHost`。需要扩展：

```rust
// Conn 新增字段
pub struct Conn {
    // ... existing fields ...
    pub rule: Option<String>,
    #[serde(rename = "rulePayload")]
    pub rule_payload: Option<String>,
}

// ConnMetaData 新增字段
pub struct ConnMetaData {
    // ... existing fields ...
    #[serde(rename = "destinationIP")]
    pub destination_ip: Option<String>,
    #[serde(rename = "sniffHost")]
    pub sniff_host: Option<String>,
}
```

**理由**：`Connections` 不需要额外的本地数据结构（不像 `ProxyTree`），因为连接是扁平列表，直接使用 Vec<Conn> 渲染。

### 4. 表格列设计

**决策**：7 列，按优先级排列：

| 列 | 宽度约束 | 字段来源 | 说明 |
|---|---------|---------|------|
| Host | Min(30) | `metadata.host` + `:destination_port` 或 `destination_ip:destination_port` | 目标地址：端口 |
| Rule | Max(15) | `rule` | 匹配的规则类型 |
| Chains | Min(15) | `chains` (反序，用 `>` 连接) | 代理链 |
| Download | Max(12) | `download` | 累计下载（人类可读） |
| Upload | Max(12) | `upload` | 累计上传（人类可读） |
| DL Speed | Max(8) | `downloadSpeed`（UI 计算） | 下载速率 |
| UL Speed | Max(8) | `uploadSpeed`（UI 计算） | 上传速率 |

默认宽度约束用 ratatui 的 `Constraint::Min`/`Constraint::Max` 混合，配合剩余空间自适应。

**替代方案**：少列（仅 Host + Chains + DL/UL）。信息量不够，用户看不到规则匹配详情。保留这些列且用可调宽度约束确保终端适配。

### 5. 刷新策略：1 秒定期轮询

**决策**：`after_sync` 实现 1 秒周期性自动刷新（类似 StatusTab/ProxiesTab），仅在 Tab 活跃时。

- 自动刷新间隔：1 秒
- 关闭连接后立即触发刷新
- 使用 `tri!` 宏：网络错误用 `or_set` 设置 error 字段显示

### 6. 按键映射设计

**决策**：复用项目 Vim-like 风格，使用 chord 多键快捷键：

| 按键 | 类型 | 动作 |
|------|------|------|
| `j`/`↓` | 单键 | 下移 |
| `k`/`↑` | 单键 | 上移 |
| `gg` | chord | 跳到第一个 |
| `G` | 单键 | 跳到最后一个 |
| `dd` | chord | 关闭当前选中的连接（含确认弹窗 AskConfirm） |
| `ac` | chord | 关闭所有连接（含确认弹窗） |
| `sd` | chord | 按下载速度排序 |
| `su` | chord | 按上传速度排序 |
| `sr` | chord | 排序还原为原始顺序 |

- `gg`/`G`：单键 `g` 触发 chord 模式（等待第二个键），`G`（大写）直接触发
- `dd`：双击 `d`，避免单键 `d` 误触关闭连接
- `ac`："all close" 缩写，关闭全部连接
- `sd`/`su`/`sr`：排序操作，"sort download/sort upload/sort reset"
- Chords 通过 `mod_agent!` 的 `[KeyCode::Char('g'), KeyCode::Char('g')]` 语法注册为 `dispatch_shortcut` 路由，`G` 为单键通过 `TryFrom<&KeyEvent>` 路由

### 7. UI 布局

表格列名（从左到右）：

| 列宽 | 列名 | 字段来源 |
|------|------|---------|
| Min(30) | Host | `host:destination_port` 或 `destination_ip:destination_port` |
| Max(15) | Rule | `rule` |
| Min(15) | Chains | `chains` 反序 `>` 连接 |
| Max(10) | Download | `download`（人类可读字节） |
| Max(10) | Upload | `upload`（人类可读字节） |
| Max(10) | DL Speed | 本地计算速率（人类可读字节/s） |
| Max(10) | UL Speed | 本地计算速率（人类可读字节/s） |

```
┌ Clashtui ──────────────────────────── Tab or num ─┐
│ 1 Status │ 2 File  │ 3 Proxies │▐ 4 Connections ▐│
├────────────────────────────────────────────────────┤
│ Host                      Rule   Chains   Down  ...│
│ api.example.com:443       DIRECT Proxy>X  1.2M ...│
│ cdn.cloudflare.net:443    DIRECT DIRECT  340K  ...│
│ ...                                                │
└────────────────────────────────────────────────────┘
```

- 第一行：Tab bar（保留项目现有布局）
- 表头行：列名（Host, Rule, Chains, Download, Upload, DL Speed, UL Speed）
- 数据行：选中的行用高亮标记
- 排序后排序列标题后加 `▼`（降序）或 `▲`（升序）标记
- 有 error 时以红色文本显示在表格上方
- 总连接数 + 当前排序状态显示在表格下方或 PopUp 区域

### 8. 排序

**决策**：支持按下载速度、上传速度排序，以及还原为原始顺序。

- 默认顺序：API 返回顺序（通常是连接建立时间）
- `sd`（chord `s` 然后 `d`）：按下载速度降序排列
- `su`（chord `s` 然后 `u`）：按上传速度降序排列
- `sr`（chord `s` 然后 `r`）：还原为原始顺序
- 排序状态用 `SortState` enum 管理：`Default` / `ByDownload` / `ByUpload`
- 排序时表头相应列名后显示 `▼` 标记表示当前排序方向

**替代方案**：点击列头切换排序（如 metacubexd）。TUI 不适用鼠标点击，用快捷键更直接。

### 9. 表格列名（国际化）

**决策**：列名使用英文缩写，与数据内容对应：

| 显示名 | 含义 |
|--------|------|
| `Host` | 目标主机:端口 |
| `Rule` | 匹配规则 |
| `Chains` | 代理链 |
| `Download` | 累计下载 |
| `Upload` | 累计上传 |
| `DL Speed` | 下载速率（1s 差值） |
| `UL Speed` | 上传速率（1s 差值） |

### 10. 速率计算：基于差值

**决策**：本地维护上一轮 `upload/download` 值，通过差值估算速率。UI struct `Connections` 持有 `last_bytes: HashMap<String, (u64, u64)>`，每轮刷新后计算每个连接的速率并存入 display 列表。

速率仅在 UI 层计算和存储，不修改 `restful::connection::Conn` struct。

## Risks / Trade-offs

- **1 秒轮询在高连接数时流量较大** → 连接通常 < 200 条，JSON 响应 < 100KB，1 秒轮询 ≈ 100KB/s，可接受
- **HTTP 轮询有 1 秒延迟** → 对 TUI 面板可接受，不像 Web 仪表盘那样需要实时性
- **删除操作无恢复** → 确认弹窗降低误操作风险
- **rate 计算首次为 0** → 首次刷新时无历史数据，速率显示 `-` 或 0；第二次刷新开始正常
- **大屏终端列空间不均衡** → 使用 Min/Max 约束 + Fill 自适应，确保基本信息始终可读

## Open Questions

（无）
