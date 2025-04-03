use super::consts;

pub fn setup(level: u8) {
    let log_path = consts::LOG_PATH.as_path();
    // auto rm old log for debug
    #[cfg(debug_assertions)]
    let _ = std::fs::remove_file(log_path);
    // remove the log file if too big
    let flag = if std::fs::metadata(log_path).is_ok_and(|m| m.len() > 1024 * 1024) {
        let _ = std::fs::remove_file(log_path);
        true
    } else {
        false
    };

    let level = level + if cfg!(debug_assertions) { 4 } else { 2 };
    let log_level = log::LevelFilter::iter()
        .nth(level as usize)
        .unwrap_or(log::LevelFilter::max());

    let log_file = std::fs::File::create(log_path).unwrap();
    env_logger::builder()
        .filter_level(log_level)
        .format_timestamp_micros()
        .target(env_logger::Target::Pipe(Box::new(log_file)))
        .init();

    log::info!("{}", "-".repeat(20));
    log::debug!("Start Log, level: {}", log_level);
    if flag {
        log::info!("Old log file too large, cleared")
    }
}

/// read file by lines, from `total_len-start-length` to `total_len-start`
pub fn logcat(start: usize, length: usize) -> std::io::Result<Vec<String>> {
    use crate::utils::consts::LOG_PATH;
    use std::io::BufRead as _;
    use std::io::Seek as _;

    let mut fp = std::fs::File::open(LOG_PATH.as_path())?;
    let size = {
        let fp = fp.try_clone()?;
        std::io::BufReader::new(fp).lines().count()
    };
    fp.seek(std::io::SeekFrom::Start(0))?;
    let fp = std::io::BufReader::new(fp).lines();
    let start = size.saturating_sub(start + length);
    let vec = fp
        .skip(start)
        .take(length)
        .collect::<std::io::Result<_>>()?;
    Ok(vec)
}
