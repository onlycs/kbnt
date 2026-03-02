use std::{
    io::{self, Write},
    path::PathBuf,
};

use crate::AppError;

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

pub(crate) struct LogWriter;

impl Write for LogWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let install_dir = crate::install::dir_infallible();
        let log_path = install_dir.join("kbnt.log");

        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)?
            .write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
