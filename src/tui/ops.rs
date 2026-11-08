//! 被多处调用、会改变核心或文件的操作。成功后自己发出事件，调用方无需 emit。
//! 服务命令执行完就发 ServiceCommandDone，无论成败。

use anyhow::{Context, Result, ensure};

use crate::config::database::Profile;
use crate::config::{CONFIG, CoreType};
use crate::functions::file::profile::{self, SelectionOutcome, UpdateResult};
use crate::functions::{command, restful};
use crate::tui::event::{CoreEvent, FileEvent, emit};

pub fn restart_core() -> Result<String> {
    service(command::restart_service())
}
pub fn stop_core() -> Result<String> {
    service(command::stop_service())
}
pub fn stop_all_cores() -> Result<String> {
    service(command::stop_all_services())
}
pub fn start_core(core: CoreType) -> Result<String> {
    service(command::start_core_service(core))
}
#[cfg(windows)]
pub fn install_core(core: CoreType) -> Result<String> {
    service(command::install_core_service(core))
}
#[cfg(windows)]
pub fn uninstall_core(core: CoreType) -> Result<String> {
    service(command::uninstall_core_service(core))
}

/// 停止所有核心、保存新的核心类型、启动新核心，返回给用户看的报告。
/// 内部经 stop_all_cores、start_core，各自发出 ServiceCommandDone。Switched 由调用方在用户确认后发出。
pub fn switch_core(core: CoreType) -> Result<String> {
    let stop = stop_all_cores();
    CONFIG.data.lock().unwrap().core_type = core;
    CONFIG.save().context("save core type")?;
    let start = start_core(core);
    let label = core.to_string();
    Ok(format!(
        "Core switched to {label}\n\nStop all services: {stop}\nStart {label}: {start}",
        stop = brief(&stop),
        start = brief(&start),
    ))
}

pub fn patch_config(payload: String) -> Result<()> {
    restful::config::patch(payload).map_err(|err| anyhow::anyhow!("{err}"))?;
    emit(CoreEvent::ConfigChanged);
    Ok(())
}

pub fn close_all_connections() -> Result<()> {
    restful::connection::terminate_all_connections().map_err(|err| anyhow::anyhow!("{err}"))?;
    emit(CoreEvent::ConnectionsClosed);
    Ok(())
}

/// 保存选择并让核心重载配置。
pub async fn select_profile(profile: Profile) -> Result<()> {
    profile::select(profile).await?;
    emit(CoreEvent::ConfigChanged);
    emit(FileEvent::ProfilesChanged);
    Ok(())
}

/// 更新 profile；若它是当前 profile，底层会重新选择并重载。
pub async fn update_profile(profile: Profile, with_proxy: bool) -> Result<UpdateResult> {
    let result = profile::update_profile(profile, with_proxy).await?;
    emit(FileEvent::ProfilesChanged);
    if matches!(result.selection, Some(Ok(SelectionOutcome::Reloaded))) {
        emit(CoreEvent::ConfigChanged);
    }
    Ok(result)
}

/// 服务命令执行完就发 ServiceCommandDone，无论成败：失败的命令也可能改变服务状态。
/// 同时把以 "Error" 开头的输出统一转成 Err，调用方不再各自判断。
fn service(output: Result<String>) -> Result<String> {
    emit(CoreEvent::ServiceCommandDone);
    let output = output?;
    ensure!(!output.starts_with("Error"), "{output}");
    Ok(output)
}

fn brief(result: &Result<String>) -> String {
    match result {
        Ok(text) => text.trim().to_owned(),
        Err(err) => err.to_string(),
    }
}
