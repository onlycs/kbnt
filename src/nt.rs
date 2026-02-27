use std::{
    net::{Ipv4Addr, SocketAddrV4},
    time::Duration,
};

use itertools::Itertools;
use network_tables::v4::{Client, PublishProperties, PublishedTopic, Type};
use rmpv::Utf8String;
use snafu::{ResultExt, Snafu};
use tokio::sync::mpsc::UnboundedReceiver;

use crate::config::KBNTConfig;

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
}

impl NTError {
    pub(crate) fn source(&self) -> &network_tables::Error {
        match self {
            NTError::Connect { source, .. } => source,
            NTError::TopicPublish { source, .. } => source,
            NTError::ValuePublish { source, .. } => source,
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
    async fn connect(team: u16) -> Result<Client, NTError> {
        let client = Client::try_new_w_config(
            SocketAddrV4::new(
                Ipv4Addr::new(10, (team / 100) as u8, (team % 100) as u8, 2),
                5810,
            ),
            Default::default(),
        )
        .await;

        let client = match client {
            Ok(client) => client,
            Err(network_tables::Error::ConnectTimeout(_)) => {
                tokio::time::sleep(Duration::from_secs(5)).await;
                Box::pin(Self::connect(team)).await?
            }
            other @ Err(_) => {
                other.context(ConnectSnafu)?;
                unreachable!()
            }
        };

        Ok(client)
    }

    pub(crate) async fn new(config: &KBNTConfig) -> Result<NT4Connection, NTError> {
        let keys = config.capture_chars.to_lowercase();
        let client = Self::connect(config.team_number).await?;

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
