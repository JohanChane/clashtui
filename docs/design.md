# demotui — 设计意图

## 一、项目定位

**demotui** 是 ClashTUI 的重构版本，分两个阶段演进：

1. **第一阶段**：完整重构 ClashTUI 业务逻辑，使用新框架架构
2. **第二阶段**：剥离业务代码，抽象为通用的 TUI 开发框架（Demo），可供其他 TUI 项目复用

最终目标：服务于 Mihomo (Clash.Meta) 用户，同时提供一套可复用的 Rust TUI 框架模式。

## 二、设计参考

### 2.1 yazi（终端文件管理器）

- 借鉴其**模块化架构**（独立 crate 划分）
- **不引入** Act（Actor）模式 — yazi 的 Act 主要服务于配置同步，对本项目过于复杂

### 2.2 原 ClashTUI

- 保留核心业务逻辑
- 摒弃以下遗留问题：
  - EventState (Consumed/NotConsumed) 返回值模式 — 难维护
  - 后端事件循环 — 代码膨胀，枚举返回类型繁琐
  - 过多弹窗 — 体验不佳
  - 事件层层传递 — 耦合度高
  - Ctrl+C 无法退出（Raw mode 副作用）

## 三、核心架构决策

### 3.1 异步模型：Callback + oneshot，无后端事件循环

**为什么不使用事件循环**：ClashTUI 使用异步的核心理由只有网络请求（订阅服务器、Mihomo API），不应为此引入完整的事件循环架构。

**方案**：后端操作直接 spawn，通过 `tokio::sync::oneshot` 将结果传回。

```
前端                   后端（全局/静态）
  │                       │
  │── Request::spawn() ──►│  tokio::spawn(async { ... tx.send(result) })
  │                       │
  │◄── oneshot::Receiver ─│
  │                       │
```

- 无需后端事件循环，无需枚举返回类型
- 请求携带 callback，结果直达调用方
- 适合与 JoinSet（自动推进事务）配合

### 3.2 关键事件路由：PopUp → App → Tab（三层，无返回值）

不再使用 EventState 返回值判断是否消费。三层级联：

1. **PopUp 层**：弹出层拿走所有输入
2. **App 层**：全局快捷键（退出、切换 Tab、帮助等）
3. **Tab 层**：当前活跃 Tab 处理剩余输入

每一层直接处理或放行，无需层层返回状态码。

### 3.3 通用 Tab 抽象：`Tab<C: TabContent>`

```rust
struct Tab<C: TabContent> {
    content: C,
    state: TabState,
    tasks: JoinSet<...>,
}

trait TabContent {
    type Key;
    fn handle_key_event(&mut self, key: Self::Key, tasks: &mut JoinSet, state: &mut TabState);
    fn render(&mut self, f: &mut Frame, area: Rect, state: &TabState);
}
```

- `Tab<C>` 负责通用逻辑（状态管理、事务推进）
- `C` 负责具体渲染与按键处理
- 事务（JoinSet 中的闭包）在 `handle_key_event` 中自动推进并应用到 Tab

### 3.4 事务模式：按键 → 闭包 → JoinSet

```rust
// 按键产生一个事务（闭包），放入 JoinSet 自动推进
fn handle_key_event(&mut self, key: Key, tasks: &mut JoinSet, state: &mut State) {
    tasks.spawn(async move {
        // 异步操作（网络请求等）
        let result = fetch_something().await;
        // 返回闭包，由 Tab 应用到自身
        move |content: &mut C| {
            content.apply(result);
        }
    });
}

// 在 render 前推进已完成的事务
fn sync(&mut self) {
    while let Some(f) = self.tasks.try_join_next() {
        (f.unwrap())(&mut self.content);
    }
}
```

- 无需 match 枚举返回值
- 返回的闭包自带类型上下文，直接修改对应数据结构

### 3.5 PopUp 集成 oneshot

```rust
struct PopUp {
    tx: Option<oneshot::Sender<UserInput>>,
    // ...
}
```

- 弹出输入框时创建 oneshot 通道
- 用户确认后通过 tx 将结果传回事务上下文
- 支持多个 PopUp 同时存在（队列）

## 四、UI 设计

### 4.1 布局

```
┌──────────────────────────────────────────┐
│ 1 Status    2 Proxies    3 Files    4 ... │  ← TabBar (3行)
├──────────────────────────────────────────┤
│                                          │
│          主内容区                          │
│                                          │
│      (支持 7:3 双栏布局，见下)              │
│                                          │
├──────────────────────────────────────────┤
│ Status info                          ... │  ← StatusBar (3行)
└──────────────────────────────────────────┘
```

**双栏模式**（如 Profile ↔ Template）：

