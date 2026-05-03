# demotui 事件处理

## 总览

```
  crossterm::EventStream
         │
         ▼
  ┌──────────────┐
  │  App::serve() │  主事件循环 (50fps)
  └──────────────┘
         │
         ├─ EventStream::next()  ──►  KeyEvent
         │  (tokio::select!)
         │    ├─ KeyEvent  ──►  app.handle_key_event()
         │    ├─ tick     ──►  continue (固定帧率等待)
         │    └─ FULL_RENDER ──►  terminal.clear() (强制重绘)
         │
         ├─ terminal.draw(|f| app.render(f))  ──►  每帧渲染
         └─ app.sync()  ──►  每帧推进异步事务
```

## 核心 Traits

### TuiWidget — 所有 UI 组件的统一接口

```rust
trait TuiWidget {
    fn handle_key_event(&mut self, kv: &KeyEvent);
    fn render(&mut self, f: &mut Frame, area: Rect);
    fn sync(&mut self);
}
```

三个方法的职责：

| 方法 | 调用时机 | 职责 |
|------|----------|------|
| `handle_key_event` | 收到按键时 | 处理用户输入，可能 spawn 异步任务 |
| `render` | 每帧 (50fps) | 绘制 UI |
| `sync` | 每帧 (render 之后) | 推进已完成的异步事务，应用结果 |

---

## 一、事件循环 (App::serve)

位于 `src/tui/app.rs:32`。

```
loop {
    terminal.draw(|f| app.render(f))?;   // 1. 渲染
    app.sync();                           // 2. 同步异步事务结果

    // 3. 等待下一个事件 (KeyEvent 或 tick)
    let ev = tokio::select! {
        Some(ev) = events.next() => ev?,
        _ = invt.tick()        => continue,   // 维持帧率
        _ = FULL_RENDER.notified() => {        // 强制重绘
            terminal.clear()?;
            continue;
        },
    };

    // 4. 处理事件
    match ev {
        Event::Key(key_event) => app.handle_key_event(&key_event),
        Event::Resize(..)     => terminal.autoresize()?,
        _ => (),
    }
}
```

关键点：
- **固定帧率**：tick 为 20ms (50fps)，即使没有输入也会持续渲染
- **Resize 事件**：调用 `autoresize()` 调整终端尺寸
- **FULL_RENDER**：用于切换界面后强制清屏重绘（如退出 raw mode 又恢复时）

---

## 二、三层按键路由

位于 `src/tui/app.rs:81`。

```
KeyEvent
   │
   ▼
 PopUp 层 ──► 有弹出窗？ ──► PopUp 处理
   │ 无
   ▼
 App 层   ──► 全局快捷键？ ──► 退出/Tab切换/...
   │ 无
   ▼
 Tab 层   ──► 当前活跃 Tab 处理
```

```rust
fn handle_key_event(&mut self, kv: &KeyEvent) {
    if self.popup.check() {
        self.popup.handle_key_event(kv);         // PopUp 层
    } else if !self.handle_global_kv(kv) {       // App 层
        self.tabs[self.tab_index as usize]
            .handle_key_event(kv);               // Tab 层
    }
}
```

**App 层全局快捷键**：

| 按键 | 行为 |
|------|------|
| `1` `2` | 直接切换到对应 Tab |
| `Tab` | 循环切换到下一个 Tab |
| `q` | 退出程序 |

**无返回值**：每层直接处理或放行，不需要 EventState/Consumed 这类返回值。

---

## 三、Tab 系统

### 3.1 Tab<C: TabContent> — 单 Tab 容器

位于 `src/tui/widget/tab.rs:43`。

```rust
struct Tab<C: TabContent> {
    content: C,            // 业务逻辑
    state: C::State,       // UI 状态（如 ListState）
    tasks: FutureSet<C>,   // 异步事务集合 (JoinSet)
}
```

**基础 trait — BasicTabContent**：

```rust
trait BasicTabContent: 'static {
    type Key: TryFrom<&KeyEvent, Error = ()>;  // 该 Tab 关心的按键
    type State;                                 // UI 状态类型

    const TITLE: &str;                         // Tab 标题

    fn after_sync(&self, task_set: &mut FutureSet<Self>) {}  // 每次事务完成后的钩子
}
```

**核心 trait — TabContent**：

