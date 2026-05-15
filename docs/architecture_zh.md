# 开发文档

本文档描述 Clashtui 的代码架构，供开发者了解项目结构。

> AGENTS.md 是更完整的参考，包含所有宏、约定、细节。本文档侧重于架构概览。

## 技术栈

- 语言：Rust（Edition 2024）
- 构建系统：Cargo
- TUI 框架：ratatui + crossterm
- 异步运行时：tokio
- 配置/持久化：serde + serde_yml
- HTTP 客户端：minreq
- CLI 解析：clap

## 目录结构

```
src/
├── main.rs              # 入口：五阶段启动流程
├── cli.rs               # CLI 模块入口，re-exports from cli/
│   └── cli/
│       ├── handler.rs   # CLI 子命令处理（profile/service/mode/update）
│       ├── widgets.rs   # CLI 交互组件（Confirm、Select）
│       └── utils.rs     # 版本信息、shell 补全生成
├── config.rs            # 配置模块入口，re-exports from config/
│   └── config/
│       ├── core.rs      # CoreType 枚举、ServiceController
│       ├── database.rs  # ProfileManager（Profile 持久化结构）
│       └── util.rs      # 配置目录解析、load_save! 宏、文件路径常量
├── functions.rs         # 业务逻辑入口，re-exports from functions/
│   └── functions/
│       ├── command.rs   # 系统操作（文件权限、服务启停、目录打开）
│       ├── file.rs      # 文件操作（Profile 导入/更新、Template 处理）
│       └── restful.rs   # REST API 调用（config/mode/proxies/connections）
├── tui.rs               # TUI 模块入口，re-exports from tui/
│   └── tui/
│       ├── app.rs       # App 结构体、事件循环（~50fps）、按键路由
│       ├── agent.rs     # 按键映射加载（keymap.yaml）
│       ├── key.rs       # Key 结构体（code + 修饰键）
│       ├── signals.rs   # OS 信号处理
│       ├── term.rs      # 终端 raw mode 进入/退出/挂起
│       ├── theme.rs     # 主题加载
│       ├── utils.rs     # 工具函数
│       ├── keymap_default.yaml  # 默认按键映射
│       ├── popmsg.rs    # 弹窗定义（Confirm、Input 等）
│       ├── widget/
│       │   ├── mod.rs   # new_type_impl_tuiwidget! 宏
│       │   ├── chord.rs # 组合键处理器
│       │   ├── dualtab.rs  # 双面板标签页容器
│       │   ├── fzffind.rs  # 模糊搜索组件
│       │   ├── help.rs  # 帮助面板
│       │   ├── popmsg.rs   # 弹窗容器（渲染 + 事件分发）
│       │   └── tab.rs   # 单面板标签页容器 + BasicTabContent/TabContent trait
│       └── tab/
│           ├── mod.rs   # Tab 枚举、newtype_tab!/enum_dispatch! 宏、agent 宏
│           ├── status.rs     # 状态标签页
│           ├── files.rs      # FileTab（DualTab: Profile + Template）
│           ├── proxies.rs    # 代理标签页
│           ├── connections.rs # 连接标签页
│           ├── logs.rs       # 日志标签页
│           ├── settings.rs   # 设置标签页
│           └── srvctl.rs     # 核心服务控制标签页
```

## 启动流程（5 阶段）

程序入口在 `src/main.rs`，按顺序执行五个阶段：

1. **CLI 解析** — 解析命令行参数和环境变量（`CLASHTUI_CONFIG_DIR`），处理提前退出（如 `--generate-shell-completion`、`migrate`）
2. **配置初始化** — 确定配置目录，加载 `config.yaml` + `clashtui.db`，创建缺失的目录和文件
3. **TUI 初始化** — 加载按键映射（`keymap.yaml`）、主题（`theme.yaml`）、设置终端 raw mode、注册 panic hook
4. **事件循环** — 运行 `App::serve()`，循环处理渲染、事件和异步任务
5. **恢复与保存** — 退出 raw mode，保存 `clashtui.db`

