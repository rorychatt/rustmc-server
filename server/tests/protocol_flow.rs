mod common;

use common::{retry_test, TestClient, TestServer};
use std::io::{Cursor, Read};
use uuid::Uuid;

#[tokio::test]
async fn test_status_flow() {
    let server = TestServer::spawn().await.expect("Failed to spawn server");
    let mut client = TestClient::connect(server.port())
        .await
        .expect("Failed to connect");

    // Send handshake with status intent
    client
        .send_handshake(775, 1)
        .await
        .expect("Failed to send handshake");

    // Send status request
    client
        .send_status_request()
        .await
        .expect("Failed to send status request");

    // Read status response
    let response = client
        .read_packet()
        .await
        .expect("Failed to read status response");
    assert_eq!(response.id, 0x00, "Expected status response packet");

    // Parse JSON response
    let mut cursor = Cursor::new(&response.data);
    let json_str = common::client::read_string(&mut cursor).expect("Failed to read JSON string");
    let json: serde_json::Value = serde_json::from_str(&json_str).expect("Failed to parse JSON");

    // Verify status response contains expected fields
    assert!(
        json["description"]["text"]
            .as_str()
            .unwrap()
            .contains("RustMC"),
        "MOTD should contain 'RustMC'"
    );
    assert_eq!(
        json["version"]["protocol"].as_i64().unwrap(),
        775,
        "Protocol version should be 775"
    );
    assert_eq!(
        json["players"]["online"].as_i64().unwrap(),
        0,
        "Online players should be 0"
    );

    // Send ping
    let ping_time = std::time::Instant::now();
    let payload = 12345i64;
    client
        .send_ping(payload)
        .await
        .expect("Failed to send ping");

    // Read pong
    let pong = client.read_packet().await.expect("Failed to read pong");
    let elapsed = ping_time.elapsed();
    assert_eq!(pong.id, 0x01, "Expected pong packet");
    assert!(elapsed.as_millis() < 100, "Ping should be under 100ms");

    // Verify payload matches
    let pong_payload = i64::from_be_bytes(pong.data.try_into().expect("Invalid pong data"));
    assert_eq!(
        pong_payload, payload,
        "Pong payload should match ping payload"
    );
}

#[tokio::test]
async fn test_login_flow() {
    let server = TestServer::spawn().await.expect("Failed to spawn server");
    let mut client = TestClient::connect(server.port())
        .await
        .expect("Failed to connect");

    // Send handshake with login intent
    client
        .send_handshake(775, 2)
        .await
        .expect("Failed to send handshake");

    // Send login start
    let uuid = Uuid::new_v4();
    let username = "TestPlayer";
    client
        .send_login_start(username, uuid)
        .await
        .expect("Failed to send login start");

    // Read set compression packet (0x03)
    let compression = client
        .read_packet()
        .await
        .expect("Failed to read compression packet");
    assert_eq!(compression.id, 0x03, "Expected set compression packet");

    // Enable compression on client side
    client.enable_compression(256);

    // Read login success
    let login_success = client
        .read_packet()
        .await
        .expect("Failed to read login success");
    assert_eq!(login_success.id, 0x02, "Expected login success packet");

    // Parse login success
    let mut cursor = Cursor::new(&login_success.data);
    let mut uuid_bytes = [0u8; 16];
    cursor
        .read_exact(&mut uuid_bytes)
        .expect("Failed to read UUID");
    let returned_uuid = Uuid::from_bytes(uuid_bytes);
    assert_eq!(returned_uuid, uuid, "UUID should match");

    let returned_username =
        common::client::read_string(&mut cursor).expect("Failed to read username");
    assert_eq!(returned_username, username, "Username should match");

    // Send login acknowledged to enter Configuration phase
    client
        .send_login_acknowledged()
        .await
        .expect("Failed to send login acknowledged");

    // Read Known Packs packet (0x0E)
    let known_packs = client
        .read_packet()
        .await
        .expect("Failed to read known packs");
    assert_eq!(known_packs.id, 0x0E, "Expected known packs packet");

    // Send Known Packs response
    client
        .send_known_packs_response()
        .await
        .expect("Failed to send known packs response");

    // Read configuration packets: registry data, tags, finish
    let got_finish;
    loop {
        let packet = client
            .read_packet()
            .await
            .expect("Failed to read config packet");
        match packet.id {
            0x07 => {
                // Registry Data
            }
            0x0D => {
                // Update Tags
            }
            0x03 => {
                // Finish Configuration
                got_finish = true;
                break;
            }
            _ => {
                panic!("Unexpected config packet: {:#04x}", packet.id);
            }
        }
    }
    assert!(got_finish, "Should receive Finish Configuration");

    // Send Acknowledge Finish Configuration to transition to Play
    client
        .send_acknowledge_finish_configuration()
        .await
        .expect("Failed to send acknowledge finish configuration");

    // Read join game packet (now 0x30 in protocol 775)

    let join_game = client
        .read_packet()
        .await
        .expect("Failed to read join game");
    assert_eq!(join_game.id, 0x31, "Expected join game packet (0x31)");
    assert!(!join_game.data.is_empty(), "Join game should have data");

    // Read Player Info Update (0x40)
    let player_info = client
        .read_packet()
        .await
        .expect("Failed to read player info update");
    assert_eq!(player_info.id, 0x40, "Expected player info update packet");

    // Read synchronize player position (0x46 in protocol 775)

    let sync_pos = client
        .read_packet()
        .await
        .expect("Failed to read sync position");
    assert_eq!(
        sync_pos.id, 0x48,
        "Expected synchronize player position packet"
    );
}

