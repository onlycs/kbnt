use std::{
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    time::Duration,
};

use frclib_nt4::client::AsyncClientHandle;

const TEAM_NUMBER: u16 = 2791;

pub(crate) async fn connect() -> AsyncClientHandle {
    let client = AsyncClientHandle::start(
        SocketAddrV4::new(
            Ipv4Addr::new(10, (TEAM_NUMBER / 100) as u8, (TEAM_NUMBER % 100) as u8, 2),
            5810,
        ),
        Default::default(),
        crate::APP_ID.to_string(),
    )
    .await;

    match client {
        Ok(client) => client,
        Err(e) => {
            tokio::time::sleep(Duration::from_secs(5));
            connect().await
        }
    }
}
