# ProxiesTab — API 调研与实现

## 一、Mihomo API

| 端点 | 方法 | 用途 |
|------|------|------|
| `/proxies` | GET | 获取所有代理信息（含策略组与节点） |
| `/proxies/<name>` | GET | 获取单个代理详情 |
| `/proxies/<name>` | PUT | 为策略组选择节点 (`{"name":"节点名"}`) |
| `/proxies/<name>/delay` | GET | 对指定代理测速 (`?url=xxx&timeout=5000`) |
| `/group` | GET | 获取策略组信息 |
| `/group/<name>/delay` | GET | 对策略组内所有节点批量测速 |

base URL = `config.external_controller`（如 `http://127.0.0.1:9090`），认证 `Authorization: Bearer ${secret}`。

### 响应结构

```jsonc
{
  "proxies": {                        // 扁平 map，key = name
    "Entry": {
      "type": "Selector",
      "all": ["vmess-xxx", "DIRECT"], // 子节点引用 (DAG)
      "now": "vmess-xxx",             // 当前选中
      "history": [{"time": "...", "delay": 191}]
    },
    "vmess-xxx": {
      "type": "Vmess",
      "history": [{"time": "...", "delay": 191}]
    }
  }
}
```

`all` 字段形成 DAG 引用图。`hidden` 字段在 DIRECT/REJECT 等内置代理中缺失，需 `#[serde(default)]`。

### REST 封装

```rust
// src/functions/restful/proxies.rs
pub fn fetch_proxies()          -> Result<ProxiesResponse>  // GET  /proxies
pub fn select_proxy(g, n)       -> Result<()>               // PUT  /proxies/<g>  {"name": n}
pub fn test_proxy_delay(n,u,t)  -> Result<u64>              // GET  /proxies/<n>/delay
pub fn test_group_delay(n,u,t)  -> Result<()>               // GET  /group/<n>/delay
```

---

## 二、设计理念

NERDTree 风格文件浏览器 —— 策略组是文件夹，节点是文件/链接。

**核心决策：**

1. **mod_agent! 按键系统** — 使用项目标准的 `mod_agent!` 宏，与 Profile/Template 一致。单键直发，多键 chord 弹 Which 面板。
2. **扁平 Vec + name_index** — 渲染顺序即存储顺序，O(1) 名字查找
3. **三种节点类型** — Folder（真实目录）、Link（交叉引用）、File（叶子节点）
4. **多组可同时展开** — 不像 clashctl 只展开一组，NERDTree 风格允许多目录展开
5. **快捷键挂 `Tab.shortcuts` 字段** — 构造时计算，避免泛型 static 跨类型共享的 bug

---

## 三、架构

```
┌─────────────────────────────────────────────────────────────┐
│  ProxiesTab                                                  │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  Proxies { tree, proxies, error }                    │    │
│  │  ┌─────────────────────────────────────────────┐    │    │
│  │  │  ProxyTree                                   │    │    │
│  │  │  nodes: Vec<NodeItem>    // 展平渲染顺序     │    │    │
│  │  │  name_index: HashMap     // name → idx      │    │    │
│  │  └─────────────────────────────────────────────┘    │    │
│  └─────────────────────────────────────────────────────┘    │
│                                                              │
│  render 直接遍历 nodes:                                       │
│   ▶ GLOBAL     [Selector]                                    │
│       vmess-1              ← Link（浅绿），l 跳转到 Folder    │
│       ss-2                 ← Link                           │
│   ▶ Entry      [Selector]                                    │
│     * vmess-2  [Vmess]     ← now，Enter 选择                 │
│       DIRECT               ← Link（跳到 DIRECT Folder）       │
│   ▶ DIRECT     [Direct]                                      │
│                                                              │
│  光标 = ListState.selected() → flat index → node_at(idx)    │
└─────────────────────────────────────────────────────────────┘
```

---

## 四、数据结构

### 4.1 NodeItem

```rust
struct NodeItem {
    name: String,               // proxy 名称
    depth: usize,               // 缩进层级 (0 = 顶层)
    node_type: NodeType,        // Folder | Link | File
    proxy_type: String,         // "Selector" | "Vmess" | ...
    delay: Option<u64>,         // 延迟 (ms)
    parent: Option<String>,     // parent name (h 键回上层)
    expanded: bool,             // Folder 是否展开
    is_now: bool,               // 是否当前选中 (显示 *)
}
```

### 4.2 NodeType

