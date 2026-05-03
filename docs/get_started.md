# demotui — 快速上手

## 环境要求

- Rust 工具链（stable，edition 2024）
- 终端：支持 true color 的现代终端（如 kitty, wezterm, alacritty, foot 等）

## 构建与运行

```sh
# 构建
cargo build

# 运行（debug 模式）
cargo run

# 运行（release 模式）
cargo run --release

# 仅检查编译
cargo check
```

## 首次运行

```sh
# 1. 创建默认配置目录（也可用 --config-dir 指定）
mkdir -p ~/.config/clashtui

# 2. 初始化配置文件
cargo run -- --config-dir ~/.config/clashtui

# 3. 程序会在配置目录自动创建需要的文件
```

### 配置文件结构

```
~/.config/clashtui/
├── config.yaml              # demotui 自身配置
├── basic_clash_config.yaml  # Mihomo 基础配置
├── clashtui.db              # 数据文件（profiles 等）
├── keymap.yaml              # 自定义按键映射（可选）
├── theme.yaml               # 自定义主题（可选，需 customized-theme feature）
├── profiles/                # 订阅配置目录
└── templates/               # 配置模板目录
```

### config.yaml 示例

```yaml
basic:
  clash_config_dir: '/srv/mihomo'
  clash_bin_path: '/usr/bin/mihomo'
  clash_config_path: '/srv/mihomo/config.yaml'
service:
  clash_service_name: 'mihomo'
  is_user: false
timeout: null
edit_cmd: ''
open_dir_cmd: ''
```

## 命令行参数

```sh
# 指定配置目录
cargo run -- --config-dir /path/to/config

# 生成 shell 补全脚本
cargo run -- --generate-shell-completion=bash

# 启用详细日志（多次 -v 提高日志级别）
cargo run -- -v -vv

# 启用主题热加载（修改 theme.yaml 无需重启）
cargo run -- --load-theme-realtime
```

## 键盘操作

### 全局按键

| 按键 | 功能 |
|------|------|
| `q` | 退出程序 |
| `1` - `2` | 切换到指定 Tab |
| `Tab` | 切换到下一个 Tab |

### 列表导航（Tab 内通用）

| 按键 | 功能 |
|------|------|
| `↑` / `k` | 上移 |
| `↓` / `j` | 下移 |

### FileTab — Profile 子面板

| 按键 | 功能 |
|------|------|
| `Enter` | 选择/应用 |
| `i` | 添加新配置（需输入名称和 URL） |
| `e` | 编辑配置 |
| `d` | 删除配置 |
| `u` | 更新配置 |
| `t` | 测试配置 |
| `p` | 预览配置内容 |
| `/` | 搜索过滤 |
| `←` / `→` | 切换到 Template 面板 |

### FileTab — Template 子面板

| 按键 | 功能 |
|------|------|
| `Enter` | 从模板生成配置到 Profile |
| `d` | 删除模板 |
| `p` | 预览模板 |
| `←` / `→` | 切换到 Profile 面板 |

### PopUp 弹窗通用按键

| 按键 | 功能 |
|------|------|
| `Enter` | 确认 |
| `Esc` | 取消/关闭 |
| `Tab` | 切换焦点（有 prompt 时在 prompt 和输入区之间切换） |

### Input 输入框

| 按键 | 功能 |
|------|------|
| `←` / `→` | 移动光标 |
| `Backspace` | 删除光标前字符 |
| `Delete` | 删除光标处字符 |
| `Enter` | 确认输入 |
| `Esc` | 取消输入 |

## 项目结构

```
src/
├── main.rs            入口：cli → config → tui 三阶段启动
├── cli.rs             命令行参数定义 (clap)
├── config.rs          配置加载/持久化
├── functions.rs
│   ├── command/       系统命令（systemctl 服务控制、编辑器打开等）
│   ├── file/          文件操作（profile 管理、template 管理）
│   └── restful/       Mihomo REST API 调用
└── tui.rs
    ├── app.rs         App 主循环 + 三层按键路由
    ├── agent.rs       可配置按键映射系统
    ├── theme.rs       主题系统（含热加载支持）
    ├── utils.rs       终端 raw mode 管理
    ├── tab/
    │   ├── mod.rs     Tab enum + TuiTab trait + enum_dispatch! 宏
    │   ├── status.rs  StatusTab — Mihomo 运行状态
    │   └── files.rs   FileTab — DualTab(Profile, Template)
    └── widget/
        ├── tab.rs     Tab<C: TabContent> 泛型容器 + FutureSet 事务系统
        ├── dualtab.rs DualTab<C1, C2> 7:3 双栏布局
        └── popmsg.rs  PopUp 弹窗系统 (Msg trait / Route / PAIR 通道)
```

