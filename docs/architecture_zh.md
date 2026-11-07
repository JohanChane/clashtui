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
│       ├── app.rs       # App 结构体、事件驱动循环、按键路由
│       ├── business.rs  # 业务提交与只读视图
│       ├── core.rs      # 服务/API 状态、请求所有权、页面数据槽
│       ├── app/
│       │   ├── actions.rs   # 全局动作、标签目标和默认绑定
│       │   └── tests.rs     # 全局快捷键和标签标题回归测试
│       ├── key.rs       # 按键、类型化序列、默认绑定宏和统一加载入口
│       ├── key/
│       │   ├── defaults.rs  # 类型化单键和双键声明
│       │   ├── config.rs    # 解析、覆盖、解绑、作用域内校验
│       │   ├── runtime.rs # 统一绑定存储、ID 索引、全局冲突校验
│       │   ├── matcher.rs   # App 持有的组合键状态机
│       │   └── notation.rs  # 按键记法解析和显示
│       ├── signals.rs   # OS 信号处理
│       ├── term.rs      # 终端初始化、带确认的借用、输入线程
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
│           ├── logs.rs       # 日志视图；logs/collector.rs 持有后台采集
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

`App::serve()` 用 `tokio::select!` 等待输入、弹窗、实际任务完成、日志变化、终端交接、退出及业务和显示期限，没有固定帧 tick。API 页面仅在可见时安排刷新期限；Files 有更新操作时，每 160 ms 推进一次转圈动画。

Business 持有 CoreBusiness 和 LogCollector。每次命令或核心任务完成，先提交状态、失效关联查询和日志会话，再推进待办工作、同步采集器。服务命令在日志失效后才启动。App 随后通过只读视图直接同步当前普通页面，在界面有变化且持有终端时绘制。没有组件 ID、订阅表或本地通知队列。窗口尺寸变化由输入线程投递。

退出时，App 先停止 Business，再让当前页 leave，然后停止弹窗和终端所有者。主循环继续接收完成，直到任务、日志采集、终端工作和输入线程真正退出。外部程序借用终端前，必须等输入线程确认停读。

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

`TuiWidget` 负责界面生命周期：

- `render(&mut self, f, area, view)` — 读取业务视图绘制界面
- `on_enter(&mut self, business)` / `on_leave(&mut self, business)` — 有序提交生命周期动作
- `sync_from(&mut self, view)` — 更新本地显示缓存，不启动工作
- `advance` — 由可见 Files 容器推进两侧加载
- `poll_tasks` / `finish_task` — 等待任务完成，应用仍属于当前可见期的回调

`TuiTab` 另外提供 `title`、`input_scope` 和 `handle_action`。页面容器接收已解析的动作；文本弹窗通过自身的 `handle_key_event` 处理原始按键。

### Tab 体系

#### 单面板（Tab）

`Tab<C>` 是泛型容器，`C` 需实现两个 trait：

- `BasicTabContent` — 定义 `Action`、`State`、标题、生命周期和只读同步
- `TabContent` — 定义 `init`、`handle_action`、`render`

#### 双面板（DualTab）

`DualTab<C1, C2>` 用于需要两个面板切换的场景（如 Files 标签页的 Profile 和 Template）。两个内容类型通过 `DualTabContent` / `DualTabContentMate` trait 互相引用。

两个可见面板直接同步，不受键盘焦点影响；焦点仅决定动作发给哪一侧。FileTab 为两侧各持有一个实际加载槽。

#### Tab 枚举

通过 `enum_dispatch!` 宏将所有标签页统一为一个 `Tab` 枚举。每个变体用 `newtype_tab!` 宏生成包装器并实现 `TuiWidget` 和 `TuiTab`。

### 异步任务模型

`init` 只初始化本地状态。`on_enter` 激活资源需求；`on_leave` 停止需求、清页面数据并保留偏好。`sync_from` 和 `render` 接收只读 BusinessView。Status、Settings 直接读取共享配置；Proxies、Connections 按数据 revision 更新必要的派生缓存。

