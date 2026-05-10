## Context

demotui 是 ClashTUI 的重构版，使用 `Tab<C: TabContent>` 泛型容器 + `FutureSet` 事务系统将 UI 同步逻辑与异步 I/O 分离。已有 StatusTab（状态轮询）和 FileTab（DualTab 文件管理）。

本次新增 ProxiesTab，需要调用 Mihomo REST API `/proxies` 获取代理数据，构建树形结构展示，支持节点选择和测速。复用现有 `Tab<C>` 容器 + `FutureSet` 事务模式。

## Goals / Non-Goals

**Goals:**
- 实现 ProxiesTab（Tab 位置 3，数字键 `3`），以文件浏览器方式展示 Mihomo 代理树
- 策略组 → 文件夹，叶子节点（Vmess/Shadowsocks/Direct 等）→ 文件
- 支持 Selector 组节点切换（PUT /proxies/<name>）
- 支持单节点和组内批量测速
- 内联显示延迟信息，符合项目"减少弹窗"原则
- 树节点可展开/折叠（Enter on folder），光标在文件时按 `u` 可折叠所属文件夹
- 遵循现有 `TabContent` + `FutureSet` + `tri!` 宏的模式

**Non-Goals:**
- 不支持代理集合（proxy providers）管理
- 不支持规则修改
- 不支持连接终止
- 不实现双栏布局（先用单栏，后续视需求扩展）

## Decisions

### 1. 数据模型：本地 ProxyTree 抽象

**决策**：API 返回扁平 map，在 Rust 侧构建树形结构。

Miho API `/proxies` 返回 `{name: Proxy}` 扁平结构，策略组通过 `all: Vec<String>` 引用子节点。需要从根组（如 GLOBAL）出发 DFS 构建树。

```
ProxyTree {
    roots: Vec<TreeEntry>,
    entries: HashMap<Name, Proxy>,  // 快查
}

TreeEntry {
    name: String,
    proxy_type: ProxyType,     // Selector | URLTest | Vmess | Direct | ...
    alive: bool,
    delay: Option<u64>,
    now: Option<String>,       // 当前选中 (仅策略组)
    children: Vec<TreeEntry>,  // 子节点
    expanded: bool,            // 展开/折叠
}
```

**替代方案**：直接存 HashMap + 展开时动态查 `all`，更简单但每次 render 需遍历，选择牺牲一些内存换渲染效率。

### 2. 刷新策略：定时轮询 + 手动触发

**决策**：`after_sync` 实现周期性自动刷新（类似 StatusTab），同时支持手动按键触发测速。

- 自动刷新间隔：5 秒（仅在 Tab 活跃时）
- 测速操作：手动触发（按键 `t`/`T`），避免不必要的网络请求
- 节点选择后立即刷新以获取最新状态
- 使用 `tri!` 宏：网络错误用 `or_set` 设置 error 字段在 StatusBar 区域显示

### 3. Tab 类型：单面板 Tab<Proxies>，位置 3

**决策**：使用 `Tab<Proxies>` 单面板，放置于 Tab 数组索引 2（数字键 `3` 切换），即 Status(1) → Proxies(3) → File(4) 的顺序。

代理树天然适合单列表展示——列表项缩进表示层级，`▶`/`▼` 前缀 + 延迟信息在同一行即可。双栏意义不大（没有"选中项详情"的强需求）。

### 4. UI 隐喻：文件夹与文件

**决策**：采用文件浏览器隐喻，而非"策略组/节点"的技术术语。

| 概念 | 隐喻 | 对应 type |
|------|------|-----------|
| 策略组 (Selector, URLTest, Fallback, LoadBalance) | 📁 文件夹 | 含 `all` 字段，可展开/折叠 |
| 叶子节点 (Vmess, Direct, Reject, Shadowsocks...) | 📄 文件 | 无 `all` 字段，不可展开 |

- 文件夹默认折叠
- 展开时缩进显示子项目
- 文件夹显示当前选中项（`now`）作为标记
- 文件显示延迟（`delay`）信息

### 4. 按钮映射设计

**决策**：复用项目 Vim-like 风格，引入文件浏览器隐喻——策略组 = 文件夹，叶子节点 = 文件：

| 按键 | 适用对象 | 动作 |
|------|----------|------|
| `j`/`↓` | 全部 | 下移 |
| `k`/`↑` | 全部 | 上移 |
| `Enter`/`→`/`l` | 文件夹 | 展开/折叠文件夹 |
| `Enter`/`→`/`l` | 文件 | 选择节点（在 Selector 组内弹出节点列表） |
| `←`/`h` | 文件夹 | 折叠文件夹 |
| `u` | 文件 | 折叠光标所属的父文件夹 |
| `t` | 全部 | 对选中项测速（单个） |
| `T` | 文件夹 | 对文件夹内所有节点批量测速 |
| `/` | 全部 | 搜索过滤 |
| `s` | 文件夹 | 切换节点选择（弹出 Choice PopUp） |

`u` 键的设计理由：文件浏览器中，"在文件上按 u 回到上层目录"是直观的操作模式，减少需要导航到父文件夹再按 ← 的步骤。

### 5. 节点类型显示：图标 + 缩进

```
 GLOBAL                     Selector  ▶
   Entry                    Selector  ▼  231ms
     At-pvd0               URLTest   ▼  191ms
       vmess-ipdktc33      Vmess        191ms
     DIRECT                Direct
   REJECT                  Reject
```

- `▶` 折叠的策略组
- `▼` 展开的策略组
- 叶子节点无前缀
- 策略组当前选中的节点用高亮标记
- 延迟直接显示在行尾（如 `231ms`）

### 6. 选择节点操作流程

选中某 Selector 组，按 `Enter`：
1. 弹出快捷列表（PopUp Choice），列出该组 `all` 中的所有子节点
2. 用户选择后通过 oneshot 传回
3. 事务中调用 `PUT /proxies/<name> {"name": "<target>"}`, 
4. 成功后 wrapper 触发 `content.refresh()` 刷新数据

替代方案：在树内就地选择（按 Enter 直接选中下一个子节点循环）。选择 PopUp 方式因为节点可能很多且存在嵌套。

### 7. 测速操作流程

- 单节点测速：`GET /proxies/<name>/delay`，结果更新本地 delay 缓存
- 组内批量测速：`GET /group/<name>/delay`，结果通过额外 API 获取或重新拉取 `/proxies`
- 测速期间节点名前显示旋转动画字符（`-/|\`）表示进行中的事务

## Risks / Trade-offs

- **API 返回数据量大**（大量节点时）→ 每 5s 轮询全量可能产生流量，后续可优化为增量更新
- **嵌套深度不确定** → DFS 构建树时有递归栈溢出风险，但实际代理树深度通常 ≤ 5，风险极低
- **测速超时** → 使用 `minreq` timeout（已有 DEFAULT_TIMEOUT），测速失败显示 `-` 而非卡住
- **PopUp 节点选择列表过长** → 后续可添加搜索过滤（`/` 键）快速定位
