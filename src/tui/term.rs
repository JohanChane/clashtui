use super::utils::raw_mode;

pub fn setup() -> anyhow::Result<()> {
    raw_mode::setup()?;
    raw_mode::set_panic_hook();
    Ok(())
}

pub fn teardown() {
    let _ = raw_mode::restore();
}

pub fn hold(on: bool) -> anyhow::Result<()> {
    if on {
        raw_mode::restore()?;
    } else {
        raw_mode::setup()?
    }
    super::app::FULL_RENDER.notify_one();
    Ok(())
}
