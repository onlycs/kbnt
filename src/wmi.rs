use futures_util::StreamExt;
use serde::Deserialize;
use snafu::prelude::*;
use wmi::WMIConnection;

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct Win32Process {
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct Win32ProcessTrace {
    process_name: String,
}

#[derive(Debug, Snafu)]
pub(crate) enum WmiError {
    #[snafu(display("At {location}: Failed to create WMI connection\n{source}"))]
    WmiConnection {
        source: wmi::WMIError,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("At {location}: Failed to query WMI\n{source}"))]
    WmiQuery {
        source: wmi::WMIError,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("At {location}: Events stopped unexpectedly"))]
    WmiEventStream {
        #[snafu(implicit)]
        location: snafu::Location,
    },
}

pub(crate) fn connection() -> Result<WMIConnection, WmiError> {
    WMIConnection::new().context(WmiConnectionSnafu)
}

pub(crate) async fn query_ds(wmi: &WMIConnection) -> Result<bool, WmiError> {
    let results: Vec<Win32Process> = wmi
        .async_raw_query("SELECT Name FROM Win32_Process")
        .await
        .context(WmiQuerySnafu)?;

    for process in results {
        if process.name.to_lowercase() == "driverstation.exe" {
            return Ok(true);
        }
    }

    Ok(false)
}

pub(crate) async fn wait_for_ds(wmi: &WMIConnection) -> Result<(), WmiError> {
    let mut events = wmi
        .exec_notification_query_async("SELECT * FROM Win32_ProcessStartTrace")
        .context(WmiConnectionSnafu)?;

    if query_ds(&wmi).await? {
        return Ok(());
    }

    loop {
        let event = match events.next().await {
            Some(evt) => evt.context(WmiQuerySnafu)?,
            None => return Err(WmiEventStreamSnafu.build()),
        };

        let process: Win32ProcessTrace = event.into_desr().context(WmiQuerySnafu)?;

        if process.process_name.to_lowercase() == "driverstation.exe" {
            break;
        }
    }

    Ok(())
}
