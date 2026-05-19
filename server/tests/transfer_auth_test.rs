mod common;

use common::{TestClient, TestServer};
use uuid::Uuid;

#[tokio::test]
async fn test_transfer_sends_cookie_token() {
    let server_a = TestServer::spawn_with_env(&[("RUSTMC_TRANSFER_SECRET", "integration-test-secret")])
        .await
        .expect("Failed to spawn server A");

    let server_b = TestServer::spawn_with_env(&[("RUSTMC_TRANSFER_SECRET", "integration-test-secret")])
        .await
        .expect("Failed to spawn server B");

    let mut client = TestClient::connect(server_a.port())
        .await
        .expect("Failed to connect to server A");

    complete_login_flow(&mut client).await;
    drain_initial_play_packets(&mut client).await;

    let target_host = "127.0.0.1";
    let target_port = server_b.port() as i32;
    client
        .send_chat_command(&format!("transfer {} {}", target_host, target_port))
        .await
        .expect("Failed to send transfer command");

    // Should receive Store Cookie packet (0x74) before Transfer packet (0x73)
    let cookie_packet = client
        .read_packet()
        .await
        .expect("Failed to read store cookie packet");
    assert_eq!(
        cookie_packet.id, 0x74,
        "Expected store cookie packet before transfer"
    );

    let transfer_packet = client
        .read_packet()
        .await
        .expect("Failed to read transfer packet");
    assert_eq!(transfer_packet.id, 0x73, "Expected transfer packet");
}

#[tokio::test]
async fn test_target_server_requests_transfer_cookie() {
    let server = TestServer::spawn_with_env(&[("RUSTMC_TRANSFER_SECRET", "integration-test-secret")])
        .await
        .expect("Failed to spawn server");

    let mut client = TestClient::connect(server.port())
        .await
        .expect("Failed to connect");

    complete_login_flow_and_check_cookie_request(&mut client).await;
}

#[tokio::test]
async fn test_transfer_without_secret_skips_token() {
    let server_a = TestServer::spawn().await.expect("Failed to spawn server A");
    let server_b = TestServer::spawn().await.expect("Failed to spawn server B");

    let mut client = TestClient::connect(server_a.port())
        .await
        .expect("Failed to connect to server A");

    complete_login_flow(&mut client).await;
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
        packet.id, 0x73,
        "Expected transfer packet directly without secret"
    );
}

async fn complete_login_flow(client: &mut TestClient) {
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
        if packet.id == 0x03 {
            break;
        }
    }

    // Send Acknowledge Finish Configuration to transition server to Play state
    client
        .send_acknowledge_finish_configuration()
        .await
        .expect("Failed to send acknowledge finish configuration");

    // Read play login sequence packets until Game Event (0x26)
    loop {
        let packet = client
            .read_packet()
            .await
            .expect("Failed to read play login packet");
        if packet.id == 0x26 {
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
        if packet.id == 0x03 {
            break;
        }
    }

    // Send Acknowledge Finish Configuration to transition server to Play state
    client
        .send_acknowledge_finish_configuration()
        .await
        .expect("Failed to send acknowledge finish configuration");

    // After configuration, server sends play login sequence.
    // With RUSTMC_TRANSFER_SECRET set, it should include a cookie request (0x16).
    let join_game = client
        .read_packet()
        .await
        .expect("Failed to read join game");
    assert_eq!(join_game.id, 0x31, "Expected join game packet");

    // Next should be cookie request (0x16) for "rustmc:transfer_token"
    let cookie_request = client
        .read_packet()
        .await
        .expect("Failed to read cookie request");
    assert_eq!(
        cookie_request.id, 0x16,
        "Expected cookie request packet after join game"
    );
}

async fn drain_initial_play_packets(client: &mut TestClient) {
    loop {
        let packet = client
            .read_packet()
            .await
            .expect("Failed to read play packet");
        if packet.id == 0x0B {
            // Chunk Batch Finished
            break;
        }
    }
}
