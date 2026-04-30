mod common;

use std::io::{Cursor, Read};
use common::{TestClient, TestServer};
use uuid::Uuid;

#[tokio::test]
async fn test_status_flow() {
    let server = TestServer::spawn().await.expect("Failed to spawn server");
    let mut client = TestClient::connect(server.port()).await.expect("Failed to connect");

    // Send handshake with status intent
    client.send_handshake(765, 1).await.expect("Failed to send handshake");

    // Send status request
    client.send_status_request().await.expect("Failed to send status request");

    // Read status response
    let response = client.read_packet().await.expect("Failed to read status response");
    assert_eq!(response.id, 0x00, "Expected status response packet");

    // Parse JSON response
    let mut cursor = Cursor::new(&response.data);
    let json_str = common::client::read_string(&mut cursor).expect("Failed to read JSON string");
    let json: serde_json::Value = serde_json::from_str(&json_str).expect("Failed to parse JSON");

    // Verify status response contains expected fields
    assert!(json["description"]["text"].as_str().unwrap().contains("RustMC"), "MOTD should contain 'RustMC'");
    assert_eq!(json["version"]["protocol"].as_i64().unwrap(), 765, "Protocol version should be 765");
    assert_eq!(json["players"]["online"].as_i64().unwrap(), 0, "Online players should be 0");

    // Send ping
    let ping_time = std::time::Instant::now();
    let payload = 12345i64;
    client.send_ping(payload).await.expect("Failed to send ping");

    // Read pong
    let pong = client.read_packet().await.expect("Failed to read pong");
    let elapsed = ping_time.elapsed();
    assert_eq!(pong.id, 0x01, "Expected pong packet");
    assert!(elapsed.as_millis() < 100, "Ping should be under 100ms");

    // Verify payload matches
    let pong_payload = i64::from_be_bytes(pong.data.try_into().expect("Invalid pong data"));
    assert_eq!(pong_payload, payload, "Pong payload should match ping payload");
}

#[tokio::test]
async fn test_login_flow() {
    let server = TestServer::spawn().await.expect("Failed to spawn server");
    let mut client = TestClient::connect(server.port()).await.expect("Failed to connect");

    // Send handshake with login intent
    client.send_handshake(765, 2).await.expect("Failed to send handshake");

    // Send login start
    let uuid = Uuid::new_v4();
    let username = "TestPlayer";
    client.send_login_start(username, uuid).await.expect("Failed to send login start");

    // Read login success
    let login_success = client.read_packet().await.expect("Failed to read login success");
    assert_eq!(login_success.id, 0x02, "Expected login success packet");

    // Parse login success
    let mut cursor = Cursor::new(&login_success.data);
    let mut uuid_bytes = [0u8; 16];
    cursor.read_exact(&mut uuid_bytes).expect("Failed to read UUID");
    let returned_uuid = Uuid::from_bytes(uuid_bytes);
    assert_eq!(returned_uuid, uuid, "UUID should match");

    let returned_username = common::client::read_string(&mut cursor).expect("Failed to read username");
    assert_eq!(returned_username, username, "Username should match");

    // Send login acknowledged
    client.send_login_acknowledged().await.expect("Failed to send login acknowledged");

    // Read join game packet
    let join_game = client.read_packet().await.expect("Failed to read join game");
    assert_eq!(join_game.id, 0x2B, "Expected join game packet (0x2B)");
    assert!(!join_game.data.is_empty(), "Join game should have data");

    // Read synchronize player position
    let sync_pos = client.read_packet().await.expect("Failed to read sync position");
    assert_eq!(sync_pos.id, 0x3E, "Expected synchronize player position packet");
}

#[tokio::test]
async fn test_play_basic() {
    let server = TestServer::spawn().await.expect("Failed to spawn server");
    let mut client = TestClient::connect(server.port()).await.expect("Failed to connect");

    // Complete login flow
    client.send_handshake(765, 2).await.expect("Failed to send handshake");
    let uuid = Uuid::new_v4();
    client.send_login_start("TestPlayer", uuid).await.expect("Failed to send login start");

    let _login_success = client.read_packet().await.expect("Failed to read login success");
    client.send_login_acknowledged().await.expect("Failed to send login acknowledged");
    let _join_game = client.read_packet().await.expect("Failed to read join game");
    let _sync_pos = client.read_packet().await.expect("Failed to read sync position");

    // Send player position
    client.send_player_position(100.0, 64.0, 200.0, true)
        .await
        .expect("Failed to send position");

    // The server should handle this without errors
    // We can't verify a specific response since the server may not echo positions,
    // but connection should remain alive
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
}

