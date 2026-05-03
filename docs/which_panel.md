# Which Panel — 多键快捷方式与按键提示面板

## 设计概述

Which 层是事件循环中**第二层按键路由**，通过 `ChordHandler` 独立模块统一处理所有快捷方式：

```
KeyEvent
   │
   ▼
  PopUp 层 (0) ─────────── 有弹窗就阻断
   │
   ▼
  ChordHandler (1) ──► 匹配 shortcut？
   │ 是                   │ 否
   ▼                       ▼
  单键？               Tab 层 (2) — content handler
   │                       │
   ├─ 是 → dispatch      Global 层 (3) — q / 1-9 / Tab
   │         无画面
   │
   └─ 否 (chord prefix) → 显示 Which Panel
          │
          ▼
     后续按键 → 过滤候选 → 剩 1 项/精确匹配 → dispatch
                         → 0 项 → 关闭，消费按键
                         → Esc → 关闭，消费按键
```

参考 yazi 的 Which panel 设计，但做了以下简化：
- 快捷方式通过 `mod_agent!` 硬编码
- 单键直接 dispatch（不显示面板），仅多键 prefix 才弹出面板
- 单键优先：同一个 key 同时是单键和 chord prefix 时，单键胜出

---

## 一、数据模型

### 1.1 Shortcut 条目（静态，零分配）

`mod_agent!` 宏生成 `all_shortcuts()` 函数，返回 `&'static [(KeyCombo, Key, &str)]`：

```rust
pub fn all_shortcuts() -> &'static [(KeyCombo, Key, &'static str)] {
    SHORTCUTS.get_or_init(|| vec![
        // 单键: (key_combo, action, description)
        ([Char('p')], Action::Preview, ""),
        // 双键 chord: (key_combo, action, description)
        ([Char('g'), Char('g')], Action::GoTop, "Go to top"),
        ([Char('g'), Char('e')], Action::GoEnd, "Go to end"),
    ])
}
```

`Tab<C>::shortcuts()` 通过 `OnceLock` 缓存为 `&[(KeyCombo, &str)]`（去掉 Key enum），零分配返回。

### 1.2 ChordHandler（运行时）

位于 `src/tui/widget/chord.rs`，存储在 `App.chord` 字段（非 Option，始终存在）：

```rust
pub struct ChordHandler {
    pub pressed: Vec<KeyEvent>,                    // 已累积的 prefix keys
    pub candidates: Vec<(KeyCombo, &'static str)>,  // 剩余候选项
}
```

| 阶段 | pressed | candidates |
|------|---------|------------|
| 用户按下 `g` | `[g]` | `[(g,g→"Go to top"), (g,e→"Go to end")]` |
| 用户按下 `e` | `[g, e]` | `[(g,e→"Go to end")]` → dispatch |

---

## 二、面板接口 — shortcuts() 与 dispatch_shortcut()

每个 Tab 通过 `TuiTab` trait 暴露两个方法：

```rust
pub trait TuiTab: TuiWidget {
    fn title(&self) -> &'static str;

    /// 返回 focused panel 的全部快捷方式 (combo, 描述) — 零分配
    fn shortcuts(&self) -> &[(KeyCombo, &'static str)];

    /// 匹配序列并执行对应 action
    fn dispatch_shortcut(&mut self, seq: &[KeyEvent]);
}
```

### 2.1 Tab\<C\> 实现（单栏）

```rust
impl<C: TabContent> Tab<C> {
    pub fn shortcuts(&self) -> &[(KeyCombo, &'static str)] {
        static CACHED: OnceLock<Vec<(KeyCombo, &str)>> = OnceLock::new();
        CACHED.get_or_init(|| {
            C::all_shortcuts().iter()
                .map(|(combo, _, desc)| (combo.clone(), *desc))
                .collect()
        })
    }

    pub fn dispatch_shortcut(&mut self, seq: &[KeyEvent]) {
        for (s, key, _) in C::all_shortcuts() {
            if &**s == seq {
                self.content.handle_key_event(*key, &mut self.tasks, &mut self.state);
                return;
            }
        }
    }
}
```

