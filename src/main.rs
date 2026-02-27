mod notify;
mod wmi;

use snafu::prelude::*;

const APP_ID: &str = "KBNT";

#[derive(Debug, Snafu)]
pub(crate) enum AppError {
    #[snafu(display("At {location}: Failed to send notification\n{source}"))]
    Notify {
        source: notify::NotifyError,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("At {location}: Waiting for DriverStation process failed\n{source}"))]
    Wmi {
        source: wmi::WmiError,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}

async fn kbnt() -> Result<(), AppError> {
    notify::active().context(NotifySnafu)?;

    loop {
        wmi::wait_for_ds().await.context(WmiSnafu)?;
        notify::driverstation().context(NotifySnafu)?;
    }
}

#[compio::main]
async fn main() {
    // TODO: error handing
    kbnt().await.unwrap();
}
