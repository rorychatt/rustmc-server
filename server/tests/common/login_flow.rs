use crate::common::TestClient;
use rustmc_server::protocol::packet_ids;
use uuid::Uuid;

use packet_ids::configuration::clientbound as config_cb;
use packet_ids::play::clientbound as play_cb;

#[allow(dead_code)]
pub async fn try_complete_login_flow_with_uuid(
    client: &mut TestClient,
    username: &str,
    uuid: Uuid,
) -> anyhow::Result<()> {
    client.send_handshake(775, 2).await?;
    client.send_login_start(username, uuid).await?;

    let _compression = client.read_packet().await?;
    client.enable_compression(256);

    let _login_success = client.read_packet().await?;

    client.send_login_acknowledged().await?;

    let _known_packs = client.read_packet().await?;

    client.send_known_packs_response().await?;

    loop {
        let packet = client.read_packet().await?;
        if packet.id == config_cb::FINISH_CONFIGURATION {
            break;
        }
    }

    client.send_acknowledge_finish_configuration().await?;

    loop {
        let packet = client.read_packet().await?;
        if packet.id == play_cb::GAME_EVENT {
            break;
        }
    }

    Ok(())
}

#[allow(dead_code)]
pub async fn try_complete_login_flow(
    client: &mut TestClient,
    username: &str,
) -> anyhow::Result<()> {
    try_complete_login_flow_with_uuid(client, username, Uuid::new_v4()).await
}

#[allow(dead_code)]
pub async fn try_drain_initial_play_packets(client: &mut TestClient) -> anyhow::Result<()> {
    loop {
        let packet = client.read_packet().await?;
        if packet.id == play_cb::CHUNK_BATCH_FINISHED {
            break;
        }
    }
    Ok(())
}
