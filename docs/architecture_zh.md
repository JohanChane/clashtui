# 开发文档

本文档描述 Clashtui 的代码架构，供开发者了解项目结构。


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
│       ├── key.rs       # 按键、默认绑定宏和统一加载入口
│       ├── key/
│       │   ├── config.rs    # 解析、覆盖、解绑、作用域内校验
│       │   ├── effective.rs # 合成页面/焦点绑定表、全局冲突校验
│       │   ├── matcher.rs   # App 持有的组合键状态机
│       │   └── notation.rs  # 按键记法解析和显示
│       ├── signals.rs   # OS 信号处理
│       ├── term.rs      # 终端 raw mode 进入/退出/挂起
│       ├── theme.rs     # 主题加载
│       ├── utils.rs     # 工具函数
│       ├── popmsg.rs    # 弹窗定义（Confirm、Input 等）
│       ├── widget/
│       │   ├── mod.rs   # new_type_impl_tuiwidget! 宏
│       │   ├── dualtab.rs  # 双面板标签页容器
│       │   ├── help.rs  # 可滚动帮助面板
│       │   ├── which.rs # 后续按键提示
│       │   ├── popmsg.rs   # 弹窗容器（渲染 + 事件分发）
│       │   └── tab.rs   # 单面板标签页容器 + BasicTabContent/TabContent trait
│       └── tab/
│           ├── mod.rs   # Tab 枚举、newtype_tab!/enum_dispatch! 宏
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

1. **CLI 解析** — 解析命令行参数和环境变量（`CLASHTUI_CONFIG_DIR`），处理提前退出（如 `--generate-shell-completion`）
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

### 有效绑定表与输入状态

`key_map!` 只声明类型化默认动作和配置解析器。`Keymaps::from_config` 先应用核心专属配置、默认覆盖和解绑，再检查全局与每个页面的冲突。完整序列相同、或单键与双键前缀重叠时，报错并阻止启动；`gp` 与 `gg` 可以共享前缀。

每个页面/焦点对应一份不可变 `EffectiveKeymap`。Files 的 Profile 和 Template 分别合成。Status 使用全局表。全部通过校验后，才将这些表安装到只读 `OnceLock`。

输入顺序：

1. 弹窗接管原始按键，并取消等待中的组合键。
2. 帮助处理滚动和关闭；启动全局双键时关闭帮助，该次序列仅使用全局候选。
3. App 的 `Matcher` 使用当前有效表，返回 `Pending`、`Execute(Action)`、`Cancelled` 或 `Unmatched`。
4. App 执行全局动作，或将类型化动作交给 `TuiTab::handle_action`。

唯一可变的组合键状态属于 App。切页、焦点变化和弹窗不会继承旧候选；Esc 或错误的第二键会被消费。Which、帮助和标签栏从同一份有效表读取数据。组合键分组名由 App 显示在顶部栏，各页面无需维护等待状态。

### TuiWidget Trait

`TuiWidget` 负责界面生命周期：

- `render(&mut self, f, area)` — 绘制界面
- `sync(&mut self)` — 推进异步任务
- `on_enter(&mut self)` / `on_leave(&mut self)` — 标签页切换回调

`TuiTab` 另外提供 `title`、`input_scope` 和 `handle_action`。页面容器接收已解析的动作；文本弹窗通过自身的 `handle_key_event` 处理原始按键。

### Tab 体系

#### 单面板（Tab）

`Tab<C>` 是泛型容器，`C` 需实现两个 trait：

- `BasicTabContent` — 定义类型化动作 `Key` 枚举、`State` 类型、标题
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
| `key_map!` | `key.rs` | 声明默认绑定和配置解析器；无运行时可变状态 |
| `newtype_tab!` | `tab/mod.rs` | 生成 Tab 包装器，实现 `TuiWidget` + `TuiTab` |
| `enum_dispatch!` | `tab/mod.rs` | 将各 Tab 枚举变体分发到 trait 方法 |
| `new_type_impl_tuiwidget!` | `widget/mod.rs` | 为新类型包装器自动实现 `TuiWidget` |
| `load_save!` | `config/util.rs` | 为 YAML 配置类型生成 `from_file()` / `to_file()` |

### key_map! 宏

`key_map!(Logs, Key, DefaultMap::new()...)` 声明 scope 对应的动作变体和默认绑定。生成的 `km::bindings` 应用覆盖，返回统一的类型化 `Action` 绑定供有效表合成。

`keymap.yaml` 使用 `[{on: j, action: MoveDown, desc: "下移"}]` 列表；`on` 支持单键或双键序列，`action: null` 用于解绑。配置格式和详细行为见 [入门文档](getting_started_zh.md)。

## 特性（Features）

| 特性 | 默认 | 说明 |
|------|------|------|
| `customized-theme` | ☑ | 自定义主题支持（自动启用 `tui`） |
| `tui` | (间接) | ratatui + crossterm + tokio 依赖 |
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

1. 定义内容类型，实现 `BasicTabContent` 和 `TabContent`（或双面板 traits）。
2. 在 `key/effective.rs` 添加 `Scope`、`Action` 变体和配置加载器，并在 `key/config.rs` 注册配置作用域。
3. 用 `key_map!` 声明默认绑定，用 `newtype_tab!(MyTab(Tab<MyContent>), MyScope)` 生成包装器；在 `tab/mod.rs` 的 `enum_dispatch!` 注册。
4. 在 `App::new()` 添加标签实例，并更新 `TabTarget` 及默认切页绑定；两者顺序必须与标签列表一致。
5. 双面板分别提供动作类型和作用域，由 `input_scope` 根据焦点选择有效表。