#[tokio::test]
async fn test_full_protocol_sequence() {
    let server = TestServer::spawn().await.expect("Failed to spawn server");

    // Client A: Status flow then disconnect
    {
        let mut client_a = TestClient::connect(server.port()).await.expect("Failed to connect A");
        client_a.send_handshake(765, 1).await.expect("Failed to send handshake A");
        client_a.send_status_request().await.expect("Failed to send status A");
        let _status = client_a.read_packet().await.expect("Failed to read status A");
        // Client A disconnects (drops)
    }

    // Client B: Login flow then disconnect
    {
        let mut client_b = TestClient::connect(server.port()).await.expect("Failed to connect B");
        client_b.send_handshake(765, 2).await.expect("Failed to send handshake B");
        let uuid_b = Uuid::new_v4();
        client_b.send_login_start("PlayerB", uuid_b).await.expect("Failed to send login B");
        let _login = client_b.read_packet().await.expect("Failed to read login B");
        client_b.send_login_acknowledged().await.expect("Failed to send ack B");
        let _join = client_b.read_packet().await.expect("Failed to read join B");
        let _sync = client_b.read_packet().await.expect("Failed to read sync B");
        // Client B disconnects (drops)
    }

    // Client C: Login and stay connected
    {
        let mut client_c = TestClient::connect(server.port()).await.expect("Failed to connect C");
        client_c.send_handshake(765, 2).await.expect("Failed to send handshake C");
        let uuid_c = Uuid::new_v4();
        client_c.send_login_start("PlayerC", uuid_c).await.expect("Failed to send login C");
        let _login = client_c.read_packet().await.expect("Failed to read login C");
        client_c.send_login_acknowledged().await.expect("Failed to send ack C");
        let _join = client_c.read_packet().await.expect("Failed to read join C");
        let _sync = client_c.read_packet().await.expect("Failed to read sync C");

        // Stay connected for a bit
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        // Client C disconnects (drops)
    }

    // All sequences completed successfully
}

#[tokio::test]
async fn test_concurrent_clients() {
    let server = TestServer::spawn().await.expect("Failed to spawn server");

    let mut handles = Vec::new();

    for i in 0..10 {
        let port = server.port();
        let handle = tokio::spawn(async move {
            let mut client = TestClient::connect(port).await.expect("Failed to connect");
            client.send_handshake(765, 2).await.expect("Failed to send handshake");

            let uuid = Uuid::new_v4();
            let username = format!("Player{i}");
            client.send_login_start(&username, uuid).await.expect("Failed to send login");

            let login_success = client.read_packet().await.expect("Failed to read login success");
            assert_eq!(login_success.id, 0x02, "Expected login success");

            client.send_login_acknowledged().await.expect("Failed to send ack");
            let _join = client.read_packet().await.expect("Failed to read join");
            let _sync = client.read_packet().await.expect("Failed to read sync");
        });
        handles.push(handle);
    }

    // Wait for all clients to complete
    for handle in handles {
        handle.await.expect("Client task panicked");
    }

    // All 10 clients successfully logged in
}

#[tokio::test]
async fn test_error_handling() {
    let server = TestServer::spawn().await.expect("Failed to spawn server");

    // Test 1: Send login start before handshake (should disconnect)
    {
        let mut client = TestClient::connect(server.port()).await.expect("Failed to connect");
        let uuid = Uuid::new_v4();

        // Skip handshake and send login start directly
        let result = client.send_login_start("BadPlayer", uuid).await;

        // Server should reject this - either the send fails or reading fails
        if result.is_ok() {
            // Try to read response - should fail or disconnect
            let read_result = tokio::time::timeout(
                tokio::time::Duration::from_secs(1),
                client.read_packet()
            ).await;

            // Either timeout or error - both are acceptable
            assert!(read_result.is_err() || read_result.unwrap().is_err(),
                "Server should disconnect on invalid protocol sequence");
        }
    }

    // Test 2: Invalid handshake next_state value
    {
        let mut client = TestClient::connect(server.port()).await.expect("Failed to connect");

        // Send handshake with invalid next_state (3 is invalid, should be 1 or 2)
        let result = client.send_handshake(765, 99).await;

        if result.is_ok() {
            // Try to continue - should fail
            let read_result = tokio::time::timeout(
                tokio::time::Duration::from_secs(1),
                client.read_packet()
            ).await;

            // Connection should fail or timeout
            assert!(read_result.is_err() || read_result.unwrap().is_err(),
                "Server should reject invalid handshake");
        }
    }
}