```rust
trait TabContent: BasicTabContent {
    fn init(&mut self, task_set: &mut FutureSet<Self>, state: &mut Self::State);
    fn handle_key_event(&mut self, key: Self::Key, task_set: &mut FutureSet<Self>, state: &mut Self::State);
    fn render(&self, f: &mut Frame, area: Rect, state: &mut Self::State);
}
```

**生命周期**：

```
Default::default()
    ├── content: C::default()
    ├── state: C::State::default()
    ├── tasks: FutureSet::default()
    └── content.init(&mut tasks, &mut state)   ◄── 初始数据加载

loop {
    ──► tab.handle_key_event(kv)
    │     └── content.handle_key_event(key, &mut tasks, &mut state)
    │           └── tasks.spawn(async { ... })  ◄── 产生事务
    │
    ──► tab.render(f, area)
    │     └── content.render(f, area, &mut state)
    │
    ──► tab.sync()
          └── while let Some(cb) = tasks.try_join_next() {
                  cb(&mut content);             ◄── 应用事务结果
                  content.after_sync(&mut tasks); ◄── 钩子（如触发后续加载）
              }
}
```

### 3.2 DualTab<C1, C2> — 双栏 Tab

位于 `src/tui/widget/dualtab.rs:46`。

```
┌──────────────────────────┬──────────────┐
│  C1 (70% when focused)   │ C2 (30%)     │
│                          │              │
│  is_focus_on_c1 = true   │              │
└──────────────────────────┴──────────────┘

                 ↕ 左右方向键切换焦点

┌──────────────┬──────────────────────────┐
│  C1 (30%)    │ C2 (70% when focused)    │
│              │                          │
│              │ is_focus_on_c1 = false   │
└──────────────┴──────────────────────────┘
```

**关键设计**：
- 两个内容共享同一个 `FutureSet<(C1, C2)>`，事务闭包可以同时修改二者
- `handle_key_event` 返回 `bool`：`true` 表示切换焦点到另一边
- `DualTabContent` 和 `DualTabContentMate` 是对称的 trait，区别仅在于 `Mate` 关联类型指谁

```rust
trait DualTabContent: BasicTabContent {
    type Mate: DualTabContentMate<Mate = Self>;

    fn handle_key_event(&mut self, key, tasks, state) -> bool;
    //                                                     ^^^^ true = switch focus

    fn render(&self, f, area, state, is_focused: bool);
}
```

### 3.3 已有的 Tab 实现

| Tab | 类型 | 实现 |
|-----|------|------|
| StatusTab | `Tab<Status>` | 显示 Mihomo 状态，每秒轮询版本和配置 |
| FileTab | `DualTab<Profile, Template>` | 7:3 双栏，管理订阅和模板 |

### 3.4 创建新 Tab 的步骤

1. 定义 `Content` struct，实现 `Default`
2. 定义 `Key` enum，实现 `TryFrom<&KeyEvent>`
3. 实现 `BasicTabContent`（指定 Key、State、TITLE）
4. 实现 `TabContent`（init、handle_key_event、render）
5. 用 `newtype_tab!` 宏包装：`newtype_tab!(MyTab(Tab<MyContent>), "显示名");`
6. 添加到 `enum_dispatch!` 和 `tabs` 向量

---

## 四、事务系统 (FutureSet)

### 核心概念

`FutureSet<C>` 是 `tokio::task::JoinSet<CallBack<C>>`，其中：

```rust
type CallBack<C> = Box<dyn FnOnce(&mut C) + Send>;
```

**事务** = 异步 Future + 完成后返回的闭包。

### 事务生命周期

```
1. 产生事务
   handle_key_event 中:
   ┌──────────────────────────────────────────┐
   │ async {                                  │
   │     let result = do_io().await;   // 异步 │
   │     wrapper(|content: &mut C| {   // 闭包 │
   │         content.apply(result);    // 应用 │
   │     })                                   │
   │ }.spawn_at(task_set);                    │
   └──────────────────────────────────────────┘

2. 自动推进
   sync() 中:
   while let Some(cb) = tasks.try_join_next() {
       cb(&mut content);  // 在主线程安全地修改 UI 数据
   }
```

**JoinSet 的优势**：
- 不需要主循环去 poll 每个 future
- 完成顺序无关，谁先完成谁先应用
- 自动管理 spawned task 的生命周期

### 辅助函数