| 类型 | 含义 | 前缀 | `l` 键 | Enter 键 |
|------|------|------|--------|----------|
| `Folder` | 真实目录位置 | `▶`/`▼` | 展开子节点 | toggle 展开/折叠 |
| `Link` | 指向 Folder 的引用 | ` ` /`*` | 跳到目标 Folder | PUT 选择 (始终) |
| `File` | 叶子节点 | ` `/`*` | — | PUT 选择 (始终) |

颜色区分：Folder 用 `tab_focused`，Link 用浅绿色 `Rgb(100,180,150)`，File 用默认颜色。

Link/File 的 Enter 一律走 `select_proxy(parent, name)`，无 Selector 类型限制。

### 4.3 ProxyTree

```rust
struct ProxyTree {
    nodes: Vec<NodeItem>,                  // 展平，顺序即渲染顺序
    name_index: HashMap<String, usize>,    // name → nodes[idx]
    sorted: bool,                          // true = 按字母排序
}
```

关键方法：
- `build(response)` → 从 ProxiesResponse 构建
- `rebuild_from_proxies(proxies)` → 保留展开状态重建
- `toggle_expand_at(name)` → 切换 Folder 展开/折叠
- `expand_at(name)` / `collapse_at(name)` → 展开/折叠指定 Folder
- `collapse_all()` / `expand_all()` → 全部折叠/展开
- `find_folder_index(name)` → 查找 Folder 的索引（线性扫描，不受 Link 干扰）
- `node_at(idx)` → 按索引获取节点

### 4.4 构建逻辑

```
build():
  1. 从 proxies map 筛选顶层：非 hidden 且 all 非空的策略组
  2. 排序：
      默认（sorted=false），按 GLOBAL.all 顺序排列，GLOBAL 放最后
      按 a s 后（sorted=true），按字母顺序排列
  3. 对每个顶层组调用 push_entry():
     a. 生成 Folder (深度 0)
     b. 若 expanded: 遍历 all，子项是策略组 → Link，否则 → File (深度+1)
  4. 重建 name_index

注意：`rebuild_index` 为所有节点建索引，当同名 Folder 和 Link（如 GLOBAL 的子项）同时存在时，Link 会覆盖 Folder 条目。因此 `expand_at`、`toggle_expand_at`、`collapse_at` 使用 `find_folder_index` 线性扫描而非 `name_index` 查找。
```

---

## 五、按键系统

使用 `mod_agent!` 宏定义，与 Profile/Template 统一模式。单键直接 dispatch，多键 chord 弹出 Which 面板。

```rust
mod_agent!(
    Key,
    [
        // 单键
        ([KeyCode::Up],                    Key::MoveUp,      ""),
        ([KeyCode::Down],                  Key::MoveDown,    ""),
        ([KeyCode::Char('k')],             Key::MoveUp,      ""),
        ([KeyCode::Char('j')],             Key::MoveDown,    ""),
        ([KeyCode::Char('h')],             Key::Parent,      ""),
        ([KeyCode::Char('l')],             Key::Expand,      ""),
        ([KeyCode::Enter],                 Key::Select,      ""),
        ([KeyCode::Char('t')],             Key::TestDelay,   "Test delay"),
        // 多键 chord（a 前缀）
        ([KeyCode::Char('a'), KeyCode::Char('s')], Key::ToggleSort,   "Toggle sort"),
        ([KeyCode::Char('a'), KeyCode::Char('f')], Key::CollapseAll,  "Collapse all"),
        ([KeyCode::Char('a'), KeyCode::Char('e')], Key::ExpandAll,    "Expand all"),
        ([KeyCode::Char('a'), KeyCode::Char('t')], Key::TestAllDelay, "Test all delay"),
    ]
);
```

### 按键行为

| 键 | 动作 | 行为 |
|----|------|------|
| `j` / `↓` | MoveDown | 光标下移 |
| `k` / `↑` | MoveUp | 光标上移 |
| `h` | Parent | Folder: 折叠自身 / Link/File: 折叠父目录并跳转 |
| `l` | Expand | Folder: 展开 / Link: 跳到目标 Folder / File: 无操作 |
| `Enter` | Select | Folder: toggle 展开 / Link: PUT 选择 / File: PUT 选择 |
| `t` | TestDelay | Folder: 对组内所有节点测速 / Link/File: 对单节点测速 |
| `a t` | TestAllDelay | 对全部节点批量测速 |
| `a s` | ToggleSort | 开关排序（字母顺序 ↔ GLOBAL.all 顺序） |
| `a f` | CollapseAll | 折叠全部 Folder |
| `a e` | ExpandAll | 展开全部 Folder |

