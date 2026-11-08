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
│       ├── app/
│       │   ├── actions.rs   # 全局动作、标签目标和默认绑定
│       │   ├── hub.rs       # 标签页列表和 CoreMonitor；Init、Shown、Hidden、广播
│       │   └── tests.rs     # 全局快捷键和标签标题回归测试
│       ├── event.rs     # Event、CoreEvent、FileEvent、CoreStatus
│       ├── event/
│       │   └── bus.rs   # emit 和 Inbox
│       ├── page.rs      # Page、Cx、Host、Tick
│       ├── page/
│       │   └── split.rs # 双页面容器（Profile 和 Template）
│       ├── monitor.rs   # CoreMonitor 和状态快照
│       ├── ops.rs       # 操作模块声明与统一事件约定
│       ├── ops/
│       │   ├── service.rs     # 核心服务生命周期与完成事件
│       │   ├── profile.rs     # Profile 选择与更新
│       │   ├── config.rs      # 核心运行时配置修改
│       │   └── connections.rs # 连接关闭与事件
│       ├── key.rs       # 按键、类型化序列、默认绑定宏和统一加载入口
│       ├── key/
│       │   ├── defaults.rs  # 类型化单键和双键声明
│       │   ├── config.rs    # 解析、覆盖、解绑、作用域内校验
│       │   ├── runtime.rs # 统一绑定存储、ID 索引、全局冲突校验
│       │   ├── matcher.rs   # App 持有的组合键状态机
│       │   └── notation.rs  # 按键记法解析和显示
│       ├── signals.rs   # OS 信号处理
│       ├── term.rs      # 终端 raw mode 进入/退出/挂起
│       ├── theme.rs     # 主题加载
│       ├── utils.rs     # 工具函数
│       ├── popmsg.rs    # 弹窗定义（Confirm、Input 等）
│       ├── widget/
│       │   ├── mod.rs   # help、which、popmsg
│       │   ├── help.rs  # 可滚动帮助面板
│       │   ├── which.rs # 后续按键提示
│       │   └── popmsg.rs   # 弹窗容器（渲染 + 事件分发）
│       └── tab/
│           ├── mod.rs   # Tab 枚举、newtype_tab!/enum_dispatch! 宏
│           ├── status.rs     # 状态标签页
│           ├── files.rs      # FileTab（Split: Profile + Template）
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

`Key` 表示规范化终端输入；页面 `Action` 枚举表示命令。`KeySequence` 在配置和运行时统一区分单键与双键。`key_map!` 声明类型化默认动作和配置解析器。`Keymaps::from_config` 先应用核心专属配置、默认覆盖和解绑，再检查全局与每个页面的冲突。完整序列相同、或单键与双键前缀重叠时，报错并阻止启动；`gp` 与 `gg` 可以共享前缀。

`Keymaps` 统一拥有每条 `RuntimeBinding`。`scope_bindings` 按来源保存 ID，`key_lookup` 为各页面/焦点编译全局和本页的 ID 查找表，`tab_shortcuts` 缓存切页绑定 ID。Profile 和 Template 分别合成，Status 使用全局表。全部通过校验后，才安装到只读 `OnceLock`。

输入顺序：

1. 弹窗接管原始按键，并取消等待中的组合键。
2. 帮助处理滚动和关闭；启动全局双键时关闭帮助，该次序列仅使用全局候选。
3. App 的 `Matcher` 使用当前有效表，返回 `Pending`、`Execute(Action)`、`Cancelled` 或 `Unmatched`。
4. App 执行全局动作，或将类型化动作交给 `TuiTab::handle_action`。

唯一可变的组合键状态属于 App。切页、焦点变化和弹窗不会继承旧候选；Esc 或错误的第二键会被消费。帮助和标签栏借用同一绑定存储。Which 从原前缀分支借用候选，显示时排序；不持久复制描述或候选顺序。Which 标题显示当前前缀，各页面无需维护等待状态。

### TuiWidget Trait

`TuiWidget` 只给弹窗用：

- `render(&mut self, f, area)` — 绘制界面
- `sync(&mut self)` — 取回已完成的弹窗结果

`TuiTab` 是标签页接口：`title`、`input_scope`、`handle_action`、`render`、`deliver`、`sync(now)`。页面容器接收已解析的动作；文本弹窗通过自身的 `handle_key_event` 处理原始按键。

### Tab 体系

`Page` 是页面 trait：`on_event`、`on_action`、`render`，以及可选的 `TICK`。`Host<P>` 运行一个页面。事件任务和动作任务分开。下一次 `Tick` 只在页面显示、事件任务全部完成、并且再过 `TICK` 之后发出。动作任务（弹窗、测速）不推迟 `Tick`。

`Split<L, R>` 放两个页面（Profile 和 Template）。两侧都收到事件，只有焦点侧收到动作。焦点切换由 File 标签页处理。

`Hub` 持有 7 个标签页和 `CoreMonitor`。启动时每个标签页收到 `Init`，然后第一个收到 `Shown`。切页时先 `Hidden` 再 `Shown`。广播事件交给 `CoreMonitor` 和当前显示的标签页；隐藏标签页在 `Shown` 时重新加载。

`enum_dispatch!` 把各标签页新类型收成一个 `Tab` 枚举。`newtype_tab!` 用 `Host<P>` 包一层并实现 `TuiTab`。

### 异步任务模型

异步工作用 `FutureSet<P>`（即 `tokio::task::JoinSet`）：

- 页面用 `async { ... }.spawn_at(cx)` 派生任务
- 完成的任务返回 `Box<dyn FnOnce(&mut P) + Send>`
- `Host::sync(now)` 在 UI 线程应用这些回调，然后检查 `Tick`

`emit` 只把广播放进队列。`App::serve` 持有 `Inbox`，在 UI 线程上一次处理一个事件。生命周期事件不广播，由 `Hub` 和 `Host` 直接投递。

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
| `newtype_tab!` | `tab/mod.rs` | 生成包住 `Host` 的标签页包装器，实现 `TuiTab` |
| `enum_dispatch!` | `tab/mod.rs` | 将各 Tab 枚举变体分发到 trait 方法 |
| `load_save!` | `config/util.rs` | 为 YAML 配置类型生成 `from_file()` / `to_file()` |

### key_map! 宏

`key_map!(Logs, Action, DefaultBindings::new()...)` 声明 scope 对应的动作变体和默认绑定。`DefaultBindings::with_keys` 和 `with_chords` 分别声明单键与双键。配置中的嵌套页面直接解析为 `Scope`，无需中间路径字符串。生成的 `km::bindings` 应用覆盖，返回统一的类型化 `Action` 绑定供有效表合成。

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
2. 在 `key/runtime.rs` 添加 `Scope`、`Action` 变体和配置加载器，并在 `key/config.rs` 注册配置作用域。
3. 用 `key_map!` 声明默认绑定，用 `newtype_tab!(MyTab(Tab<MyContent>), MyScope)` 生成包装器；在 `tab/mod.rs` 的 `enum_dispatch!` 注册。
4. 在 `App::new()` 添加标签实例，并更新 `TabTarget` 及默认切页绑定；两者顺序必须与标签列表一致。
5. 双面板分别提供动作类型和作用域，由 `input_scope` 根据焦点选择有效表。
