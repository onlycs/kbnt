use std::{io::Write, path::PathBuf};

use crate::AppError;

pub(crate) fn message(entry: String) {
    let install_dir = crate::install::dir_infallible();
    let log_path = install_dir.join("kbnt.log");

    if let Err(e) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .and_then(|mut file| writeln!(file, "{}", entry))
    {
        eprintln!("Failed to write to log file: {}", e);
    }
}

pub(crate) fn error(app: AppError) -> PathBuf {
    let install_dir = crate::install::dir_infallible();
    let log_path = install_dir.join(format!(
        "kbnt_error_{}.log",
        chrono::Local::now().format("%Y-%m-%d_%H-%M-%S")
    ));

    let message = format!("{}", app);

    if let Err(e) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .and_then(|mut file| writeln!(file, "{}", message))
    {
        eprintln!("Failed to write to log file: {}", e);
    }

    log_path
}