## 开发：添加一个新 Tab

### 1. 定义内容类型和按键

```rust
// src/tui/tab/my_feature.rs

#[derive(Default)]
struct MyContent {
    items: Vec<String>,
}

#[derive(Clone, Copy)]
enum Key {
    MoveUp,
    MoveDown,
    Select,
    // ...
}

impl TryFrom<&KeyEvent> for Key {
    type Error = ();
    fn try_from(ev: &KeyEvent) -> Result<Self, ()> {
        // 简单的硬编码匹配，或通过 agent 系统
        if ev.kind != KeyEventKind::Press { return Err(()); }
        match ev.code {
            KeyCode::Up => Ok(Key::MoveUp),
            KeyCode::Down => Ok(Key::MoveDown),
            KeyCode::Enter => Ok(Key::Select),
            _ => Err(()),
        }
    }
}
```

### 2. 实现 BasicTabContent 和 TabContent

```rust
impl BasicTabContent for MyContent {
    type Key = Key;
    type State = ListState;
    const TITLE: &str = "MyFeature";
}

impl TabContent for MyContent {
    fn init(&mut self, tasks: &mut FutureSet<Self>, _: &mut Self::State) {
        // 加载初始数据
        async {
            // ... 异步操作 ...
            wrapper(|content: &mut Self| {
                content.items = vec!["Item 1".into(), "Item 2".into()];
            })
        }
        .spawn_at(tasks);
    }

    fn handle_key_event(&mut self, key: Key, tasks: &mut FutureSet<Self>, state: &mut Self::State) {
        match key {
            Key::MoveUp => state.select_previous(),
            Key::MoveDown => state.select_next(),
            Key::Select => {
                // 产生一个事务
                async {
                    // ... 异步操作 ...
                    do_nothing()  // 或无操作的闭包
                }
                .spawn_at(tasks);
            }
        }
    }

    fn render(&self, f: &mut Frame, area: Rect, state: &mut Self::State) {
        let widget = List::new(self.items.iter().map(ListItem::new))
            .block(Block::bordered().title(Self::TITLE))
            .highlight_style(Theme::get().tab.item_highlighted);
        f.render_stateful_widget(widget, area, state);
    }
}
```

### 3. 注册到系统

```rust
// src/tui/tab/mod.rs

// 添加 newtype
newtype_tab!(MyFeatureTab(Tab<MyContent>), "MyFeature");

// 在 enum_dispatch! 中注册
enum_dispatch!(
    pub enum Tab {
        FileTab,
        StatusTab,
        MyFeatureTab,  // 新增
    }
);

// 在 prelude 中导出
pub mod prelude {
    // ...
    pub use super::my_feature::MyFeatureTab;
}
```

```rust
// src/tui/app.rs

fn new() -> Self {
    Self {
        tabs: vec![
            StatusTab::default().into(),
            MyFeatureTab::default().into(),  // 新增
            FileTab::default().into(),
        ],
        // ...
    }
}

// 更新 TAB_COUNT 和按键映射
const TAB_COUNT: u8 = 3;
KeyCode::Char(c @ '1'..='3') => self.tab_index = c as u8 - '1' as u8,
```

## 开发注意事项

1. **不要在 render 中修改状态** — render 是 `&self`，修改应放在 sync 或 handle_key_event 中
2. **异步操作使用 spawn_at** — 将 async block 放入 FutureSet，在 sync 中统一推进
3. **事务闭包只修改数据** — 在 async block 中完成 I/O，返回的闭包仅负责赋值
4. **错误处理使用 tri! 宏** — 根据场景选择 `tri!` / `tri!(, or_cancel)` / `tri!(, or_set)`
5. **render 在 sync 之前** — 当前帧看到的是上轮 sync 的结果，避免闪烁

## Feature Flags

| Feature | 默认 | 说明 |
|---------|------|------|
| `tui` | 否 | 启用 TUI 模式（ratatui + crossterm + tokio） |
| `customized-theme` | 是 | 支持 YAML 主题自定义和热加载 |
| `migration_v0_2_3` | 否 | 从 ClashTUI v0.2.3 迁移配置 |
| `deprecated` | 否 | 已弃用的依赖 |

默认 features: `["customized-theme"]`（会自动启用 `tui` 和 `ratatui/serde`）。