若命令行有子命令（`profile`、`service`、`mode`、`update`），则跳过阶段 3-5，执行子命令后直接退出。

## 配置系统

### 配置目录解析

优先级从高到低：
1. `--config-dir` 命令行参数
2. `CLASHTUI_CONFIG_DIR` 环境变量
3. 可执行文件所在目录的 `data/` 子目录（便携模式）
4. `$XDG_CONFIG_HOME/clashtui`
5. `~/.config/clashtui`

### 配置加载

- `ConfigFile` — 从 `config.yaml` 加载核心路径和服务配置
- `BasicInfo` — 从 `core_override_config.yaml` 加载 API 地址、密钥等
- `ProfileManager` — 从 `clashtui.db` 加载 Profile 列表和当前选择
- 以上三者合并为 `Config` 结构体，通过 `config::CONFIG` 全局访问

### 持久化

使用 `load_save!` 宏自动生成 `from_file()` 和 `to_file()` 方法。格式为 YAML。

## TUI 架构

### 事件循环

运行在 `App::serve()` 中，以约 50fps（20ms/帧）循环执行：

```
每帧流程：
1. 处理 resize（原子标志，在帧顶部处理避免竞态）
2. terminal.draw(render) — 渲染当前帧
3. sync() — 推进完成的异步任务
4. tokio::select! 等待下一事件（按键/tick/resize）
5. 处理按键事件
```

### 按键路由（六层）

按键按以下顺序被处理，命中即停：

| 层级 | 处理器 | 作用 |
|------|--------|------|
| 0 — PopUp | `popup.handle_key_event` | 弹窗/对话框劫持所有按键 |
| 0.5 — GlobalChord | `global_chord.handle` | 全局组合键（如 Ctrl-g c 打开配置目录） |
| 1 — Help | `help.dismiss` | 帮助面板打开时，按任意键关闭 |
| 2 — Chord | `chord.handle` | 标签页级别的多键组合 |
| 3 — Tab | `tabs[ti].handle_key_event` | 当前标签页处理按键 |
| 4 — Global | `handle_global_kv` | 标签页切换（1-7、Tab）、退出（q、Ctrl-c）、帮助（?） |

### TuiWidget Trait

所有可渲染、可处理按键的元素都实现 `TuiWidget` trait：

- `handle_key_event(&mut self, kv: &Key)` — 处理按键
- `render(&mut self, f, area)` — 绘制界面
- `sync(&mut self)` — 推进异步任务
- `on_enter(&mut self)` / `on_leave(&mut self)` — 切换标签页时的回调

渲染阶段不要修改状态（`render` 接受 `&self`）。状态变更应在 `handle_key_event` 或 `sync` 回调中完成。

### Tab 体系

#### 单面板（Tab）

`Tab<C>` 是泛型容器，`C` 需实现两个 trait：

- `BasicTabContent` — 定义 `Key` 枚举（哪些按键触发）、`State` 类型、标题
- `TabContent` — 定义 `init`、`handle_key_event`、`render`

#### 双面板（DualTab）

`DualTab<C1, C2>` 用于需要两个面板切换的场景（如 Files 标签页的 Profile 和 Template）。两个内容类型通过 `DualTabContent` / `DualTabContentMate` trait 互相引用。

#### Tab 枚举

通过 `enum_dispatch!` 宏将所有标签页统一为一个 `Tab` 枚举。每个变体用 `newtype_tab!` 宏生成包装器并实现 `TuiWidget` 和 `TuiTab`。

### 异步任务模型

异步 I/O 操作通过 `FutureSet<C>`（即 `tokio::task::JoinSet`）管理：

- 在 `handle_key_event` 或 `init` 中通过 `task_set.spawn(async { ... })` 生成异步任务
- 任务完成后产生 `Callback<C>`（即 `Box<dyn FnOnce(&mut C)>`）
- `sync()` 会在每帧推进已完成的回调，状态变更统一在此发生

