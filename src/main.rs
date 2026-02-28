#![feature(if_let_guard, exit_status_error)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod install;
mod kb;
mod log;
mod notify;
mod nt;
mod wmi;

use snafu::prelude::*;
use tokio_tungstenite::tungstenite;

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

    #[snafu(display("At {location}: Failed to get config\n{source}"))]
    Config {
        source: install::InstallError,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("At {location}: Failed to install KBNT\n{source}"))]
    Install {
        source: install::InstallError,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}

async fn kbnt() -> Result<(), AppError> {
    install::install().context(InstallSnafu)?;

    let wmi = wmi::connection().context(WmiSnafu)?;
    let cfg = install::config().context(ConfigSnafu)?;

    notify::active().context(NotifySnafu)?;
    wmi::wait_for_ds(&wmi).await.context(WmiSnafu)?;
    notify::driverstation().context(NotifySnafu)?;

    loop {
        let nt4 = match nt::NT4Connection::new(&cfg, &wmi).await {
            Ok(nt4) => nt4,
            Err(nt::NTError::DsClosed { .. }) => {
                notify::disconnected_ds().context(NotifySnafu)?;
                wmi::wait_for_ds(&wmi).await.context(WmiSnafu)?;
                notify::driverstation().context(NotifySnafu)?;
                continue;
            }
            Err(e) => return Err(e).context(NetworkTablesSnafu),
        };

        notify::connected().context(NotifySnafu)?;

        let rx = kb::listen_keys().context(KeyboardHookSnafu)?;

        match nt::keypress_loop(nt4, rx).await {
            Ok(()) => {
                return Err(KeyboardHookStoppedSnafu.build());
            }
            Err(e) => {
                let Some(network_tables::Error::Tungstenite(tungstenite::Error::ConnectionClosed)) =
                    e.nt_source()
                else {
                    return Err(e).context(NetworkTablesSnafu);
                };

                if wmi::query_ds(&wmi).await.context(WmiSnafu)? {
                    // attempt reconnection immediately if DS is still open
                    notify::disconnected().context(NotifySnafu)?;
                    continue;
                }

                notify::disconnected_ds().context(NotifySnafu)?;
                wmi::wait_for_ds(&wmi).await.context(WmiSnafu)?;
                notify::driverstation().context(NotifySnafu)?;
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