### 2.2 DualTab\<C1, C2\> 实现（双栏）

双栏根据 `is_focus_on_c1` 决定从哪个 pane 取数据。使用两个 `static` 的 `OnceLock` 分别缓存两个 content 类型的 shortcuts：

```rust
impl<C1, C2> DualTab<C1, C2> {
    pub fn shortcuts(&self) -> &[(KeyCombo, &'static str)] {
        static C1_CACHED: OnceLock<Vec<(KeyCombo, &str)>> = OnceLock::new();
        static C2_CACHED: OnceLock<Vec<(KeyCombo, &str)>> = OnceLock::new();
        let v = if self.is_focus_on_c1 {
            C1_CACHED.get_or_init(|| ...)
        } else {
            C2_CACHED.get_or_init(|| ...)
        };
        v
    }

    pub fn dispatch_shortcut(&mut self, seq: &[KeyEvent]) {
        let target = if self.is_focus_on_c1 { C1Side } else { C2Side };
        // 在对应 pane 的 all_shortcuts() 中查找匹配并 dispatch
    }
}
```

### 2.3 类型层级

```
TuiTab trait                    ← shortcuts() / dispatch_shortcut()
   │
  Tab enum (enum_dispatch!)     ← dispatch to variant
   │
   ├── StatusTab(Tab<Status>)   ← Tab::shortcuts / Tab::dispatch_shortcut
   └── FileTab(DualTab<Profile, Template>) ← DualTab::shortcuts / DualTab::dispatch_shortcut
             │                                     │
             │ is_focus_on_c1 决定用哪个 pane       │
             │                                     │
             ├── C1 = Profile  → C1::all_shortcuts()
             └── C2 = Template → C2::all_shortcuts()
```

---

## 三、mod_agent! 宏

### 3.1 语法

```rust
mod_agent!(
    Key,
    [
        // 单键: ([KeyCode], Key, description)
        ([KeyCode::Left],                   Key::Switch, ""),
        ([KeyCode::Down],                   Key::MoveDown, ""),
        ([KeyCode::Char('i')],              Key::Action(Action::Add), ""),
        // 多键 chord: ([prefix, suffix, ...], Key, description)
        ([KeyCode::Char('g'), KeyCode::Char('g')], Key::Action(Action::GoTop), "Go to top"),
        ([KeyCode::Char('g'), KeyCode::Char('e')], Key::Action(Action::GoEnd), "Go to end"),
        // N 键同理扩展
    ]
);
```

- 键序列统一用 `[...]` 包起来，macro 内部自动区分：
  - `@agent` muncher 只收集单键条目 → `HashMap<KeyEvent, Key>`
  - `@shortcuts` muncher 收集全部条目 → `all_shortcuts()` 返回类型
- KeyCombo 长度决定是单键、双键还是多键 chord

### 3.2 BasicTabContent 集成

```rust
impl BasicTabContent for Profile {
    type Key = Key;
    type State = ListState;
    const TITLE: &str = "Profile";

    fn all_shortcuts() -> &'static [(KeyCombo, Self::Key, &'static str)] {
        all_shortcuts()  // 来自 mod_agent! 导出的函数
    }
}
```

没有键绑定的 Content（如 Status）使用默认实现返回 `&[]`。

---

## 四、ChordHandler 实现

### 4.1 入口：handle()

```rust
pub fn handle(
    &mut self,
    kv: &KeyEvent,
    shortcuts: &[(KeyCombo, &'static str)],
    dispatch: &mut dyn FnMut(&[KeyEvent]),
) -> bool {
    if self.is_active() {
        self.continue_(kv, dispatch)          // 已在 chord 中 → 过滤+分派
    } else {
        self.check_init(kv, shortcuts, dispatch)  // 未在 chord 中 → 尝试启动
    }
}
```