错误处理：
- `tri!()` 宏 — 捕获错误并弹窗提示用户
- `tri!(, or_cancel)` — 静默吞下错误

### 弹窗

弹窗通过 `oneshot` channel 模式实现：
- 调用 `Input::new().with_title(...).build_and_send().await` 阻塞等待用户输入
- 弹出事件由 `PopUp::check()` 和 `PopUp::handle_key_event()` 管理
- 仅在需要用户输入时使用弹窗；简单确认/错误用内联状态显示

## 核心宏

| 宏 | 位置 | 用途 |
|-----|------|------|
| `tri!` | `tab/mod.rs` | 异步回调中的错误处理 |
| `mod_agent!` | `tab/mod.rs` | 定义标签页默认按键绑定和组合快捷键 |
| `newtype_tab!` | `tab/mod.rs` | 生成 Tab 包装器，实现 `TuiWidget` + `TuiTab` |
| `enum_dispatch!` | `tab/mod.rs` | 将各 Tab 枚举变体分发到 trait 方法 |
| `new_type_impl_tuiwidget!` | `widget/mod.rs` | 为新类型包装器自动实现 `TuiWidget` |
| `load_save!` | `config/util.rs` | 为 YAML 配置类型生成 `from_file()` / `to_file()` |

### mod_agent! 宏

两种 key 定义方式：
- `[KeyCode::Char('j')]` — 普通字符键
- `key("<C-a>")` — 修饰键语法（C=Ctrl, A=Alt, S=Shift）

支持两种 keymap.yaml 格式：
- **Mapping**: `j: SelectDown`（简单，不含描述和组合键）
- **Sequence**: `[{on: j, action: SelectDown, desc: "下移"}]`（含描述，支持组合键）

## 特性（Features）

| 特性 | 默认 | 说明 |
|------|------|------|
| `customized-theme` | ☑ | 自定义主题支持（自动启用 `tui`） |
| `tui` | (间接) | ratatui + crossterm + tokio 依赖 |
| `migration_v0_2_3` | ☐ | v0.2.3 配置迁移 |
| `deprecated` | ☐ | 废弃功能 |

使用 `#[cfg(feature = "tui")]` 而非 `#[cfg(feature = "customized-theme")]` 进行条件编译，除非是主题专用代码。

## 业务逻辑

`functions/` 目录包含所有业务逻辑，分为三个模块：

| 模块 | 职责 |
|------|------|
| `command` | 系统级操作：服务启停（systemd）、文件权限修复、打开目录、文件编辑器 |
| `file` | Profile 管理：导入、更新（下载 + 解析）、Template 展开、订阅类型检测 |
| `restful` | REST API：获取/设置 config、切换 proxies、查询 connections、获取日志 |

## 构建脚本

`build.rs` 通过 git 生成版本号，格式为 `{CARGO_PKG_VERSION}-{git-short-hash}[-dirty]`，保存在 `CLASHTUI_VERSION` 环境变量中。

## 版本命名约定

- crate 名为 `clashtui`（Cargo.toml）
- 内部标识使用 `clashtui`（`CLASHTUI_VERSION`、`CLASHTUI_CONFIG_DIR`、配置目录 `~/.config/clashtui`）
- 所有环境变量、YAML key、用户可见字符串均使用 `clashtui`

## 添加标签页

大致步骤：

1. 定义内容类型，实现 `BasicTabContent` + `TabContent`（或 `DualTabContent`）
2. 在 `tab/mod.rs` 中：
   - 添加 `mod mytab;`
   - 用 `newtype_tab!` 生成包装器
   - 在 `prelude` 的 `enum_dispatch!` 和 agent_init 中注册
   - 在 `Tab` 枚举中添加变体
3. 在 `app.rs` 中：
   - `App::new()` 的 `tabs` vec 中添加实例
   - 更新 `TAB_COUNT` 和 `'1'..='7'` 范围
   - 在 `prelude::agent_init` 调用 init
4. 若为双面板：两个内容类型通过 `DualTabContent` / `DualTabContentMate` 互相指定关联类型
