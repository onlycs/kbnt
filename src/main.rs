mod notify;
mod nt;
mod wmi;

use std::time::Duration;

use cuid2::cuid;
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

        let nt4 = nt::connect().await;
        notify::connected().context(NotifySnafu)?;
    }
}

#[tokio::main]
async fn main() {
    // TODO: error handing
    kbnt().await.unwrap();
}