#[tokio::test]
async fn test_play_basic() {
    let server = TestServer::spawn().await.expect("Failed to spawn server");
    let mut client = TestClient::connect(server.port())
        .await
        .expect("Failed to connect");

    // Complete login + configuration flow
    complete_login_flow(&mut client).await;

    // Send player position (0x1E in protocol 775)
    client
        .send_player_position(100.0, 64.0, 200.0, true)
        .await
        .expect("Failed to send position");

    // The server should handle this without errors
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
}

#[tokio::test]
async fn test_full_protocol_sequence() {
    let server = TestServer::spawn().await.expect("Failed to spawn server");

    // Client A: Status flow then disconnect
    {
        let mut client_a = TestClient::connect(server.port())
            .await
            .expect("Failed to connect A");
        client_a
            .send_handshake(775, 1)
            .await
            .expect("Failed to send handshake A");
        client_a
            .send_status_request()
            .await
            .expect("Failed to send status A");
        let _status = client_a
            .read_packet()
            .await
            .expect("Failed to read status A");
    }

    // Client B: Login flow then disconnect
    {
        let mut client_b = TestClient::connect(server.port())
            .await
            .expect("Failed to connect B");
        complete_login_flow_with_client(&mut client_b, "PlayerB").await;
    }

    // Client C: Login and stay connected
    {
        let mut client_c = TestClient::connect(server.port())
            .await
            .expect("Failed to connect C");
        complete_login_flow_with_client(&mut client_c, "PlayerC").await;

        // Stay connected for a bit
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}

#[tokio::test]
async fn test_concurrent_clients() {
    let server = TestServer::spawn().await.expect("Failed to spawn server");

    let mut handles = Vec::new();

    for i in 0..10 {
        let port = server.port();
        let handle = tokio::spawn(async move {
            let username = format!("Player{i}");
            let mut attempts = 0;
            loop {
                attempts += 1;
                let result = try_complete_login_flow(port, &username).await;
                match result {
                    Ok(()) => break,
                    Err(e) if attempts < 3 => {
                        eprintln!("Client {username} attempt {attempts} failed: {e}, retrying...");
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    }
                    Err(e) => panic!("Client {username} failed after {attempts} attempts: {e}"),
                }
            }
        });
        handles.push(handle);
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }

    // Wait for all clients to complete
    for handle in handles {
        handle.await.expect("Client task panicked");
    }
}

#[tokio::test]
async fn test_error_handling() {
    let server = TestServer::spawn().await.expect("Failed to spawn server");

    // Test 1: Send login start before handshake (should disconnect)
    {
        let mut client = TestClient::connect(server.port())
            .await
            .expect("Failed to connect");
        let uuid = Uuid::new_v4();

        let result = client.send_login_start("BadPlayer", uuid).await;

        if result.is_ok() {
            let read_result =
                tokio::time::timeout(tokio::time::Duration::from_secs(1), client.read_packet())
                    .await;

            assert!(
                read_result.is_err() || read_result.unwrap().is_err(),
                "Server should disconnect on invalid protocol sequence"
            );
        }
    }

    // Test 2: Invalid handshake next_state value
    {
        let mut client = TestClient::connect(server.port())
            .await
            .expect("Failed to connect");

        let result = client.send_handshake(775, 99).await;

        if result.is_ok() {
            let read_result =
                tokio::time::timeout(tokio::time::Duration::from_secs(1), client.read_packet())
                    .await;

            assert!(
                read_result.is_err() || read_result.unwrap().is_err(),
                "Server should reject invalid handshake"
            );
        }
    }
}

#[tokio::test]
async fn test_configuration_phase() {
    let server = TestServer::spawn().await.expect("Failed to spawn server");
    let mut client = TestClient::connect(server.port())
        .await
        .expect("Failed to connect");

    client
        .send_handshake(775, 2)
        .await
        .expect("Failed to send handshake");

    let uuid = Uuid::new_v4();
    client
        .send_login_start("ConfigTest", uuid)
        .await
        .expect("Failed to send login start");

    // Compression
    let compression = client
        .read_packet()
        .await
        .expect("Failed to read compression");
    assert_eq!(compression.id, 0x03);
    client.enable_compression(256);

    // Login Success
    let login_success = client
        .read_packet()
        .await
        .expect("Failed to read login success");
    assert_eq!(login_success.id, 0x02);

    // Send Login Acknowledged
    client
        .send_login_acknowledged()
        .await
        .expect("Failed to send ack");

    // Should receive Known Packs
    let known_packs = client
        .read_packet()
        .await
        .expect("Failed to read known packs");
    assert_eq!(known_packs.id, 0x0E, "Expected Known Packs");

    // Send Known Packs response
    client
        .send_known_packs_response()
        .await
        .expect("Failed to send known packs response");

    // Read all config packets until Finish Configuration
    let mut registry_count = 0;
    let mut got_tags = false;
    loop {
        let packet = client
            .read_packet()
            .await
            .expect("Failed to read config packet");
        match packet.id {
            0x07 => registry_count += 1,
            0x0D => got_tags = true,
            0x03 => break, // Finish Configuration
            other => panic!("Unexpected config packet: {other:#04x}"),
        }
    }

    assert_eq!(
        registry_count, 12,
        "Should receive 12 registry data packets (got {registry_count})"
    );
    assert!(got_tags, "Should receive Update Tags packet");
}

#[tokio::test]
async fn test_chunk_batching() {
    retry_test("test_chunk_batching", 3, || async {
        let server = TestServer::spawn().await?;
        let mut client = TestClient::connect(server.port()).await?;

        complete_login_flow(&mut client).await;

        let game_event = client.read_packet().await?;
        anyhow::ensure!(game_event.id == 0x26, "Expected game event packet");

        let center_chunk = client.read_packet().await?;
        anyhow::ensure!(
            center_chunk.id == 0x58,
            "Expected set center chunk packet before chunk data"
        );

        let batch_start = client.read_packet().await?;
        anyhow::ensure!(batch_start.id == 0x0C, "Expected chunk batch start");

        let mut chunk_count = 0;
        loop {
            let packet = client.read_packet().await?;
            if packet.id == 0x2D {
                chunk_count += 1;
            } else if packet.id == 0x0B {
                break;
            } else {
                anyhow::bail!("Unexpected packet during chunk batch: {:#04x}", packet.id);
            }
        }

        anyhow::ensure!(
            chunk_count > 0,
            "Should receive at least one chunk data packet"
        );
        anyhow::ensure!(
            chunk_count == 289,
            "Should receive 17x17 chunks, got {chunk_count}"
        );
        Ok(())
    })
    .await;
}

async fn try_read_packet_skip_keepalive(
    client: &mut TestClient,
) -> anyhow::Result<common::client::RawPacket> {
    loop {
        let packet = client.read_packet().await?;
        if packet.id != 0x2C {
            return Ok(packet);
        }
    }
}

async fn try_read_packet_skip_keepalive_timeout(
    client: &mut TestClient,
    duration: tokio::time::Duration,
) -> anyhow::Result<Option<common::client::RawPacket>> {
    loop {
        let packet = match tokio::time::timeout(duration, client.read_packet()).await {
            Ok(Ok(p)) => p,
            Ok(Err(e)) => return Err(e.into()),
            Err(_) => return Ok(None),
        };
        if packet.id != 0x2C {
            return Ok(Some(packet));
        }
    }
}

#[tokio::test]
async fn test_chunk_throttling_via_batch_received() {
    use tokio::time::Duration;

    retry_test("test_chunk_throttling_via_batch_received", 3, || async {
        let server = TestServer::spawn().await?;
        let mut client = TestClient::connect(server.port()).await?;

        complete_login_flow(&mut client).await;

        // Consume the initial batch
        let _game_event = try_read_packet_skip_keepalive(&mut client).await?;
        let _center_chunk = try_read_packet_skip_keepalive(&mut client).await?;
        let _batch_start = try_read_packet_skip_keepalive(&mut client).await?;
        loop {
            let packet = try_read_packet_skip_keepalive(&mut client).await?;
            if packet.id == 0x0B {
                break;
            }
        }

        client.send_chunk_batch_received(3.0).await?;

        client.send_player_position(256.0, 64.0, 0.0, true).await?;

        let mut total_chunks = 0;
        let mut batch_count = 0;
        let mut max_batch_size = 0;

        loop {
            let packet =
                match try_read_packet_skip_keepalive_timeout(&mut client, Duration::from_secs(5))
                    .await?
                {
                    Some(p) => p,
                    None => break,
                };
            if packet.id == 0x0C {
                let mut batch_size = 0;
                loop {
                    let inner = try_read_packet_skip_keepalive(&mut client).await?;
                    if inner.id == 0x2D {
                        batch_size += 1;
                    } else if inner.id == 0x0B {
                        break;
                    } else if inner.id == 0x25 || inner.id == 0x2C {
                        continue;
                    } else {
                        anyhow::bail!("Unexpected packet in batch: {:#04x}", inner.id);
                    }
                }
                total_chunks += batch_size;
                batch_count += 1;
                if batch_size > max_batch_size {
                    max_batch_size = batch_size;
                }

                client.send_chunk_batch_received(3.0).await?;
            } else if packet.id == 0x25 || packet.id == 0x2C {
                continue;
            } else if packet.id == 0x58 {
                continue;
            } else {
                break;
            }
        }

        anyhow::ensure!(
            batch_count >= 2,
            "Expected multiple batches, got {batch_count}"
        );
        anyhow::ensure!(
            max_batch_size <= 3,
            "No batch should exceed 3 chunks, but got {max_batch_size}"
        );
        anyhow::ensure!(
            total_chunks > 3,
            "Should receive more than 3 total chunks, got {total_chunks}"
        );
        Ok(())
    })
    .await;
}

#[tokio::test]
async fn test_client_tick_end_drains_chunks() {
    retry_test("test_client_tick_end_drains_chunks", 3, || async {
        let server = TestServer::spawn().await?;
        let mut client = TestClient::connect(server.port()).await?;

        complete_login_flow(&mut client).await;

        let game_event = try_read_packet_skip_keepalive(&mut client).await?;
        anyhow::ensure!(game_event.id == 0x26, "Expected game event packet");

        let center_chunk = try_read_packet_skip_keepalive(&mut client).await?;
        anyhow::ensure!(center_chunk.id == 0x58, "Expected set center chunk packet");

        let batch_start = try_read_packet_skip_keepalive(&mut client).await?;
        anyhow::ensure!(batch_start.id == 0x0C, "Expected chunk batch start");

        loop {
            let packet = try_read_packet_skip_keepalive(&mut client).await?;
            if packet.id == 0x0B {
                break;
            }
            anyhow::ensure!(
                packet.id == 0x2D || packet.id == 0x58 || packet.id == 0x2C,
                "Expected chunk data, batch finished, or skippable packet, got {:#04x}",
                packet.id
            );
        }

        client.send_chunk_batch_received(25.0).await?;

        client
            .send_player_position(1000.0, 64.0, 1000.0, true)
            .await?;

        let mut position_chunks = 0;
        loop {
            let packet =
                tokio::time::timeout(tokio::time::Duration::from_secs(5), client.read_packet())
                    .await
                    .map_err(|_| anyhow::anyhow!("Timed out reading position response"))??;

            match packet.id {
                0x25 | 0x58 | 0x2C => {}
                0x0C => {}
                0x2D => position_chunks += 1,
                0x0B => break,
                other => {
                    anyhow::bail!("Unexpected packet during position response: {other:#04x}")
                }
            }
        }
        anyhow::ensure!(
            position_chunks > 0 && position_chunks <= 25,
            "Position handler should drain at most 25 chunks (got {position_chunks})"
        );

        client.send_client_tick_end().await?;

        let response = try_read_packet_skip_keepalive_timeout(
            &mut client,
            tokio::time::Duration::from_secs(5),
        )
        .await?
        .ok_or_else(|| anyhow::anyhow!("Timed out waiting for chunk response after tick end"))?;

        anyhow::ensure!(
            response.id == 0x0C,
            "Expected chunk batch start after client tick end"
        );

        let mut chunk_count = 0;
        loop {
            let packet = try_read_packet_skip_keepalive(&mut client).await?;
            if packet.id == 0x2D {
                chunk_count += 1;
            } else if packet.id == 0x0B {
                break;
            } else if packet.id == 0x2C {
                continue;
            } else {
                anyhow::bail!("Unexpected packet during chunk batch: {:#04x}", packet.id);
            }
        }

        anyhow::ensure!(
            chunk_count > 0,
            "Client Tick End should have drained pending chunks"
        );
        Ok(())
    })
    .await;
}
#[tokio::test]
async fn test_configuration_timeout() {
    let server = TestServer::spawn_with_env(&[("RUSTMC_NON_PLAY_TIMEOUT", "3")])
        .await
        .expect("Failed to spawn server");
    let mut client = TestClient::connect(server.port())
        .await
        .expect("Failed to connect");

    client
        .send_handshake(775, 2)
        .await
        .expect("Failed to send handshake");
    let uuid = Uuid::new_v4();
    client
        .send_login_start("TimeoutTest", uuid)
        .await
        .expect("Failed to send login start");

    let compression = client
        .read_packet()
        .await
        .expect("Failed to read compression");
    assert_eq!(compression.id, 0x03);
    client.enable_compression(256);

    let _login_success = client
        .read_packet()
        .await
        .expect("Failed to read login success");

    client
        .send_login_acknowledged()
        .await
        .expect("Failed to send login acknowledged");

    let _known_packs = client
        .read_packet()
        .await
        .expect("Failed to read known packs");

    client
        .send_known_packs_response()
        .await
        .expect("Failed to send known packs response");

    loop {
        let packet = client
            .read_packet()
            .await
            .expect("Failed to read config packet");
        if packet.id == 0x03 {
            break;
        }
    }

    let start = std::time::Instant::now();
    let result =
        tokio::time::timeout(tokio::time::Duration::from_secs(10), client.read_packet()).await;
    let elapsed = start.elapsed();

    assert!(
        result.is_err() || result.unwrap().is_err(),
        "Server should drop the connection after timeout"
    );
    assert!(
        elapsed.as_secs() >= 2 && elapsed.as_secs() <= 5,
        "Timeout should fire around 3 seconds, but took {}s",
        elapsed.as_secs()
    );
}

/// Helper to complete the full login + configuration flow
async fn complete_login_flow(client: &mut TestClient) {
    complete_login_flow_with_client(client, "TestPlayer").await;
}

async fn complete_login_flow_with_client(client: &mut TestClient, username: &str) {
    client
        .send_handshake(775, 2)
        .await
        .expect("Failed to send handshake");
    let uuid = Uuid::new_v4();
    client
        .send_login_start(username, uuid)
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
            break; // Finish Configuration
        }
    }

    // Send Acknowledge Finish Configuration to transition to Play
    client
        .send_acknowledge_finish_configuration()
        .await
        .expect("Failed to send acknowledge finish configuration");

    // Read join game (0x30)

    let _join_game = client
        .read_packet()
        .await
        .expect("Failed to read join game");

    // Read Player Info Update (0x40)
    let _player_info = client
        .read_packet()
        .await
        .expect("Failed to read player info update");

    // Read sync position (0x46)

    let _sync_pos = client
        .read_packet()
        .await
        .expect("Failed to read sync position");
}

async fn try_complete_login_flow(port: u16, username: &str) -> anyhow::Result<()> {
    let mut client = TestClient::connect(port).await?;
    client.send_handshake(775, 2).await?;
    let uuid = Uuid::new_v4();
    client.send_login_start(username, uuid).await?;

    // Compression
    let _compression = client.read_packet().await?;
    client.enable_compression(256);

    // Login Success
    let _login_success = client.read_packet().await?;

    // Login Acknowledged
    client.send_login_acknowledged().await?;

    // Known Packs
    let _known_packs = client.read_packet().await?;

    // Send Known Packs response
    client.send_known_packs_response().await?;

    // Read configuration packets until Finish Configuration
    loop {
        let packet = client.read_packet().await?;
        if packet.id == 0x03 {
            break;
        }
    }

    // Send Acknowledge Finish Configuration to transition to Play
    client.send_acknowledge_finish_configuration().await?;

    // Read join game (0x30)
    let _join_game = client.read_packet().await?;

    // Read Player Info Update (0x40)
    let _player_info = client.read_packet().await?;

    // Read sync position (0x46)
    let _sync_pos = client.read_packet().await?;

    Ok(())
}