```rust
// 包装一个闭包为 CallBack
fn wrapper<C>(f: impl FnOnce(&mut C) + Send + 'static) -> CallBack<C>

// 空操作（如操作取消时）
fn do_nothing<C>() -> CallBack<C>

// 扩展 trait，让 async block 可以直接 .spawn_at(task_set)
trait FutureSetExt<C> {
    fn spawn_at(self, set: &mut FutureSet<C>);
}
```

### tri! 宏 — 事务中的错误处理

```rust
// 基本用法：错误时显示确认弹窗，返回空操作
tri!(fallible_operation());

// or_cancel：错误时静默取消
tri!(fallible_operation(), or_cancel);

// or_set：错误时通过闭包设置错误状态
tri!(fallible_operation(), or_set);
```

### 事务中使用 PopUp

PopUp 通过 oneshot 通道与事务上下文通信：

```rust
async fn search() -> CB {
    let filter = tri!(
        Input::new()
            .with_title("Filter".to_owned())
            .build_and_send()    // 发送 PopUp，返回 Receiver
            .await,              // 等待用户输入
        or_cancel                // 用户按 Esc 取消
    );

    wrapper(|content: &mut C| {
        content.filter = Some(filter);
    })
}
```

完整流程：
1. `build_and_send()` 创建 `oneshot::channel`，将 PopUp 通过 `PAIR` 全局通道推入队列
2. 事务 async block 等待 `rx.await`（用户确认/取消）
3. `sync()` 中 popup 通过 `PAIR` 接收新弹窗，按键处理中调用 `tx.send()` 传回结果
4. 事务得到结果，返回闭包应用到 content

---

## 五、PopUp 系统

### 架构

```
事务 (async block)
   │ build_and_send()
   ▼
PAIR (全局 mpsc channel)
   │
   ▼
PopUp::content: Vec<Wrapped>    ← sync() 中接收
   │ handle_key_event(kv)
   ├── Route::Keep  ──►  继续等待
   ├── Route::Send  ──►  pop + send(tx)  ──►  oneshot → 事务拿到结果
   └── Route::Drop  ──►  pop (丢弃)
```

### Msg trait — 弹窗内容

```rust
trait Msg {
    type Result;

    fn match_key_event(&mut self, kv: &KeyEvent) -> Route;
    fn send(self, tx: Sender<Self::Result>);
    fn render(&self, f: &mut Frame, area: Rect, block: Block, is_focused: bool);
    fn size(&self) -> (u16, u16);  // 估计渲染大小 (宽, 高)
}
```

### Route — 弹窗的三种状态变更

| Route | 含义 | 效果 |
|-------|------|------|
| `Keep` | 继续 | 弹窗保持打开 |
| `Send` | 确认发送 | 调用 `send(tx)`，弹窗关闭，结果传回事务 |
| `Drop` | 取消 | 弹窗关闭，oneshot 断开，事务得到 `RecvError` |

### 已有的 Msg 实现

| 类型 | Result | 用途 |
|------|--------|------|
| `Input` | `String` | 文本输入（Enter 确认，Esc 取消） |
| `Confirm` | `()` | 确认框（任意 Enter/Esc/Space 关闭） |

### MsgBuilder — 弹窗构建

```rust
Input::new()
    .with_title("Name".to_owned())        // 标题
    .with_prompt("Enter profile name")    // 提示文本（可选，可滚动）
    .build_and_send()                     // → Receiver<R>
```

**布局**（有 prompt 时）：

```
┌title──────────────┐
│prompt text         │  ← 可滚动的多行提示
├────────────────────┤
│> user input_       │  ← 输入区
└────────────────────┘
```

**布局**（无 prompt 时）：

```
┌title──────────────┐
│> user input_       │
└────────────────────┘
```

### 全局通道 PAIR

```rust
static PAIR: LazyLock<(
    mpsc::Sender<Wrapped>,     // 发送端（在事务中调用）
    Mutex<mpsc::Receiver<Wrapped>>  // 接收端（在 PopUp::sync() 中读取）
)>;
```

- 使用 std `mpsc` 而非 tokio channel，因为 PopUp 在同步上下文中接收
- `Wrapped` = `Box<dyn Wrapper>`，类型擦除 `Instance<C>` 的泛型参数
- `Wrapped::send()` 是 trait 方法，内部调用 `self.content.send(self.tx)`

### 多弹窗支持

`PopUp.content` 是 `Vec<Wrapped>`，支持弹窗队列：
- 最新弹窗在 `last()` 位置，优先接收按键
- 关闭后自动弹出，下一个弹窗接管

