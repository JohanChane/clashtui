## Why

当前多键快捷方式框架存在三个问题：(1) **Which 层阻塞 Tab 切换与全局键** — `handle_which` 激活时对所有按键返回 `true`，无法在 chord 输入中切 Tab 或按 `q` 退出；(2) **`shortcuts()` 每次调用都分配新 Vec** — 每帧按键触发 clone，无意义的堆分配；(3) **无法方便地添加新 Layer** — 未来若想加入 Help 面板（如 `<F1>` 展示快捷键），现有硬编码路由无法清晰扩展。需要一个设计简洁、接口清晰、易于扩展的 Layer 架构。

## What Changes

- **定义清晰的 Layer 优先级**：PopUp(0) > Which(1) > Tab(2) > Global(3)，Global 作为最后的 fallback
- **Which 行为改进**：非匹配键取消 chord 并消费（关闭 Which 面板，不传递）；Esc 取消 chord 并消费
- **`shortcuts()` 改为返回 `&[(KeyCombo, &str)]`**：Tab<C> 通过 `OnceLock` 缓存，零分配
- **ChordHandler 抽离为独立模块**：从 `App::handle_which` 中抽出 `src/tui/widget/chord.rs`，职责单一
- **添加新 Layer 仅需 (a) 定义 Layer 结构体 (b) 在 App 中加字段 (c) 在 handle_key_event/render/sync 中加一行** — 与现有 `popup` 字段对称
- **BREAKING**: `TuiTab` 的 `shortcuts()` 返回类型从 `Vec` 改为 `&[(KeyCombo, &str)]`，`dispatch_shortcut()` 返回 `bool` → `()`

## Capabilities

### New Capabilities
- `layer-architecture`: 优先级有序的 Layer 路由（PopUp > Which > Tab > Global），PopUp/Which 可消费按键阻断后续 Layer
- `chord-handler`: 独立的和弦处理器（ChordHandler），管理 chord 状态、候选过滤、自动 dispatch
- `which-panel-rendering`: 查询式 Which 面板渲染（居中、自适应列数、Clear mask）

### Modified Capabilities
<!-- No existing specs -->

## Impact

- `src/tui/app.rs`: 移除 `WhichState` 和 `handle_which`，新增 `ChordHandler` 字段；路由变为 `if popup → if chord_handler → if global → tab`；render 中 query chord_handler
- `src/tui/widget/chord.rs`: 新文件，ChordHandler 结构体 + 所有 chord 逻辑
- `src/tui/widget/tab.rs`: `KeyCombo` 保留；`Tab<C>::shortcuts()` 改用 `OnceLock` 缓存返回 `&[]`；`dispatch_shortcut()` 返回 `()` 不返回 bool
- `src/tui/widget/dualtab.rs`: `DualTab::shortcuts()` 同理缓存；`dispatch_shortcut()` 返回 `()`
- `src/tui/tab/mod.rs`: `TuiTab` trait 签名变更；`newtype_tab!` 和 `enum_dispatch!` 跟随
- `docs/which_panel.md`: 架构文档更新
