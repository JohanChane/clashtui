## Context

当前 `App::handle_key_event` 将路由硬编码为 `handle_which → popup → handle_global_kv → tab`，每层采用不同的接口模式（Which 返回 bool 并自行 dispatch，PopUp 通过 `check()` 判断，global 返回 bool）。要添加新 Layer（如 Help）需要深入修改路由代码，扩展成本高。

设计目标：Layer 之间接口统一、优先级显式、扩展时只需模式化添加（field + 一行 check）。

## Goals / Non-Goals

**Goals:**
- 定义统一的 Layer 契约：每个 Layer 有 `is_active()` 判断、`handle_key_event(kv)` 处理、`render(f, area)` 渲染
- Which 行为改进：非匹配键取消 chord + 消费，Esc 取消 chord + 消费，匹配键继续/完成 chord
- 零分配 shortcuts 查询：`&[(KeyCombo, &str)]` 引用返回
- 添加新 Layer 仅需 3 步：(a) 定义 struct (b) App 加字段 (c) handle/render/sync 各加一行
- 保持 `mod_agent!` 宏语法不变

**Non-Goals:**
- 不用 trait object 实现 Layer——固定字段 + 静态分发，3-4 层时比 dyn 更简洁
- 不在 `keymap.yaml` 中配置和弦
- 不改变 PopUp 系统、Agent 系统、FutureSet 事务系统
- 不改变 C/DualTab 的 `TabContent` trait 接口

## Decisions

### 1. Layer 优先级顺序

```
Priority  Layer        Active when              Consumes
──────────────────────────────────────────────────────────
  0       PopUp        popup.check()            Always (modal, blocks all)
  1       Which        chord handler active     Matching keys + Esc
                        (pressed not empty)      Non-matching → cancel, consumed
  2       Tab          (always called)          Tab handles own keys normally
  3       App global   (always called)          q, digits, Tab only (last resort)
```

PopUp 是真正模态的——有弹窗时必须完成/取消交互。Which 是软性的——非匹配键取消 chord 并消费该键（不传递）。Esc 取消 chord 同样消费。Tab 层处理 Tab 内容自身的按键。Global 键（`q` 退出、`1`-`9` 切换、`Tab` 轮换）作为最后的 fallback，在 Tab 之后始终被调用。

### 2. ChordHandler 独立模块

从 `App` 中抽出 `ChordHandler` 到 `src/tui/widget/chord.rs`：

```rust
pub struct ChordHandler {
    pressed: Vec<KeyEvent>,
    candidates: Vec<(KeyCombo, &'static str)>,
}

impl ChordHandler {
    pub fn is_active(&self) -> bool { !self.pressed.is_empty() }

    /// Returns true if the key was consumed (chord continues or completed).
    /// Returns false if the chord was cancelled and the key should be reprocessed.
    pub fn handle(&mut self, kv: &KeyEvent, shortcuts: &[(KeyCombo, &str)], 
                  dispatch: &mut dyn FnMut(&[KeyEvent])) -> bool {
        if self.is_active() {
            self.continue_(kv, dispatch)
        } else {
            self.check_init(kv, shortcuts)
        }
    }
}
```

`dispatch: &mut dyn FnMut(&[KeyEvent])` 是闭包形式，调用方（App）闭包内调用 `tab.dispatch_shortcut(seq)`。这避免 ChordHandler 直接依赖 TuiTab trait，保持其纯粹性。

**替代方案**: ChordHandler 泛型化为 `ChordHandler<T: TuiTab>`。拒绝原因：增加类型参数污染，而闭包方案既简单又灵活。

### 3. TuiTab 接口简化

```rust
pub trait TuiTab: TuiWidget {
    fn title(&self) -> &'static str;

    /// Shortcuts of the focused pane, for display in Which/Help panels.
    /// MUST be zero-alloc — implement via static cache.
    fn shortcuts(&self) -> &[(KeyCombo, &'static str)];

    /// Execute the action bound to `combo`.
    fn dispatch_shortcut(&mut self, combo: &[KeyEvent]);
}
```

变化：
- `shortcuts()`: `Vec` → `&[]`（零分配）
- `dispatch_shortcut()`: `bool` → `()`（不再需要返回值来表示"已分发"，ChordHandler 负责跟踪）

