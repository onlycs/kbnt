use directories::ProjectDirs;
use std::io::Write;
use std::path::PathBuf;

use crate::AppError;

pub(crate) fn message(entry: String) {
    let config_dir = crate::install::dir_infallible();
    let log_path = config_dir.join("kbnt.log");

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
    let mut config_dir = match ProjectDirs::from("org", "team2791", "kbnt") {
        Some(dirs) => dirs.config_dir().to_path_buf(),
        None => PathBuf::from("C:\\Users\\Default"),
    };

    if !config_dir.exists() {
        if let Err(_) = std::fs::create_dir_all(&config_dir) {
            config_dir = PathBuf::from("C:\\Users\\Default");
        }
    }

    let log_path = config_dir.join(format!(
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
