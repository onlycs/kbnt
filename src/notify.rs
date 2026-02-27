use core::fmt;

use snafu::prelude::*;
use tauri_winrt_notification::Toast;

#[derive(Debug, Snafu)]
#[snafu(display(
    "At {location}: Failed to send notification\n{source}\nTitle: {title}\nContent: {content}"
))]
pub(crate) struct NotifyError {
    source: tauri_winrt_notification::Error,
    #[snafu(implicit)]
    location: snafu::Location,
    title: String,
    content: String,
}

fn toast(title: impl AsRef<str>, content: impl AsRef<str>) -> Result<(), NotifyError> {
    let title = title.as_ref().to_string();
    let content = content.as_ref().to_string();

    crate::log::message(format!(
        "[{}] [{title}] {content}",
        chrono::Local::now().format("%Y-%m-%d_%H-%M-%S")
    ));

    Toast::new(Toast::POWERSHELL_APP_ID)
        .title(&title)
        .text1(&content)
        .show()
        .context(NotifySnafu { title, content })?;

    Ok(())
}

const ACTIVE_TITLE: &str = "KBNT: Started";
const ACTIVE_MSG: &str = "KBNT is now active and waiting for the DriverStation process";

const DS_TITLE: &str = "KBNT: DriverStation";
const DS_MSG: &str = "KBNT has detected the DriverStation and is waiting for robot connection";

const CONNECTED_TITLE: &str = "KBNT: Robot Connected";
const CONNECTED_MSG: &str = "KBNT has connected to the robot. Paddles are now functional!";

const DISCONNECTED_TITLE: &str = "KBNT: Robot Disconnected";
const DISCONNECTED_MSG: &str = "Robot has disconnected. Attempting reconnection...";
const DISCONNECTED_DS_MSG: &str = "Robot has disconnected. Waiting for DriverStation process...";

const ERROR_TITLE: &str = "KBNT: Error";
const ERROR_FILE_MSG: &str = "Please restart the app. See the log file for more info:";

pub(crate) fn active() -> Result<(), NotifyError> {
    toast(ACTIVE_TITLE, ACTIVE_MSG)
}

pub(crate) fn driverstation() -> Result<(), NotifyError> {
    toast(DS_TITLE, DS_MSG)
}

pub(crate) fn connected() -> Result<(), NotifyError> {
    toast(CONNECTED_TITLE, CONNECTED_MSG)
}

pub(crate) fn disconnected() -> Result<(), NotifyError> {
    toast(DISCONNECTED_TITLE, DISCONNECTED_MSG)
}

pub(crate) fn disconnected_ds() -> Result<(), NotifyError> {
    toast(DISCONNECTED_TITLE, DISCONNECTED_DS_MSG)
}

pub(crate) fn error(filename: impl fmt::Display) -> Result<(), NotifyError> {
    toast(ERROR_TITLE, format!("{ERROR_FILE_MSG} {filename}"))
}
