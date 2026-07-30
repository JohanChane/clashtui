# macOS 代理节点不显示延时的根因与修复

## 现象

macOS 上使用 proxy-provider 方式加载出站节点时，Proxies 页面展开组后，
子节点 **不显示延时**（无 `23ms` / `FAIL` 等文字），且手动测速后依然不显示。
Linux 上正常显示。

核心版本相同（Mihomo 1.19.x），问题出在配置方式的差异。

---

## 根因：Mihomo `/proxies` API 返回格式因配置来源不同而异

### 对比一：key 数量

| | **Fixture (Linux 内联)** | **Live macOS (proxy-provider)** |
|---|---|---|
| `/proxies` 总 key 数 | **53** | **17** |
| Group（有 `all[]`）| 9 | 11 |
| Leaf（无 `all[]`）| **44** | **6** |
| 其中实际出站节点 (Vmess/Vless) | ✅ 35 个 | ❌ **0 个** |

### macOS live — 6 个 leaf 全是内置节点

```json
{
  "proxies": {
    "DIRECT":      { "type": "Direct" },
    "REJECT":      { "type": "Reject" },
    "COMPATIBLE":  { "type": "Compatible" },
    "PASS":        { "type": "Pass" },
    "PASS-RULE":   { "type": "PassRule" },
    "REJECT-DROP": { "type": "RejectDrop" },
    "At-hajimi":   { "type": "URLTest",  "all": ["aa-0jezgv2bi8"],   "now": "aa-0jezgv2bi8", "history": [{"delay": 57}] },
    "Sl-hajimi":   { "type": "Selector", "all": ["aa-0jezgv2bi8"],   "now": "aa-0jezgv2bi8", "history": [{"delay": 57}] },
    "Entry":       { "type": "Selector", "all": ["At-hajimi", "Sl-hajimi", "aa-0jezgv2bi8", "rn-ba"], "now": "aa-0jezgv2bi8" },
    // ... 更多 groups
    // ❌ "aa-0jezgv2bi8" 不在顶层 key 中
    // ❌ "rn-ba" 不在顶层 key 中
    // ❌ "[bak]日本-优化" 不在顶层 key 中
  }
}
```

### Linux fixture — 44 个 leaf 包含所有出站节点

```json
{
  "proxies": {
    "DIRECT":     { "type": "Direct" },
    "REJECT":     { "type": "Reject" },
    "At-hajimi":  { "type": "URLTest",  "all": ["vmess-node001"], "now": "vmess-node001" },
    "Entry":      { "type": "Selector", "all": ["vmess-node001", "日本-优化", "日本-优化2", "日本-优化3"], "now": "vmess-node001" },
    // ✅ 所有出站节点都在顶层 key 中：
    "vmess-node001":  { "type": "Vmess",  "history": [{"delay": 42}] },
    "日本-优化":       { "type": "Vmess",  "history": [{"delay": 57}] },
    "日本-优化2":      { "type": "Vmess",  "history": [{"delay": 88}] },
    "日本-优化3":      { "type": "Vmess",  "history": [{"delay": 0}] },    // FAIL
    "香港-优化-Gemini": { "type": "Vmess",  "history": [{"delay": 33}] },
    "香港HK-HY2":     { "type": "Hysteria2", "history": [{"delay": 28}] },
    // ... 共 35+ 个出站节点
  }
}
```

### macOS 的出站节点在哪？—— `/providers/proxies`

proxy-provider 加载的叶子节点在另一个 API 端点里：