### Which 面板

多键 chord（`a s`、`a f`、`a e`）按 `a` 时弹出 Which 面板：

```
┌ Which? ───────────┐
│  s  Toggle sort    │
│  f  Collapse all   │
│  e  Expand all     │
│  t  Test all delay │
└───────────────────┘
```

后续按键过滤候选：精确匹配 → dispatch，0 候选 → 关闭并消费，Esc → 关闭。

### Key enum 映射

```rust
#[derive(Clone, Copy)]
enum Key {
    MoveUp, MoveDown, Parent, Expand, Select,
    TestDelay, TestAllDelay,
    ToggleSort, CollapseAll, ExpandAll,
}

impl TryFrom<&KeyEvent> for Key {
    fn try_from(ev: &KeyEvent) -> Result<Self, Self::Error> {
        agent().get(ev).map(|act| *act).ok_or(())
    }
}
```

仅单键存在于 `agent()` HashMap 中；chord 键（`a`、`s`、`f`、`e`、`t`）不由 TryFrom 处理，走 ChordHandler Which 路由。

### 延时测试

`t` 键根据选中节点类型分发：

| 节点类型 | `t` 行为 | 实现 |
|---------|---------|------|
| Folder | 对组内所有节点批量测速 | `test_group_delay(name, url, timeout)` → 等 2s → re-fetch |
| File | 对单节点测速 | `test_proxy_delay(name, url, timeout)` → re-fetch |
| Link | 对 Link 指向的节点测速 | 同上，target = Link 的 name |

`a t` 遍历所有 Folder 和顶层 File，顺序调用测速 API，完成后 re-fetch 刷新全树。

测试 URL：优先使用 proxy 自身的 `test_url`（来自 API 响应）。若无 `test_url`，则不传 `url` 参数，由 Mihomo 使用该节点配置的 `test-url`（或内核默认 `https://www.gstatic.com/generate_204`）。超时取 `config.timeout`（默认 5s）。

测试期间 `Proxies.error` 字段显示状态信息（`"Testing {name}..."` 或 `"Testing all (N groups/nodes)..."`），完成后清除。

---

## 六、ChordHandler 路由

```
KeyEvent
   │
   ▼
  PopUp 层 (0) ──── 有弹窗就阻断
   │
   ▼
  ChordHandler (1) ──► check_init(kv, tab.shortcuts(), dispatch)
   │ 是                    │ 否
   ▼                        ▼
  单键？              Tab content (2) — handle_key_event
   │
   ├─ 是 → dispatch (直接分派，无 Which 面板)
   │
   └─ 否 (a) → 弹 Which 面板
           │
           ▼
      后续按键过滤 → 剩 1 / 精确匹配 → dispatch
                  → 0 候选 → 关闭消费
```

`tab.shortcuts()` 返回 `&[(KeyCombo, &'static str)]`，数据在 `Tab` 构造时预计算并存于字段中。

`dispatch_shortcut(seq)` 遍历 `all_shortcuts()` 匹配完整键序列，调用 `handle_key_event(key, ...)`。

---

## 七、文件结构

```
src/
├── functions/restful/proxies.rs     REST API 封装
│   ├── ProxiesResponse, Proxy, DelayRecord
│   ├── fetch_proxies()              GET  /proxies
│   ├── select_proxy(group, node)    PUT  /proxies/<group>
│   ├── test_proxy_delay(name)       GET  /proxies/<name>/delay
│   └── test_group_delay(name)       GET  /group/<name>/delay
│
└── tui/tab/proxies.rs              TUI 实现
    ├── mod_agent! → Key enum + agent() + all_shortcuts()
    ├── NodeType (Folder/Link/File), NodeItem
    ├── ProxyTree { nodes: Vec<NodeItem>, name_index }
    │   ├── build() / rebuild_from_proxies()
    │   ├── toggle_expand_at() / expand_at() / collapse_at()
    │   ├── collapse_all() / expand_all()
    │   ├── find_folder_index() / node_at() / len()
    │   └── push_entry() / rebuild_index()
    ├── Proxies (TabContent impl)
    │   ├── dispatch_key()     按键分发
    │   ├── render()            展平渲染 (ListItem)
    │   └── spawn_select_inline()  PUT /proxies/<group>
    └── ProxiesTab (newtype_tab!)
```

---

## 八、TODO

- **侧边栏** — 大屏时显示节点详情面板。

---

## 九、参考

- [Mihomo API](https://wiki.metacubex.one/api/)
