mod common;

use anyhow::Result;
use common::{TestClient, TestServer};
use tokio::time::{sleep, Duration};
use uuid::Uuid;

const OP_UUID: &str = "069a79f4-44e9-4726-a5be-fca90e38aaf5";

fn ops_config(uuid: &str) -> String {
    format!("[[operators]]\nuuid = \"{uuid}\"\nname = \"TestOp\"\nlevel = 4\n")
}

#[tokio::test]
async fn test_transfer_packet_sent() {
    let op_uuid = Uuid::parse_str(OP_UUID).unwrap();
    let ops = ops_config(OP_UUID);

    let server_a = TestServer::spawn_with_ops(Some(&ops))
        .await
        .expect("Failed to spawn server A");
    let server_b = TestServer::spawn().await.expect("Failed to spawn server B");

    let mut client = connect_with_retry(server_a.port(), op_uuid).await;
    drain_initial_play_packets(&mut client).await;

    let target_host = "127.0.0.1";
    let target_port = server_b.port() as i32;
    client
        .send_chat_command(&format!("transfer {} {}", target_host, target_port))
        .await
        .expect("Failed to send transfer command");

    let packet = client
        .read_packet()
        .await
        .expect("Failed to read transfer packet");
    assert_eq!(packet.id, 0x73, "Expected transfer packet");

    let (host, port) = packet.read_transfer().unwrap();
    assert_eq!(host, target_host);
    assert_eq!(port, target_port);

    let _client_b = connect_with_retry(server_b.port(), Uuid::new_v4()).await;
}

#[tokio::test]
async fn test_transfer_denied_without_permission() {
    let server = TestServer::spawn().await.expect("Failed to spawn server");

    let mut client = connect_with_retry(server.port(), Uuid::new_v4()).await;
    drain_initial_play_packets(&mut client).await;

    client
        .send_chat_command("transfer localhost 25565")
        .await
        .expect("Failed to send transfer command");

    let packet = client
        .read_packet()
        .await
        .expect("Failed to read response packet");
    // System Chat Message packet (0x79)
    assert_eq!(packet.id, 0x79, "Expected system chat message packet");

    let json = packet.read_system_chat().unwrap();
    assert!(
        json.contains("permission"),
        "Expected permission denial message, got: {json}"
    );
}

#[tokio::test]
async fn test_transfer_invalid_command() {
    let op_uuid = Uuid::parse_str(OP_UUID).unwrap();
    let ops = ops_config(OP_UUID);

    let server = TestServer::spawn_with_ops(Some(&ops))
        .await
        .expect("Failed to spawn server");

    let mut client = connect_with_retry(server.port(), op_uuid).await;
    drain_initial_play_packets(&mut client).await;

    // Send a malformed transfer command (missing port)
    client
        .send_chat_command("transfer localhost")
        .await
        .expect("Failed to send command");

    // Connection should stay alive - send a position and confirm no crash
    client
        .send_player_position(0.0, 64.0, 0.0, true)
        .await
        .expect("Failed to send position");

    sleep(Duration::from_millis(200)).await;

    client
        .send_player_position(1.0, 64.0, 1.0, true)
        .await
        .expect("Connection should still be alive");
}

async fn connect_with_retry(port: u16, uuid: Uuid) -> TestClient {
    let mut attempts = 0;
    loop {
        attempts += 1;
        match try_connect_and_login(port, uuid).await {
            Ok(client) => return client,
            Err(e) if attempts < 3 => {
                eprintln!("Connection attempt {attempts} failed: {e}, retrying...");
                sleep(Duration::from_millis(100)).await;
            }
            Err(e) => panic!("Failed after {attempts} attempts: {e}"),
        }
    }
}

async fn try_connect_and_login(port: u16, uuid: Uuid) -> Result<TestClient> {
    let mut client = TestClient::connect(port).await?;
    try_complete_login_flow_with_uuid(&mut client, uuid).await?;
    Ok(client)
}

async fn try_complete_login_flow_with_uuid(client: &mut TestClient, uuid: Uuid) -> Result<()> {
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
        if packet.id == 0x03 {
            break;
        }
    }

    client.send_acknowledge_finish_configuration().await?;

    // Read join game (0x31)
    let _join_game = client.read_packet().await?;

    // Read player info update (0x40)
    let _player_info = client.read_packet().await?;

    // Read sync position (0x48)
    let _sync_pos = client.read_packet().await?;

    Ok(())
}

async fn drain_initial_play_packets(client: &mut TestClient) {
    // Read Game Event (0x26)
    let game_event = client
        .read_packet()
        .await
        .expect("Failed to read game event");
    assert_eq!(game_event.id, 0x26, "Expected game event packet");

    // Read Set Center Chunk (0x58)
    let center_chunk = client
        .read_packet()
        .await
        .expect("Failed to read set center chunk");
    assert_eq!(center_chunk.id, 0x58, "Expected set center chunk packet");

    // Read Chunk Batch Start (0x0C)
    let batch_start = client
        .read_packet()
        .await
        .expect("Failed to read chunk batch start");
    assert_eq!(batch_start.id, 0x0C, "Expected chunk batch start");

    // Read chunk data packets until Chunk Batch Finished (0x0B)
    loop {
        let packet = client
            .read_packet()
            .await
            .expect("Failed to read chunk packet");
        if packet.id == 0x0B {
            break;
        }
    }
}
