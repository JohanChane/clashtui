use super::utils::raw_mode;

pub fn setup() -> anyhow::Result<()> {
    raw_mode::setup()?;
    raw_mode::set_panic_hook();
    Ok(())
}

pub fn teardown() {
    let _ = raw_mode::restore();
}

#[cfg(unix)]
pub fn hold(on: bool) -> anyhow::Result<()> {
    if on {
        raw_mode::restore()?;
        super::app::FULL_RENDER.notify_one();
    } else {
        raw_mode::setup()?
    }
    Ok(())
}
