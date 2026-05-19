mod common;

use common::{TestClient, TestServer};
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

    let mut client = TestClient::connect(server_a.port())
        .await
        .expect("Failed to connect to server A");

    complete_login_flow_with_uuid(&mut client, op_uuid).await;
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

    let mut client_b = TestClient::connect(server_b.port())
        .await
        .expect("Failed to connect to server B");
    complete_login_flow(&mut client_b).await;
}

#[tokio::test]
async fn test_transfer_denied_without_permission() {
    let server = TestServer::spawn().await.expect("Failed to spawn server");

    let mut client = TestClient::connect(server.port())
        .await
        .expect("Failed to connect");

    complete_login_flow(&mut client).await;
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

    let mut client = TestClient::connect(server.port())
        .await
        .expect("Failed to connect");

    complete_login_flow_with_uuid(&mut client, op_uuid).await;
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

    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    client
        .send_player_position(1.0, 64.0, 1.0, true)
        .await
        .expect("Connection should still be alive");
}

async fn complete_login_flow(client: &mut TestClient) {
    complete_login_flow_with_uuid(client, Uuid::new_v4()).await;
}

async fn complete_login_flow_with_uuid(client: &mut TestClient, uuid: Uuid) {
    client
        .send_handshake(775, 2)
        .await
        .expect("Failed to send handshake");
    client
        .send_login_start("TransferTest", uuid)
        .await
        .expect("Failed to send login start");

    // Compression
    let _compression = client
        .read_packet()
        .await
        .expect("Failed to read compression");
    client.enable_compression(256);

    // Login Success
    let _login_success = client
        .read_packet()
        .await
        .expect("Failed to read login success");

    // Login Acknowledged
    client
        .send_login_acknowledged()
        .await
        .expect("Failed to send login acknowledged");

    // Known Packs
    let _known_packs = client
        .read_packet()
        .await
        .expect("Failed to read known packs");

    // Send Known Packs response
    client
        .send_known_packs_response()
        .await
        .expect("Failed to send known packs response");

    // Read configuration packets until Finish Configuration
    loop {
        let packet = client
            .read_packet()
            .await
            .expect("Failed to read config packet");
        if packet.id == 0x03 {
            break;
        }
    }

    // Send Acknowledge Finish Configuration to transition server to Play state
    client
        .send_acknowledge_finish_configuration()
        .await
        .expect("Failed to send acknowledge finish configuration");

    // Read join game (0x31)
    let _join_game = client
        .read_packet()
        .await
        .expect("Failed to read join game");

    // Read player info update (0x40)
    let _player_info = client
        .read_packet()
        .await
        .expect("Failed to read player info update");

    // Read sync position (0x48)
    let _sync_pos = client
        .read_packet()
        .await
        .expect("Failed to read sync position");
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