```
┌────────────────────┬──────────┐
│                    │          │
│   Profile (70%)    │ Template │
│                    │  (30%)   │
│                    │          │
└────────────────────┴──────────┘
```

### 4.2 Tab 功能规划

| Tab | 功能 |
|-----|------|
| Status | Mihomo 状态、运行模式、端口信息 |
| Proxies | 代理节点管理、延迟测试、选择 |
| Files | 配置文件浏览、编辑 |
| Connect | 连接管理 |
| Profile | 订阅管理（导入/更新/选择） |
| Template | 配置模板系统 |
| ClashSrvCtl | 服务控制（start/stop/restart） |

> 初期先用 Status + Files 验证框架，逐步加入业务 Tab。

### 4.3 状态显示原则：内联优先，减少弹窗

| 情景 | 旧方式 | 新方式 |
|------|--------|--------|
| 选中项 | 弹窗提示 | 列表项前加 `*` 标记 |
| 处理中 | 弹窗等待 | 列表项前加动画字符（`-/|\`） |
| 错误 | 弹窗展示 | 底部 StatusBar 或行内红色标记 |
| 确认操作 | 弹窗确认 | 底部 Prompt 行（单行确认） |

只有在需要**用户输入**或**多选题**时才使用 PopUp。

### 4.4 进度显示

- Tab 标题处显示动画：`Profile-/` `Profile\|` 循环
- 帮助区分"正在加载"和"空闲"状态

## 五、模块架构

```
src/
├── main.rs           入口
├── cli.rs            命令行解析 (clap)
├── cli/              命令行子模块
├── config.rs         配置入口
├── config/           配置管理 (YAML 反序列化、持久化)
├── functions.rs      业务逻辑入口
├── functions/
│   ├── command/      系统命令（systemctl 等服务控制）
│   ├── file/         文件操作
│   └── restful/      Mihomo RESTful API
├── tui.rs            TUI 入口
└── tui/
    ├── app.rs        应用主循环、三层路由
    ├── agent.rs      按键映射（YAML → KeyEvent HashMap）
    ├── theme.rs      主题系统（YAML 自定义颜色）
    ├── utils.rs      终端原始模式管理
    ├── tab/          Tab 实现
    │   ├── mod.rs    TuiTab trait, Tab enum
    │   ├── files.rs  FileTab
    │   └── status.rs StatusTab
    └── widget/       可复用组件
        ├── mod.rs    new_type_impl_tuiwidget! 宏
        ├── tab.rs    Tab<C> 通用容器
        ├── dualtab.rs 7:3 双栏布局
        └── popmsg/   弹窗系统（Input/Choice/Single/Multi）
```

## 六、关键设计原则

1. **TUI 同步，后端异步**：UI 部分保持同步逻辑（渲染、按键分发），只有 I/O 操作 spawn 异步任务
2. **泛型优于枚举**：`Tab<C>` 代替大枚举，新增 Tab 只需实现 `TabContent`
3. **KeyMap 可配置**：默认提供 Vim-like 键位，用户可通过 `keymap.yaml` 覆盖
4. **主题可自定义**：通过 `theme.yaml` 覆盖颜色方案（通过 serde 反序列化）
5. **减少弹窗**：信息优先内联展示，弹窗仅用于需要用户交互的场景

## 七、依赖选择

| 用途 | 依赖 | 原因 |
|------|------|------|
| TUI | ratatui + crossterm | 成熟稳定，生态好 |
| 异步运行时 | tokio (multi-thread) | 网络请求、JoinSet |
| CLI | clap (derive) | 比 argh 更灵活，支持 shell completion |
| 配置 | serde + serde_yml | YAML 配置反序列化 |
| HTTP | minreq (https, proxy) | 轻量级，支持代理 |
| 日志 | env_logger | 简洁，与 tokio 兼容 |

## 八、演进路线

1. ~~搭建基础框架~~（已完成：main.rs + cli + config + tui 骨架）
2. 完善 `Tab<C>` 泛型容器与事务系统
3. 实现双栏布局 `DualTab`
4. 迁移 ClashTUI 的 Profile/Template 业务逻辑
5. 实现 Proxies 节点管理（延迟测速、选择、过滤）
6. 实现 Connect 连接管理
7. 添加鼠标支持和 CSI u 协议
8. 剥离业务代码，生成通用 TUI 框架文档和示例

## 九、参考

- [GitHub Discussion #1 — 关于 ClashTUI 的框架设计](https://github.com/JohanChane/demotui/discussions/1)
- [yazi — 终端文件管理器](https://github.com/sxyazi/yazi)
- [ClashTUI — Mihomo TUI Client](https://github.com/JohanChane/clashtui)