---

## 六、Agent 系统 — 可配置按键映射

### 设计

每个 Tab 的按键类型通过 `mod_agent!` 宏生成一个静态 `HashMap<KeyEvent, Key>`。

```rust
mod_agent!(
    Key,
    [
        (KeyCode::Enter,   Key::Select),
        (KeyCode::Char('i'), Key::Action(Action::Add)),
        // ...
    ]
);
```

### 工作流程

1. 启动时 `agent::init()` 从 `keymap.yaml` 加载配置
2. 如果文件不存在或为空，使用 `mod_agent!` 中定义的默认映射
3. `TryFrom<&KeyEvent>` 实现优先查 agent，agent 为空则回退到默认匹配

```rust
impl TryFrom<&KeyEvent> for Key {
    fn try_from(value: &KeyEvent) -> Result<Self, ()> {
        let agent = agent();
        if !agent.is_empty() {
            return agent.get(value).map(|k| *k).ok_or(());
        }
        // fallback matching...
    }
}
```

### keymap.yaml 格式

```yaml
keymap:
  file:
    profile:
      ? code: Enter
        modifiers: ''
        kind: Press
        state: ''
      : Select
      ? code: Char('i')
        modifiers: ''
        kind: Press
        state: ''
      : !Action Add
```

---

## 七、Theme 系统

位于 `src/tui/theme.rs`。

### 结构

```rust
struct Theme {
    popup: Popup,              // 弹窗颜色
    tab: Tab,                  // Tab 边框/高亮色
    bars: Bars,                // TabBar 文本色
    profile_tab: ProfileTab,   // Profile 专用色
    connection_tab: ConnectionTab,
    browser: Browser,
}
```

### 访问方式

```rust
Theme::get()  // → RwLockReadGuard<'static, Theme>
```

**热加载**：`--load-theme-realtime` 命令行参数启用后，每次 `get()` 都会从 `theme.yaml` 重新读取。

### 自定义主题

在配置目录创建 `theme.yaml`（仅在 `customized-theme` feature 启用时生效）。

---

## 八、render 与 sync 的顺序关系

```
每帧 (约 20ms):

  terminal.draw(|f| {
      app.render(f) ──────────────────────┐
         ├── render_tabbar()              │  纯绘制，不修改状态
         ├── active_tab.render(f, area)   │
         └── popup.render(f, area)        │
  })                                      │
                                          │
  app.sync() ─────────────────────────────┘
      ├── popup.sync()          ← 从 PAIR 取新弹窗
      └── tab.sync()
            └── while let Some(cb) = tasks.try_join_next() {
                   cb(&mut content);       ← 应用异步结果
                   content.after_sync();   ← 触发后续任务
                }

  注意：sync 在 render 之后，意味着：
  - 当前帧看到的是处理前状态
  - 下一次 render 才显示事务结果
  - 这避免了渲染中途状态变化导致的闪烁
```

---

## 九、完整事件流示例

以 Profile Tab 中按 `i` 添加配置为例：

```
1. 用户按 i
   │
2. App::handle_key_event()
   │  popup 未激活  →  非全局键  →  Tab 层
   │
3. Tab::handle_key_event()
   │  KeyEvent → Key::Action(Action::Add)
   │
4. Profile::handle_key_event(Key::Action(Add), ...)
   │  match Action::Add:
   │    action.act(name).spawn_at(task_set)
   │
5. tokio spawn async block:
   │   ① Input::new().with_title("Name").build_and_send().await
   │      │  └── PAIR.0.send() → PopUp 队列收到 Input 弹窗
   │      │  下一帧 sync() 后 popup 活跃
   │      │  用户输入 "my-profile"，按 Enter
   │      │  Input::match_key_event → Route::Send
   │      │  tx.send("my-profile") → oneshot 传回
   │      │
   │   ② Input::new().with_title("Url").build_and_send().await
   │      └── 同上，获得 url
   │
   │   ③ db::create(name, url)   ← 创建数据库记录
   │   ④ update_profile(pf).await ← 异步下载
   │
   │   ⑤ 返回 wrapper(|(content, _)| sync_helper(...))
   │
6. 下一帧 sync():
   │  while let Some(cb) = tasks.try_join_next():
   │     cb(&mut content)         ← 更新 items 和 atime 列表
   │     content.after_sync()
   │
7. 下一帧 render() 显示新项目
```