```json
// GET /providers/proxies
{
  "providers": {
    "hajimi": {
      "name": "hajimi",
      "vehicleType": "HTTP",
      "proxies": [
        {
          "name": "aa-0jezgv2bi8",
          "type": "Vless",
          "udp": true,
          "history": [{"delay": 212}, {"delay": 215}, {"delay": 205}]
          // ✅ 有完整的 delay/type/udp/tcp 等字段
        }
      ]
    },
    "rn": {
      "name": "rn",
      "vehicleType": "HTTP",
      "proxies": [
        {
          "name": "rn-ba",
          "type": "Vmess",
          "udp": true,
          "history": [{"delay": 919}, {"delay": 904}, {"delay": 408}]
        }
      ]
    },
    "bak": {
      "name": "bak",
      "vehicleType": "HTTP",
      "proxies": [
        { "name": "[bak]日本-优化",  "type": "Vmess", "history": [{"delay": 196}] },
        { "name": "[bak]日本-优化2", "type": "Vmess", "history": [{"delay": 175}] },
        { "name": "[bak]日本-优化3", "type": "Vmess", "history": [{"delay": 227}] },
        { "name": "[bak]印度-优化",  "type": "Vmess", "history": [{"delay": 163}] },
        // ... 共 30+ 个节点
      ]
    }
  }
}
```

### 为什么显示不出延时

`resolve_delay()` 从 `proxies.get(name)` 拿节点信息，但 proxy-provider 的节点名（`aa-0jezgv2bi8`）在 `/proxies` map 中不存在：

```rust
pub fn resolve_delay(name, proxies) -> Option<u64> {
    let proxy = proxies.get(name)?;   // "aa-0jezgv2bi8" → None → 函数退出
    // ...
}
```

而 Groups 虽然通过 `all[]` 和 `now` 引用了这些子节点，但由于子节点不在 map 里，
`resolve_delay("aa-0jezgv2bi8", proxies)` 直接返回 `None` → 不显示延时。

---

## 修复方案

**把 `/providers/proxies` 的出站节点 merge 进 proxies map。**

新增 `fetch_proxies_with_providers()` 函数（`src/functions/restful/proxies.rs`）：

```rust
pub fn fetch_proxies_with_providers() -> Result<ProxiesResponse> {
    // 1. 先调 /proxies 拿 groups
    let mut response: ProxiesResponse =
        request(Method::Get, "/proxies", None).and_then(|r| r.json())?;
    // 2. 再调 /providers/proxies 拿所有 provider 的出站节点
    // 3. merge 进 response.proxies（不覆盖 groups 已有的 key）
    if let Ok(pp) = fetch_providers_proxies() {
        for (name, proxy) in pp {
            response.proxies.entry(name).or_insert(proxy);
        }
    }
    Ok(response)
}
```

合并后 proxies map 变为：

```
Groups（原有 11 个）：
  At-hajimi, At-rn, Entry, Entry-LastMatch, Entry-RuleMode,
  FltAt-hajimi, FltAt-rn, GLOBAL, Sl-hajimi, Sl-rn, 看视频和下载不要选这个

内置（原有 6 个）：
  COMPATIBLE, DIRECT, PASS, PASS-RULE, REJECT, REJECT-DROP

Provider 出站（新增 ~35 个）：
  aa-0jezgv2bi8, rn-ba, [bak]日本-优化, [bak]日本-优化2, ...
```

所有调用 `fetch_proxies` 的地方统一换成 `fetch_proxies_with_providers`：

| 调用位置 | 场景 |
|---------|------|
| `content.rs:on_enter()` | 进入 Proxies 面板 |
| `content.rs:after_sync()` | 每 5 秒自动刷新 |
| `handlers.rs:select_inline()` | 切换代理后刷新 |
| `handlers.rs:test_delay()` | 单节点/组测速后刷新 |
| `handlers.rs:test_all_delay()` | 全测后刷新 |
| `handlers.rs:refresh()` | 手动 `r` 刷新 |

`tree.rs` 和 `resolve_delay` **零改动**——因为合并后 `proxies.get("aa-0jezgv2bi8")` 能直接找到节点。

---

## 修改的文件

```
src/functions/restful/proxies.rs   ← 新增 fetch_providers_proxies() + fetch_proxies_with_providers()
src/tui/tab/proxies/content.rs     ← fetch_proxies → fetch_proxies_with_providers
src/tui/tab/proxies/handlers.rs    ← fetch_proxies → fetch_proxies_with_providers
src/functions/command/macos.rs     ← rustfmt (无行为变更)
src/config/util.rs                 ← clippy (pre-existing)
```
