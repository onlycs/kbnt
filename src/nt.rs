use std::{net::SocketAddr, time::Duration};

use itertools::Itertools;
use network_tables::v4::{Client, PublishProperties, PublishedTopic, Type};
use rmpv::Utf8String;
use snafu::{ResultExt, Snafu};
use tokio::sync::mpsc::UnboundedReceiver;
use wmi::WMIConnection;

use crate::install::KBNTConfigHandle;

#[derive(Debug, Snafu)]
pub(crate) enum NTError {
    #[snafu(display("At {location}: Failed to connect to NT4\n{source}"))]
    Connect {
        source: network_tables::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("At {location}: Failed to publish topic\n{source}"))]
    TopicPublish {
        source: network_tables::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("At {location}: Failed to publish value\n{source}"))]
    ValuePublish {
        source: network_tables::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("At {location}: WMI error\n{source}"))]
    Wmi {
        source: crate::wmi::WmiError,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("At {location}: DriverStation closed while waiting for connection"))]
    DsClosed {
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display(
        "At {location}: Failed parsing NT4 IP address\n{source}\nFailed parsing {ip_str}:5810"
    ))]
    IpParse {
        source: std::net::AddrParseError,
        #[snafu(implicit)]
        location: snafu::Location,
        ip_str: String,
    },

    #[snafu(display("At {location}: Failed to read config\n{source}"))]
    Config {
        source: crate::install::InstallError,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}

impl NTError {
    pub(crate) fn nt_source(&self) -> Option<&network_tables::Error> {
        match self {
            NTError::Connect { source, .. } => Some(source),
            NTError::TopicPublish { source, .. } => Some(source),
            NTError::ValuePublish { source, .. } => Some(source),
            NTError::Wmi { .. } => None,
            NTError::DsClosed { .. } => None,
            NTError::IpParse { .. } => None,
            NTError::Config { .. } => None,
        }
    }
}

#[derive(Debug)]
pub(crate) struct NT4Connection {
    client: Client,
    topic: PublishedTopic,
    keys: String,
    presses: Vec<i32>,
}

impl NT4Connection {
    async fn connect(config: &KBNTConfigHandle, wmi: &WMIConnection) -> Result<Client, NTError> {
        let ipv4 = config.robot_ip().context(ConfigSnafu)?;
        let addr = format!("{ipv4}:5810")
            .parse::<SocketAddr>()
            .context(IpParseSnafu { ip_str: ipv4 })?;

        let mut client = Client::try_new_w_config(addr, Default::default()).await;

        while let Err(network_tables::Error::ConnectTimeout(_)) = client {
            tokio::time::sleep(Duration::from_secs(5)).await;

            if !crate::wmi::query_ds(wmi).await.context(WmiSnafu)? {
                return Err(DsClosedSnafu.build());
            }

            client = Client::try_new_w_config(addr, Default::default()).await;
        }

        let client = client.context(ConnectSnafu)?;

        Ok(client)
    }

    pub(crate) async fn new(
        config: &KBNTConfigHandle,
        wmi: &WMIConnection,
    ) -> Result<NT4Connection, NTError> {
        let client = Self::connect(&config, wmi).await?;
        let keys = config.capture_chars().context(ConfigSnafu)?.to_lowercase();

        let k2p = client
            .publish_topic(
                "KBNT/KeysToPress",
                Type::String,
                Some(PublishProperties {
                    retained: Some(true),
                    ..Default::default()
                }),
            )
            .await
            .context(TopicPublishSnafu)?;

        client
            .publish_value(&k2p, &rmpv::Value::String(Utf8String::from(keys.clone())))
            .await
            .context(ValuePublishSnafu)?;

        let topic = client
            .publish_topic("KBNT/NumKeydowns", Type::IntArray, None)
            .await
            .context(TopicPublishSnafu)?;

        client
            .publish_value(&topic, &rmpv::Value::Array(vec![]))
            .await
            .context(ValuePublishSnafu)?;

        Ok(NT4Connection {
            client,
            presses: vec![0; keys.len()],
            topic,
            keys,
        })
    }

    pub(crate) async fn keydown(&mut self, key: char) -> Result<(), NTError> {
        let Some(i) = self.keys.find(key.to_string().to_lowercase().as_str()) else {
            return Ok(());
        };

        self.presses[i] += 1;

        self.client
            .publish_value(
                &self.topic,
                &rmpv::Value::Array(
                    self.presses
                        .iter()
                        .map(|&press| rmpv::Value::from(press))
                        .collect_vec(),
                ),
            )
            .await
            .context(ValuePublishSnafu)?;

        Ok(())
    }
}

pub(crate) async fn keypress_loop(
    mut nt4: NT4Connection,
    mut rx: UnboundedReceiver<char>,
) -> Result<(), NTError> {
    while let Some(key) = rx.recv().await {
        nt4.keydown(key).await?;
    }

    Ok(())
}