### 4.2 首次按键：check_init()

```
fn check_init(kv, shortcuts, dispatch) -> bool:
    1. 检查单键匹配 → dispatch, return true
    2. 收集前缀匹配的多键候选项 (seq.len() > 1 && seq[0] == kv)
    3. 如果有候选项 → pressed=[kv], candidates=候选项, return true
    4. 否则 → return false (放行到后续 Layer)
```

### 4.3 后缀按键：continue_()

```
fn continue_(kv, dispatch) -> bool:
    Esc → reset, return true (消费)
    非 Esc:
        idx = pressed.len()
        pressed.push(kv)
        candidates.retain(|seq,_| idx < seq.len() && seq[idx] == kv)

        match candidates.len():
            0 → reset, return true (消费, 不传递)
            1 → dispatch, reset, return true
            >1:
                exact match (seq.len() == pressed.len())? → dispatch, reset
                返回 true (留在 chord 中)
```

### 4.4 关键行为总结

| 场景 | 行为 | 返回值 |
|------|------|--------|
| 单键快捷方式 | dispatch，不进入 chord | true |
| chord prefix | 进入 chord，显示 Which 面板 | true |
| chord 中匹配 | dispatch，复位 | true |
| chord 中不匹配 | 复位，关闭 Which 面板 | true（消费） |
| chord 中 Esc | 复位，关闭 Which 面板 | true（消费） |
| 无匹配 | — | false（放行） |

---

## 五、按键路由器（App 层）

### 5.1 路由代码

位于 `src/tui/app.rs`：

```rust
/// KeyEvent Route:
/// PopUp(0) → Which(1) → Tab(2) → Global(3)
fn handle_key_event(&mut self, kv: &KeyEvent) {
    // Layer 0: PopUp (modal)
    if self.popup.check() {
        self.popup.handle_key_event(kv);
        return;
    }

    // Layer 1: ChordHandler (Which)
    let ti = self.tab_index as usize;
    let shortcuts_ptr: *const [(KeyCombo, &str)] = {
        self.tabs[ti].shortcuts() as *const _
    };
    if self.chord.handle(kv, unsafe { &*shortcuts_ptr }, &mut |seq| {
        self.tabs[ti].dispatch_shortcut(seq);
    }) {
        return;
    }

    // Layer 2: Tab content
    self.tabs[ti].handle_key_event(kv);
    // Layer 3: Global (fallback — q, 1-9, Tab)
    self.handle_global_kv(kv);
}
```

### 5.2 每帧流程

```
serve():
    terminal.draw(|f| app.render(f))   // ① 渲染 (tabbar → tab content → which → popup)
    app.sync()                         // ② async callback 处理
    tokio::select! {                   // ③ 等待事件
        event = events.next() => ...   //    按 tick rate 50fps
        _ = tick => continue
    }
    app.handle_key_event(&kv)          // ④ 四层路由
```

### 5.3 添加新 Layer

以 Help 为例，只需在 App 中三个位置插入：

```rust
// 1. struct 加字段
pub struct App {
    help: HelpPanel,
    ...
}

// 2. handle_key_event — 在 Tab 和 Global 之间插入
self.tabs[ti].handle_key_event(kv);
if self.help.is_active() { self.help.handle_key_event(kv); return; }
self.handle_global_kv(kv);

// 3. render — 在 tab content 之后
self.tabs[ti].render(f, area);
if self.help.is_active() { self.help.render(f, area); }

// 4. sync — 加一行
self.help.sync();
```

---

## 六、Which Panel 渲染

### 6.1 布局

```
                    ┌ Which? ────────────────────┐
                    │                            │
 Tab Content        │  g   Go to top             │
 (正常渲染在       │  e   Go to end             │
  下方被覆盖)      │                            │
                    └────────────────────────────┘
```

