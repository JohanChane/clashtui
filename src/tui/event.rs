//! 页面能收到的全部事件。在 `on_event` 中 match 某个变体，就是订阅它。

use crate::config::{CONFIG, CoreType};

mod bus;
pub(in crate::tui) use bus::{Inbox, emit};

/// 页面能收到的全部事件。在 on_event 中 match 某个变体，就是订阅它。
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    // —— 生命周期：Hub 或 Host 定向投递给一个标签页，不能 emit ——
    /// 启动时每个标签页收到一次，早于任何 Shown。
    Init,
    /// 标签页切入。Split 的两侧同时收到。
    Shown,
    /// 标签页切出。此后不再收到 Tick。
    Hidden,
    /// 显示期间按 Page::TICK 发出。
    Tick,

    // —— 广播：任何地方 emit，投递给所有标签页和 CoreMonitor ——
    Core(CoreEvent),
    File(FileEvent),
}

/// 核心（mihomo / sing-box）的变化。
#[derive(Clone, Debug, PartialEq)]
pub enum CoreEvent {
    /// 核心可用性变化。只由 CoreMonitor 发出；状态不变时不发。
    StatusChanged(CoreStatus),
    /// clashtui 执行了服务命令（启动、停止、重启、安装、卸载）。执行完就发，无论成败。
    /// 它不是状态：CoreMonitor 据此重新确认 CoreStatus，CoreSrvCtl 据此刷新服务显示；页面不处理它。
    ServiceCommandDone,
    /// 配置的核心类型已切换。App 收到后退出。
    Switched(CoreType),
    /// 运行时配置被重载（选择或更新当前 profile）或修改（Settings）。
    ConfigChanged,
    /// Connections 页之外关闭了全部连接。
    ConnectionsClosed,
}

/// clashtui 管理的文件的变化。
#[derive(Clone, Debug, PartialEq)]
pub enum FileEvent {
    /// profile 列表或元数据变化：导入、删除、更新、选择、切换选项、由模板生成。
    ProfilesChanged,
}

/// 配置的核心走到了哪一步。服务状态和 API 状态合成这一个值：
/// 服务没运行，API 必然不可达；服务在运行，API 仍可能不可达。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum CoreStatus {
    /// 尚无确认的结果：启动时，或服务命令执行后等待确认。
    #[default]
    Unknown,
    /// 配置的核心的服务没有运行，API 必然不可达。
    Stopped,
    /// 服务在运行或无法判断，但 API 不可达。reason 直接显示给用户。
    Unreachable { reason: String },
    /// API 返回的是另一个核心的数据。
    Mismatch { running: CoreType },
    /// API 可用，且属于配置的核心。
    Ready { version: String },
}

impl CoreStatus {
    /// Ok 表示可以请求 API；Err 是页面应显示的原因。
    pub fn ready(&self) -> Result<(), String> {
        match self {
            Self::Ready { .. } => Ok(()),
            Self::Unknown => Err("Detecting core...".into()),
            Self::Stopped => Err("Core service is not running".into()),
            Self::Unreachable { reason } => Err(reason.clone()),
            Self::Mismatch { running } => Err(format!(
                "API returned {running} data, but {} is configured",
                CONFIG.core_type()
            )),
        }
    }
}

/// 可以广播的事件组。生命周期事件没有实现它，所以 emit(Event::Shown) 无法编译。
pub trait Broadcast: Into<Event> + Send + 'static {}
impl Broadcast for CoreEvent {}
impl Broadcast for FileEvent {}
impl From<CoreEvent> for Event {
    fn from(e: CoreEvent) -> Self {
        Self::Core(e)
    }
}
impl From<FileEvent> for Event {
    fn from(e: FileEvent) -> Self {
        Self::File(e)
    }
}
