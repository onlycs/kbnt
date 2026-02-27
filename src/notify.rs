use snafu::prelude::*;
use tauri_winrt_notification::Toast;

#[derive(Debug, Snafu)]
#[snafu(display("At {location}: Failed to send notification\n{source}"))]
pub(crate) struct NotifyError {
    source: tauri_winrt_notification::Error,
    #[snafu(implicit)]
    location: snafu::Location,
    title: &'static str,
    content: &'static str,
}

fn toast(title: &'static str, content: &'static str) -> Result<(), NotifyError> {
    Toast::new(Toast::POWERSHELL_APP_ID)
        .title(title)
        .text1(content)
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
const DISCONNECTED_MSG: &str = "Robot has disconnected. Paddles are now non-functional!";

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