- 使用 `Clear` widget 做遮罩效果
- 标题：`" Which? "`
- 内容：候选项按 `key  description` 格式排列
- ≤4 项 → 1 列，≥5 项 → 2 列
- 居中弹出，`render_which()` 直接从 `self.chord.pressed` 和 `self.chord.candidates` 读取数据

### 6.2 key_event_to_str

```rust
pub fn key_event_to_str(k: &KeyEvent) -> String {
    match k.code {
        KeyCode::Char(' ')  => "<Space>".into(),
        KeyCode::Char(c)    => c.to_string(),
        KeyCode::Enter      => "<Enter>".into(),
        KeyCode::Esc        => "<Esc>".into(),
        KeyCode::Tab        => "<Tab>".into(),
        // ...
        _                   => format!("{:?}", k.code),
    }
}
```

Panel 只显示未按的后缀键：`remaining = &seq[self.chord.pressed.len()..]`。

---

## 七、完整事件流示例

以 Profile Tab 中按 `g g` 跳到列表顶部为例：

```
1. 用户按 g (ChordHandler 未激活)
   │
2. App::handle_key_event(&KeyEvent('g'))
   │  PopUp → false
   │
   │  chord.handle('g', shortcuts_from_active_tab, dispatch_closure)
   │    check_init():
   │      单键匹配: 遍历 seq.len()==1 → 'g' 不在单键中
   │      chord prefix: 遍历 seq.len()>1 && seq[0]=='g'
   │        → 找到 (g,g)→GoTop, (g,e)→GoEnd
   │        → pressed = ['g']
   │        → candidates = [(g,g,"Go to top"), (g,e,"Go to end")]
   │        → return true (消费按键)
   │
3. 下一帧 render():
   │  chord.is_active() == true → render_which()
   │    ┌ Which? ───────────┐
   │    │  g   Go to top    │
   │    │  e   Go to end    │
   │    └───────────────────┘
   │
4. 用户按 g (ChordHandler 已激活)
   │
5. App::handle_key_event(&KeyEvent('g'))
   │  chord.handle('g', shortcuts, dispatch_closure)
   │    continue_():
   │      pressed = ['g', 'g']
   │      过滤: seq[1]=='g' → (g,g) 保留, (g,e) 删除
   │      candidates.len() = 1 → auto-dispatch
   │        → dispatch(&['g', 'g'])
   │        → tabs[1].dispatch_shortcut(['g', 'g'])
   │          → DualTab → C1::all_shortcuts() 查找
   │          → 匹配 GoTop → Profile::handle_key_event(GoTop)
   │            → state.select_first()
   │        → reset() (pressed 清空, candidates 清空)
   │        → return true
   │
6. 下一帧 render() 显示列表已跳转到顶部
```

---

## 八、与 yazi 的对比

| 方面 | Yazi | demotui |
|------|------|---------|
| 路由层 | Router → Which → Dispatcher | PopUp → Which → Tab → Global |
| 单键 | Router 直接 dispatch，不进 Which | Which 透明 dispatch（无画面） |
| 数据存储 | TOML `keymap.toml` 可配置 | `mod_agent!` 硬编码 |
| 候选结构 | `Chord { on, run, desc }` | `(KeyCombo, &'static str)` |
| Shortcuts 查询 | 每次构造 | OnceLock 缓存，零分配 |
| 过滤算法 | `cands.retain` + `times` | `cands.retain` + `pressed.len()` |
| 自动 dispatch | 剩 1 项 | 剩 1 项 + 精确长度匹配 |
| 非匹配键 | 取消 + 放行 | 取消 + 消费 |
| ChordHandler | 内嵌在 Router | 独立模块 `src/tui/widget/chord.rs` |
| 面板位置 | 终端底部 | 居中 |
| 列数 | 可配置（默认 3） | 自适应（≤4 → 1 列，≥5 → 2 列） |
