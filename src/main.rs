#![feature(if_let_guard)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod kb;
mod log;
mod notify;
mod nt;
mod wmi;

use snafu::prelude::*;

use crate::nt::keypress_loop;

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

    #[snafu(display("At {location}: NetworkTables error\n{source}"))]
    NetworkTables {
        source: nt::NTError,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("At {location}: Keyboard hook error\n{source}"))]
    KeyboardHook {
        source: kb::KBError,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("At {location}: Keyboard hook stopped unexpectedly"))]
    KeyboardHookStopped {
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("At {location}: Failed to parse config\n{source}"))]
    ConfigParse {
        source: config::ConfigError,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}

async fn kbnt() -> Result<(), AppError> {
    let cfg = config::parse().await.context(ConfigParseSnafu)?;

    notify::active().context(NotifySnafu)?;
    wmi::wait_for_ds().await.context(WmiSnafu)?;
    notify::driverstation().context(NotifySnafu)?;

    loop {
        let nt4 = nt::NT4Connection::new(&cfg)
            .await
            .context(NetworkTablesSnafu)?;

        notify::connected().context(NotifySnafu)?;

        let rx = kb::listen_keys().context(KeyboardHookSnafu)?;

        match keypress_loop(nt4, rx).await {
            Ok(()) => {
                return Err(KeyboardHookStoppedSnafu.build());
            }
            Err(e)
                if let network_tables::Error::Tungstenite(
                    tokio_tungstenite::tungstenite::Error::ConnectionClosed,
                ) = e.source() =>
            {
                if wmi::query_ds().await.context(WmiSnafu)? {
                    // attempt reconnection immediately if DS is still open
                    notify::disconnected().context(NotifySnafu)?;
                    continue;
                }

                notify::disconnected_ds().context(NotifySnafu)?;
                wmi::wait_for_ds().await.context(WmiSnafu)?;
                notify::driverstation().context(NotifySnafu)?;
            }
            Err(e) => {
                return Err(e).context(NetworkTablesSnafu);
            }
        }
    }
}

#[tokio::main]
async fn main() {
    if let Err(e) = kbnt().await {
        let filename = log::error(e);
        notify::error(filename.display()).ok();
    }
}