`CoreBusiness` 持有服务/API 状态、请求句柄和已验收的页面数据。API 由系统服务的同一 core 进程提供：服务 Stopped 时 API 不可用，Running 后仍须确认 API；Unknown 保留服务查询诊断。API 检查等待当前服务确认 Running，以及该服务的待执行和在途检查结束。新服务检查废弃旧 API 回复，保留一次确认需求；API 成功不改写服务观察。普通 API 页可复用已确认的 Running，旧 Stopped／Unknown 则在进入或刷新时先复查当前服务。启动窗口内按服务、API 的顺序有限重试；外部启停没有即时发现保证。

请求只有仍持有有效所有权才能提交。服务变化先使不再有效的请求和日志连接失效，再推进待办工作。多阶段写操作持有可失效的 API 许可，在重载配置或每次批量请求前复核；恢复资格不会复活旧许可。

每个 QuerySlot 用一个 InFlight 直接返回任务结果，同时保存接纳资格、待刷新需求和期限。激活资源失效时保留需求及实际句柄；每次完成，包括废弃结果，都继续推进需求。Status、Proxies 成功后间隔 5 秒刷新，Connections 为 1 秒，Settings 无周期；错误停止自动续刷。Connections 暂停不会被手动刷新或 API 恢复解除。

`FutureSet<C>` 用 JoinSet 持有真实页面操作，用 Weak 标识回调所属可见期。离页废弃回调；实际操作仍完成、清 busy 并交付反馈。隐藏任务也参与 App 等待。Files 变更和手动刷新增加共享 revision；可见容器加载尚未尝试的版本。两侧 LoadSlot 跨离页保留实际 JoinHandle，拒收旧结果，完成后立即追赶最新 revision。失败也记为已尝试，仅新变更或重新进入才重试。

Files 选择配置先完成本地校验、合并、落盘和当前选择记录。API 许可只限制在线 reload；无许可或许可失效时，仍保存配置并提示启动或重启核心。更新当前配置后的自动应用复用同一入口，分别报告更新和应用结果。

`LogCollector` 属于 Business，隐藏时仍维护最多 300 条 `VecDeque<Arc<LogEntry>>` 记录，满后淘汰最旧记录。WebSocket 单消息和单帧均限制为 256 KiB。快照共享记录，不复制字符串；最后一个引用释放后回收。核心失效和暂停切换更换连接身份，拒收旧记录及诊断。采集先检查暂停再解析 JSON，追加前再次检查暂停和连接身份。

后台变化通过 Notify 合并唤醒。Logs 可见时，首次变化安排 34 ms 后的显示期限，后续通知不后移它。每轮绘制或等待前检查到期状态，即使 select 选中了输入或其他任务。仅到期、进入 Logs 或本地控制才复制快照；暂停、清空和级别变化立即显示并清旧期限。隐藏日志不触发普通页重绘；离页和退出取消期限。日志记录不进入弹窗队列。连接有有限超时和可中断退避，实际 worker 退出后才释放句柄；系统域名解析仍受平台解析器约束。

错误处理：
- `tri!()` 宏 — 捕获错误并弹窗提示用户
- `tri!(, or_cancel)` — 静默吞下错误

### 弹窗

后台任务通过有界通道提交弹窗：`let reply = Input::new().with_title(...).build_and_send_async().await;`，再用 `reply.await` 等待用户回答。通道满时施加背压。App 的同步回调把弹窗收集到本地，避免等待自己消费的队列。

`PopUp::check()` 和 `PopUp::handle_key_event()` 管理显示和输入；确认、错误消息使用同一入口。退出时关闭待答复请求，让等待中的操作能够结束。

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

`functions/` 提供命令、文件和 REST 操作；`tui/core.rs` 持有这些操作在 TUI 中的状态和请求生命周期。

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
6. `init` 只做本地初始化；生命周期动作经 Business 提交，`sync_from` 和 `render` 只读已提交数据，不启动工作。操作清理和反馈不得依赖可丢弃的页面回调。
