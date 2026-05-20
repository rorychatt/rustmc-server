mod common;

use common::{TestClient, TestServer};
use rustmc_server::protocol::packet_ids;
use uuid::Uuid;

use packet_ids::configuration::clientbound as config_cb;
use packet_ids::play::clientbound as play_cb;

const OP_UUID: &str = "069a79f4-44e9-4726-a5be-fca90e38aaf5";

fn ops_config(uuid: &str) -> String {
    format!("[[operators]]\nuuid = \"{uuid}\"\nname = \"TransferAuthTest\"\nlevel = 4\n")
}

#[tokio::test]
async fn test_transfer_sends_cookie_token() {
    let op_uuid = Uuid::parse_str(OP_UUID).unwrap();
    let ops = ops_config(OP_UUID);

    let server_a = TestServer::spawn_with_env_and_ops_content(
        &[("RUSTMC_TRANSFER_SECRET", "integration-test-secret")],
        Some(&ops),
    )
    .await
    .expect("Failed to spawn server A");

    let server_b =
        TestServer::spawn_with_env(&[("RUSTMC_TRANSFER_SECRET", "integration-test-secret")])
            .await
            .expect("Failed to spawn server B");

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

    // Should receive Store Cookie packet before Transfer packet
    let cookie_packet = client
        .read_packet()
        .await
        .expect("Failed to read store cookie packet");
    assert_eq!(
        cookie_packet.id, play_cb::STORE_COOKIE,
        "Expected store cookie packet before transfer"
    );

    let transfer_packet = client
        .read_packet()
        .await
        .expect("Failed to read transfer packet");
    assert_eq!(transfer_packet.id, play_cb::TRANSFER, "Expected transfer packet");
}

#[tokio::test]
async fn test_target_server_requests_transfer_cookie() {
    let server =
        TestServer::spawn_with_env(&[("RUSTMC_TRANSFER_SECRET", "integration-test-secret")])
            .await
            .expect("Failed to spawn server");

    let mut client = TestClient::connect(server.port())
        .await
        .expect("Failed to connect");

    complete_login_flow_and_check_cookie_request(&mut client).await;
}

#[tokio::test]
async fn test_transfer_without_secret_skips_token() {
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

    // Without secret, should get transfer packet directly (no cookie)
    let packet = client
        .read_packet()
        .await
        .expect("Failed to read transfer packet");
    assert_eq!(
        packet.id, play_cb::TRANSFER,
        "Expected transfer packet directly without secret"
    );
}

async fn complete_login_flow_with_uuid(client: &mut TestClient, uuid: Uuid) {
    client
        .send_handshake(775, 2)
        .await
        .expect("Failed to send handshake");
    client
        .send_login_start("TransferAuthTest", uuid)
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
        if packet.id == config_cb::FINISH_CONFIGURATION {
            break;
        }
    }

    // Send Acknowledge Finish Configuration to transition server to Play state
    client
        .send_acknowledge_finish_configuration()
        .await
        .expect("Failed to send acknowledge finish configuration");

    // Read play login sequence packets until Game Event
    loop {
        let packet = client
            .read_packet()
            .await
            .expect("Failed to read play login packet");
        if packet.id == play_cb::GAME_EVENT {
            break;
        }
    }
}

async fn complete_login_flow_and_check_cookie_request(client: &mut TestClient) {
    client
        .send_handshake(775, 2)
        .await
        .expect("Failed to send handshake");
    let uuid = Uuid::new_v4();
    client
        .send_login_start("TransferAuthTest", uuid)
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
        if packet.id == config_cb::FINISH_CONFIGURATION {
            break;
        }
    }

    // Send Acknowledge Finish Configuration to transition server to Play state
    client
        .send_acknowledge_finish_configuration()
        .await
        .expect("Failed to send acknowledge finish configuration");

    // After configuration, server sends play login sequence.
    // With RUSTMC_TRANSFER_SECRET set, it should include a cookie request.
    let join_game = client
        .read_packet()
        .await
        .expect("Failed to read join game");
    assert_eq!(join_game.id, play_cb::LOGIN_PLAY, "Expected join game packet");

    // Next should be cookie request for "rustmc:transfer_token"
    let cookie_request = client
        .read_packet()
        .await
        .expect("Failed to read cookie request");
    assert_eq!(
        cookie_request.id, play_cb::COOKIE_REQUEST,
        "Expected cookie request packet after join game"
    );
}

async fn drain_initial_play_packets(client: &mut TestClient) {
    loop {
        let packet = client
            .read_packet()
            .await
            .expect("Failed to read play packet");
        if packet.id == play_cb::CHUNK_BATCH_FINISHED {
            break;
        }
    }
}