### 4. Tab<C> 零分配 shortcuts

```rust
impl<C: TabContent> Tab<C> {
    pub fn shortcuts(&self) -> &[(KeyCombo, &'static str)] {
        use std::sync::OnceLock;
        static CACHED: OnceLock<Vec<(KeyCombo, &str)>> = OnceLock::new();
        CACHED.get_or_init(|| {
            C::all_shortcuts()
                .iter()
                .map(|(combo, _, desc)| (combo.clone(), *desc))
                .collect()
        })
    }
}
```

`OnceLock` 对每个 `C` 类型单态化一次，clone 只发生在初始化时。运行时零分配。

`DualTab` 同理：为 `(C1, C2)` 组合分别缓存两套 shortcuts。

### 5. App 路由代码结构

```rust
impl App {
    fn handle_key_event(&mut self, kv: &KeyEvent) {
        // Layer 0: PopUp (modal)
        if self.popup.check() {
            self.popup.handle_key_event(kv);
            return;
        }

        // Layer 1: Which (chord mode)
        if self.chord.handle(kv, self.tabs[self.tab_index].shortcuts(), &mut |seq| {
            self.tabs[self.tab_index].dispatch_shortcut(seq);
        }) {
            return;
        }

        // Layer 2: Global
        if self.handle_global_kv(kv) {
            return;
        }

        // Layer 3: [Future] Help — same pattern
        // if self.help.handle_key_event(kv) { return; }

        // Layer 4: Tab (fallback)
        self.tabs[self.tab_index].handle_key_event(kv);
    }
}
```

添加新 Layer 的模式：在 `App` 中加字段 → 在 `handle_key_event` 中加一行 `if layer.handle(kv) { return; }` → 在 `render` 中加一行渲染 → 在 `sync` 中加一行同步。无需理解全局路由逻辑。

### 6. ChordHandler 的关键行为

**非匹配键取消并消费**（关闭 Which 面板，不传递）：
```rust
fn continue_(&mut self, kv: &KeyEvent, dispatch: &mut dyn FnMut(&[KeyEvent])) -> bool {
    if kv.code == KeyCode::Esc {
        self.reset();
        return true; // Esc consumed
    }

    let idx = self.pressed.len();
    self.pressed.push(kv.clone());
    self.candidates.retain(|(seq, _)| idx < seq.len() && seq[idx] == *kv);

    match self.candidates.len() {
        0 => { self.reset(); true } // cancel chord, consume the key
        1 => { /* dispatch */ self.reset(); true }
        _ => {
            if let Some((exact, _)) = self.candidates.iter().find(|(s, _)| s.len() == self.pressed.len()) {
                /* dispatch */ self.reset(); true
            } else {
                true // still in chord mode
            }
        }
    }
}
```

这样当用户按 `g` 进入 chord 模式后，按 `1`（不匹配任何候选）会：取消 chord（0 candidates）→ 返回 true → 消费该键 → Which 面板关闭，不传递给 Tab 或 Global。

## Risks / Trade-offs

- **[Risk] `OnceLock` 使 shortcuts 缓存为静态全局** → 对每个 `C` 类型，缓存生命周期为 `'static`。对 profile/template 等类型完全合适——它们的内容类型在编译时确定。Mitigation: 如未来需要多次创建、销毁同一类型的 Tab，`OnceLock` 的 get_or_init 只执行一次，后续调用返回同一引用，无问题。

- **[Risk] 非匹配键取消 chord + 消费 = 误触无副作用** → 在 chord 中按任何不匹配的键只会关闭 Which 面板，不会产生意外行为（不会切 Tab、不会退出）。用户可按 `Esc` 同样取消。

- **[Trade-off] `dispatch: &mut dyn FnMut` 有虚函数开销** → 仅在 chord 完成时调用（低频），每次 chord 生命周期中最多 1 次。性能影响可忽略。

## Open Questions

- Help Layer 的具体行为（全屏遮罩 vs 小型浮动面板）待未来定义。当前设计预留其插入位置即可。
- 未来是否有 Layer 需要优先级高于 PopUp？目前无场景，架构可容纳——在 handle_key_event 最前方插入 check 即可。
