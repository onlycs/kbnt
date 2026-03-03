#![feature(if_let_guard, str_as_str)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod install;
mod kb;
mod log;
mod notify;
mod nt;
mod wmi;

use std::io;

use snafu::prelude::*;
use tokio_tungstenite::tungstenite;
use tracing::{Level, info};
use tracing_subscriber::{filter::Targets, fmt, layer::SubscriberExt, util::SubscriberInitExt};

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
    info!("Installation Complete. Application active.");

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

        let rx = kb::listen_keys().await.context(KeyboardHookSnafu)?;

        match nt::keypress_loop(nt4, rx).await {
            Ok(()) => {
                return Err(KeyboardHookStoppedSnafu.build());
            }
            Err(e) => {
                let Some(nt) = e.nt_source() else {
                    return Err(e).context(NetworkTablesSnafu);
                };

                let ok = match nt {
                    network_tables::Error::Io(io) => [
                        io::ErrorKind::ConnectionReset,
                        io::ErrorKind::ConnectionAborted,
                        io::ErrorKind::ConnectionRefused,
                        io::ErrorKind::NotConnected,
                    ]
                    .contains(&io.kind()),
                    network_tables::Error::ConnectTimeout(_) => true,
                    network_tables::Error::Tungstenite(tungstenite::Error::ConnectionClosed) => {
                        true
                    }
                    _ => false,
                };

                if !ok {
                    return Err(e).context(NetworkTablesSnafu);
                }

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
    tracing_subscriber::registry()
        .with(Targets::new().with_default(Level::DEBUG))
        .with(
            fmt::layer()
                .with_ansi(false)
                .with_writer(move || log::LogWriter),
        )
        .init();

    if let Err(e) = kbnt().await {
        let filename = log::error(e);
        notify::error(filename.display()).ok();
    }
}
