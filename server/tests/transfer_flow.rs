mod common;

use common::{retry_test, TestClient, TestServer};
use rustmc_server::protocol::packet_ids;
use uuid::Uuid;

use packet_ids::configuration::clientbound as config_cb;
use packet_ids::play::clientbound as play_cb;

const OP_UUID: &str = "069a79f4-44e9-4726-a5be-fca90e38aaf5";

fn ops_config(uuid: &str) -> String {
    format!("[[operators]]\nuuid = \"{uuid}\"\nname = \"TestOp\"\nlevel = 4\n")
}

#[tokio::test]
async fn test_transfer_packet_sent() {
    retry_test("test_transfer_packet_sent", 3, || async {
        let op_uuid = Uuid::parse_str(OP_UUID).unwrap();
        let ops = ops_config(OP_UUID);

        let server_a = TestServer::spawn_with_ops(Some(&ops)).await?;
        let server_b = TestServer::spawn().await?;

        let mut client = TestClient::connect(server_a.port()).await?;

        try_complete_login_flow_with_uuid(&mut client, op_uuid).await?;
        try_drain_initial_play_packets(&mut client).await?;

        let target_host = "127.0.0.1";
        let target_port = server_b.port() as i32;
        client
            .send_chat_command(&format!("transfer {} {}", target_host, target_port))
            .await?;

        let packet = client.read_packet().await?;
        assert_eq!(packet.id, play_cb::TRANSFER, "Expected transfer packet");

        let (host, port) = packet.read_transfer().unwrap();
        assert_eq!(host, target_host);
        assert_eq!(port, target_port);

        let mut client_b = TestClient::connect(server_b.port()).await?;
        try_complete_login_flow(&mut client_b).await?;

        Ok(())
    })
    .await;
}

#[tokio::test]
async fn test_transfer_denied_without_permission() {
    retry_test("test_transfer_denied_without_permission", 3, || async {
        let server = TestServer::spawn().await?;

        let mut client = TestClient::connect(server.port()).await?;

        try_complete_login_flow(&mut client).await?;
        try_drain_initial_play_packets(&mut client).await?;

        client.send_chat_command("transfer localhost 25565").await?;

        let packet = client.read_packet().await?;
        assert_eq!(
            packet.id,
            play_cb::SYSTEM_CHAT_MESSAGE,
            "Expected system chat message packet"
        );

        let json = packet.read_system_chat().unwrap();
        assert!(
            json.contains("permission"),
            "Expected permission denial message, got: {json}"
        );

        Ok(())
    })
    .await;
}

#[tokio::test]
async fn test_transfer_invalid_command() {
    retry_test("test_transfer_invalid_command", 3, || async {
        let op_uuid = Uuid::parse_str(OP_UUID).unwrap();
        let ops = ops_config(OP_UUID);

        let server = TestServer::spawn_with_ops(Some(&ops)).await?;

        let mut client = TestClient::connect(server.port()).await?;

        try_complete_login_flow_with_uuid(&mut client, op_uuid).await?;
        try_drain_initial_play_packets(&mut client).await?;

        client.send_chat_command("transfer localhost").await?;

        client.send_player_position(0.0, 64.0, 0.0, true).await?;

        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        client.send_player_position(1.0, 64.0, 1.0, true).await?;

        Ok(())
    })
    .await;
}

async fn try_complete_login_flow(client: &mut TestClient) -> anyhow::Result<()> {
    try_complete_login_flow_with_uuid(client, Uuid::new_v4()).await
}

async fn try_complete_login_flow_with_uuid(
    client: &mut TestClient,
    uuid: Uuid,
) -> anyhow::Result<()> {
    client.send_handshake(775, 2).await?;
    client.send_login_start("TransferTest", uuid).await?;

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

    let _join_game = client.read_packet().await?;

    let _player_info = client.read_packet().await?;

    let _sync_pos = client.read_packet().await?;

    Ok(())
}

async fn try_drain_initial_play_packets(client: &mut TestClient) -> anyhow::Result<()> {
    let game_event = client.read_packet().await?;
    assert_eq!(
        game_event.id,
        play_cb::GAME_EVENT,
        "Expected game event packet"
    );

    let center_chunk = client.read_packet().await?;
    assert_eq!(
        center_chunk.id,
        play_cb::SET_CENTER_CHUNK,
        "Expected set center chunk packet"
    );

    let batch_start = client.read_packet().await?;
    assert_eq!(
        batch_start.id,
        play_cb::CHUNK_BATCH_START,
        "Expected chunk batch start"
    );

    loop {
        let packet = client.read_packet().await?;
        if packet.id == play_cb::CHUNK_BATCH_FINISHED {
            break;
        }
    }

    Ok(())
}
