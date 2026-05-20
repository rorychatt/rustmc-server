mod common;

use common::{TestClient, TestServer};
use rustmc_server::protocol::packet_ids;
use std::io::{Cursor, Read};
use uuid::Uuid;

use packet_ids::configuration::clientbound as config_cb;
use packet_ids::login::clientbound as login_cb;
use packet_ids::play::clientbound as play_cb;
use packet_ids::status::clientbound as status_cb;

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
    assert_eq!(
        response.id, status_cb::STATUS_RESPONSE,
        "Expected status response packet"
    );

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
    assert_eq!(pong.id, status_cb::PONG_RESPONSE, "Expected pong packet");
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

    // Read set compression packet
    let compression = client
        .read_packet()
        .await
        .expect("Failed to read compression packet");
    assert_eq!(
        compression.id, login_cb::SET_COMPRESSION,
        "Expected set compression packet"
    );

    // Enable compression on client side
    client.enable_compression(256);

    // Read login success
    let login_success = client
        .read_packet()
        .await
        .expect("Failed to read login success");
    assert_eq!(
        login_success.id, login_cb::LOGIN_SUCCESS,
        "Expected login success packet"
    );

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

    // Read Known Packs packet
    let known_packs = client
        .read_packet()
        .await
        .expect("Failed to read known packs");
    assert_eq!(
        known_packs.id, config_cb::KNOWN_PACKS,
        "Expected known packs packet"
    );

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
            id if id == config_cb::REGISTRY_DATA => {}
            id if id == config_cb::UPDATE_TAGS => {}
            id if id == config_cb::FINISH_CONFIGURATION => {
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

    // Read join game packet
    let join_game = client
        .read_packet()
        .await
        .expect("Failed to read join game");
    assert_eq!(join_game.id, play_cb::LOGIN_PLAY, "Expected join game packet");
    assert!(!join_game.data.is_empty(), "Join game should have data");

    // Read Player Info Update
    let player_info = client
        .read_packet()
        .await
        .expect("Failed to read player info update");
    assert_eq!(
        player_info.id, play_cb::PLAYER_INFO_UPDATE,
        "Expected player info update packet"
    );

    // Read synchronize player position
    let sync_pos = client
        .read_packet()
        .await
        .expect("Failed to read sync position");
    assert_eq!(
        sync_pos.id, play_cb::SYNCHRONIZE_PLAYER_POSITION,
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

    // Send player position
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
            let mut client = TestClient::connect(port).await.expect("Failed to connect");
            let username = format!("Player{i}");
            complete_login_flow_with_client(&mut client, &username).await;
        });
        handles.push(handle);
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
    assert_eq!(compression.id, login_cb::SET_COMPRESSION);
    client.enable_compression(256);

    // Login Success
    let login_success = client
        .read_packet()
        .await
        .expect("Failed to read login success");
    assert_eq!(login_success.id, login_cb::LOGIN_SUCCESS);

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
    assert_eq!(known_packs.id, config_cb::KNOWN_PACKS, "Expected Known Packs");

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
            id if id == config_cb::REGISTRY_DATA => registry_count += 1,
            id if id == config_cb::UPDATE_TAGS => got_tags = true,
            id if id == config_cb::FINISH_CONFIGURATION => break,
            other => panic!("Unexpected config packet: {other:#04x}"),
        }
    }

    assert_eq!(
        registry_count, 12,
        "Should receive 12 registry data packets (got {registry_count})"
    );
    assert!(got_tags, "Should receive Update Tags packet");

    // Send Acknowledge Finish Configuration to transition to Play
    client
        .send_acknowledge_finish_configuration()
        .await
        .expect("Failed to send acknowledge finish configuration");

    // Verify server transitions to Play by sending LOGIN_PLAY (join game)
    let join_game = client
        .read_packet()
        .await
        .expect("Failed to read join game after acknowledge finish");
    assert_eq!(
        join_game.id, play_cb::LOGIN_PLAY,
        "Expected LOGIN_PLAY packet after configuration phase completes (got {:#04x})",
        join_game.id
    );
}

#[tokio::test]
async fn test_chunk_batching() {
    let server = TestServer::spawn().await.expect("Failed to spawn server");
    let mut client = TestClient::connect(server.port())
        .await
        .expect("Failed to connect");

    complete_login_flow(&mut client).await;

    // After login, we should have received Game Event, Set Center Chunk, Chunk Batch Start,
    // chunks, and Chunk Batch Finished.

    // Read Game Event
    let game_event = client
        .read_packet()
        .await
        .expect("Failed to read game event");
    assert_eq!(game_event.id, play_cb::GAME_EVENT, "Expected game event packet");

    // Read Set Center Chunk
    let center_chunk = client
        .read_packet()
        .await
        .expect("Failed to read set center chunk");
    assert_eq!(
        center_chunk.id, play_cb::SET_CENTER_CHUNK,
        "Expected set center chunk packet before chunk data"
    );

    // Read Chunk Batch Start
    let batch_start = client
        .read_packet()
        .await
        .expect("Failed to read chunk batch start");
    assert_eq!(batch_start.id, play_cb::CHUNK_BATCH_START, "Expected chunk batch start");

    // Read chunk data packets
    let mut chunk_count = 0;
    loop {
        let packet = client
            .read_packet()
            .await
            .expect("Failed to read chunk/batch packet");
        if packet.id == play_cb::LEVEL_CHUNK_WITH_LIGHT {
            chunk_count += 1;
        } else if packet.id == play_cb::CHUNK_BATCH_FINISHED {
            break;
        } else {
            panic!("Unexpected packet during chunk batch: {:#04x}", packet.id);
        }
    }

    assert!(
        chunk_count > 0,
        "Should receive at least one chunk data packet"
    );
    // 17x17 = 289 chunks for view distance 8
    assert_eq!(chunk_count, 289, "Should receive 17x17 chunks");
}

#[tokio::test]
async fn test_chunk_throttling_via_batch_received() {
    use tokio::time::{timeout, Duration};

    let server = TestServer::spawn().await.expect("Failed to spawn server");
    let mut client = TestClient::connect(server.port())
        .await
        .expect("Failed to connect");

    complete_login_flow(&mut client).await;

    // Consume the initial batch: Game Event, Set Center Chunk, Chunk Batch Start, chunks, Chunk Batch Finished
    let _game_event = client.read_packet().await.unwrap();
    let _center_chunk = client.read_packet().await.unwrap();
    let _batch_start = client.read_packet().await.unwrap();
    loop {
        let packet = client.read_packet().await.unwrap();
        if packet.id == play_cb::CHUNK_BATCH_FINISHED {
            break;
        }
    }

    // Tell the server we can only handle 3 chunks per tick
    client.send_chunk_batch_received(3.0).await.unwrap();

    // Move far enough to require new chunks (256 blocks in X)
    client
        .send_player_position(256.0, 64.0, 0.0, true)
        .await
        .unwrap();

    // Read chunk batches — each batch should have at most 3 chunks
    let mut total_chunks = 0;
    let mut batch_count = 0;
    let mut max_batch_size = 0;

    // Drain all pending chunks by repeatedly acknowledging batches
    loop {
        let packet = match timeout(Duration::from_secs(2), client.read_packet()).await {
            Ok(Ok(p)) => p,
            _ => break,
        };
        if packet.id == play_cb::CHUNK_BATCH_START {
            // Chunk Batch Start — read chunks until Chunk Batch Finished
            let mut batch_size = 0;
            loop {
                let inner = client.read_packet().await.unwrap();
                if inner.id == play_cb::LEVEL_CHUNK_WITH_LIGHT {
                    batch_size += 1;
                } else if inner.id == play_cb::CHUNK_BATCH_FINISHED {
                    break;
                } else if inner.id == play_cb::UNLOAD_CHUNK || inner.id == play_cb::KEEP_ALIVE {
                    continue;
                } else {
                    panic!("Unexpected packet in batch: {:#04x}", inner.id);
                }
            }
            total_chunks += batch_size;
            batch_count += 1;
            if batch_size > max_batch_size {
                max_batch_size = batch_size;
            }

            // Acknowledge this batch to trigger the next drain
            client.send_chunk_batch_received(3.0).await.unwrap();
        } else if packet.id == play_cb::UNLOAD_CHUNK
            || packet.id == play_cb::KEEP_ALIVE
            || packet.id == play_cb::SET_CENTER_CHUNK
        {
            continue;
        } else {
            // No more batch starts — we're done
            break;
        }
    }

    assert!(
        batch_count >= 2,
        "Expected multiple batches, got {batch_count}"
    );
    assert!(
        max_batch_size <= 3,
        "No batch should exceed 3 chunks, but got {max_batch_size}"
    );
    assert!(
        total_chunks > 3,
        "Should receive more than 3 total chunks, got {total_chunks}"
    );
}

#[tokio::test]
async fn test_client_tick_end_drains_chunks() {
    let server = TestServer::spawn().await.expect("Failed to spawn server");
    let mut client = TestClient::connect(server.port())
        .await
        .expect("Failed to connect");

    complete_login_flow(&mut client).await;

    // Consume initial chunk batch (Game Event, Set Center Chunk, Chunk Batch Start, chunks, Chunk Batch Finished)
    let game_event = client
        .read_packet()
        .await
        .expect("Failed to read game event");
    assert_eq!(game_event.id, play_cb::GAME_EVENT, "Expected game event packet");

    let center_chunk = client
        .read_packet()
        .await
        .expect("Failed to read set center chunk");
    assert_eq!(
        center_chunk.id, play_cb::SET_CENTER_CHUNK,
        "Expected set center chunk packet"
    );

    let batch_start = client
        .read_packet()
        .await
        .expect("Failed to read chunk batch start");
    assert_eq!(batch_start.id, play_cb::CHUNK_BATCH_START, "Expected chunk batch start");

    loop {
        let packet = client
            .read_packet()
            .await
            .expect("Failed to read chunk/batch packet");
        if packet.id == play_cb::CHUNK_BATCH_FINISHED {
            break;
        }
        assert_eq!(
            packet.id, play_cb::LEVEL_CHUNK_WITH_LIGHT,
            "Expected chunk data or batch finished"
        );
    }

    // Acknowledge the initial batch so the server knows we're ready
    client
        .send_chunk_batch_received(25.0)
        .await
        .expect("Failed to send chunk batch received");

    // Move far away to queue new chunks
    client
        .send_player_position(1000.0, 64.0, 1000.0, true)
        .await
        .expect("Failed to send position");

    // Consume the position response: unload packets + first drain batch
    let mut position_chunks = 0;
    loop {
        let packet = tokio::time::timeout(
            tokio::time::Duration::from_secs(5),
            client.read_packet(),
        )
        .await
        .expect("Timed out reading position response")
        .expect("Failed to read position response packet");

        match packet.id {
            id if id == play_cb::UNLOAD_CHUNK || id == play_cb::SET_CENTER_CHUNK => {}
            id if id == play_cb::KEEP_ALIVE => {}
            id if id == play_cb::CHUNK_BATCH_START => {}
            id if id == play_cb::LEVEL_CHUNK_WITH_LIGHT => position_chunks += 1,
            id if id == play_cb::CHUNK_BATCH_FINISHED => {
                break;
            }
            other => panic!("Unexpected packet during position response: {other:#04x}"),
        }
    }
    assert!(
        position_chunks > 0 && position_chunks <= 25,
        "Position handler should drain at most 25 chunks (got {position_chunks})"
    );

    // Now send Client Tick End — should drain more pending chunks
    client
        .send_client_tick_end()
        .await
        .expect("Failed to send client tick end");

    // Read the chunk batch triggered by tick end (skip Keep Alive packets)
    let response = loop {
        let pkt = tokio::time::timeout(
            tokio::time::Duration::from_secs(5),
            client.read_packet(),
        )
        .await
        .expect("Timed out waiting for chunk response after tick end")
        .expect("Failed to read packet after tick end");
        if pkt.id != play_cb::KEEP_ALIVE {
            break pkt;
        }
    };

    assert_eq!(
        response.id, play_cb::CHUNK_BATCH_START,
        "Expected chunk batch start after client tick end"
    );

    let mut chunk_count = 0;
    loop {
        let packet = client
            .read_packet()
            .await
            .expect("Failed to read chunk/batch packet");
        if packet.id == play_cb::LEVEL_CHUNK_WITH_LIGHT {
            chunk_count += 1;
        } else if packet.id == play_cb::CHUNK_BATCH_FINISHED {
            break;
        } else if packet.id == play_cb::KEEP_ALIVE {
            continue;
        } else {
            panic!(
                "Unexpected packet during chunk batch: {:#04x}",
                packet.id
            );
        }
    }

    assert!(
        chunk_count > 0,
        "Client Tick End should have drained pending chunks"
    );
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
    assert_eq!(compression.id, login_cb::SET_COMPRESSION);
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
        if packet.id == config_cb::FINISH_CONFIGURATION {
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
        if packet.id == config_cb::FINISH_CONFIGURATION {
            break;
        }
    }

    // Send Acknowledge Finish Configuration to transition to Play
    client
        .send_acknowledge_finish_configuration()
        .await
        .expect("Failed to send acknowledge finish configuration");

    // Read join game
    let _join_game = client
        .read_packet()
        .await
        .expect("Failed to read join game");

    // Read Player Info Update
    let _player_info = client
        .read_packet()
        .await
        .expect("Failed to read player info update");

    // Read sync position
    let _sync_pos = client
        .read_packet()
        .await
        .expect("Failed to read sync position");
}

#[tokio::test]
async fn test_custom_gameplay_configuration() {
    use std::fs::File;
    use std::io::Write;
    use rustmc_server::protocol::types::VarInt;

    // Create a temporary server config file
    let config_dir = std::env::temp_dir();
    let config_path = config_dir.join(format!("test_config_{}.yaml", uuid::Uuid::new_v4()));
    let mut config_file = File::create(&config_path).expect("Failed to create temp config file");

    let yaml_content = r#"
server:
  bind: "127.0.0.1:0"
  view_distance: 6

rate_limit:
  invalid_packet_threshold: 16
  invalid_packet_window_secs: 10

gameplay:
  motd: "Configured Test MOTD"
  max_players: 77
  gamemode: "survival"
  difficulty: "hard"
  pvp: true
  allow_flight: true
  hardcore: true
  simulation_distance: 5
  sea_level: 60
"#;
    config_file.write_all(yaml_content.as_bytes()).expect("Failed to write temp config");

    // Spawn server with RUSTMC_CONFIG env pointing to our config file
    let config_path_str = config_path.to_string_lossy().into_owned();
    let server = TestServer::spawn_with_env(&[("RUSTMC_CONFIG", &config_path_str)])
        .await
        .expect("Failed to spawn server with custom config");

    // Step 1: Verify Status response MOTD and max players
    let mut client = TestClient::connect(server.port())
        .await
        .expect("Failed to connect to server");

    client.send_handshake(775, 1).await.expect("Failed to send handshake");
    client.send_status_request().await.expect("Failed to send status request");

    let response = client.read_packet().await.expect("Failed to read status response");
    assert_eq!(response.id, status_cb::STATUS_RESPONSE);

    let mut cursor = Cursor::new(&response.data);
    let json_str = common::client::read_string(&mut cursor).expect("Failed to read JSON string");
    let json: serde_json::Value = serde_json::from_str(&json_str).expect("Failed to parse JSON");

    assert_eq!(json["description"]["text"].as_str().unwrap(), "Configured Test MOTD");
    assert_eq!(json["players"]["max"].as_i64().unwrap(), 77);

    // Clean up current client connection
    drop(client);

    // Step 2: Verify Login Play packet values
    let mut client = TestClient::connect(server.port())
        .await
        .expect("Failed to connect to server for play");

    client.send_handshake(775, 2).await.expect("Failed to send handshake for play");
    client.send_login_start("TestConfigPlayer", Uuid::new_v4()).await.expect("Failed to send login start");

    // Read compression
    let comp_packet = client.read_packet().await.expect("Failed to read compression");
    assert_eq!(comp_packet.id, login_cb::SET_COMPRESSION);
    let mut comp_cursor = Cursor::new(&comp_packet.data);
    let threshold = VarInt::read(&mut comp_cursor).unwrap().0;
    client.enable_compression(threshold);

    // Read Login Success
    let success = client.read_packet().await.expect("Failed to read login success");
    assert_eq!(success.id, login_cb::LOGIN_SUCCESS);

    // Acknowledge Login
    client.send_login_acknowledged().await.expect("Failed to send login ack");

    // Read Known Packs
    let packs = client.read_packet().await.expect("Failed to read known packs");
    assert_eq!(packs.id, config_cb::KNOWN_PACKS);

    // Respond Known Packs
    client.send_known_packs_response().await.expect("Failed to send known packs response");

    // Skip registry & config finish
    loop {
        let packet = client.read_packet().await.expect("Failed to read config packet");
        if packet.id == config_cb::FINISH_CONFIGURATION {
            break;
        }
    }

    // Acknowledge Config Finish
    client.send_acknowledge_finish_configuration().await.expect("Failed to send config ack");

    // Read login play packet
    let join_game = client.read_packet().await.expect("Failed to read join game packet");
    assert_eq!(join_game.id, play_cb::LOGIN_PLAY);

    // Decode and verify login play values
    let mut play_cursor = Cursor::new(&join_game.data);
    
    // Read Entity ID (4 bytes)
    let mut entity_id_bytes = [0u8; 4];
    play_cursor.read_exact(&mut entity_id_bytes).unwrap();
    
    // Read Is Hardcore (1 byte)
    let mut hardcore_bytes = [0u8; 1];
    play_cursor.read_exact(&mut hardcore_bytes).unwrap();
    let is_hardcore = hardcore_bytes[0] != 0;
    assert!(is_hardcore, "Hardcore should be true");
    
    // Read Dimension count (VarInt)
    let _dim_count = VarInt::read(&mut play_cursor).unwrap().0;
    
    // Read Dimension name (String)
    let _dim_name = common::client::read_string(&mut play_cursor).unwrap();
    
    // Read Max players (VarInt)
    let max_players_decoded = VarInt::read(&mut play_cursor).unwrap().0;
    assert_eq!(max_players_decoded, 77);
    
    // Read View distance (VarInt)
    let view_dist = VarInt::read(&mut play_cursor).unwrap().0;
    assert_eq!(view_dist, 6);
    
    // Read Simulation distance (VarInt)
    let sim_dist = VarInt::read(&mut play_cursor).unwrap().0;
    assert_eq!(sim_dist, 5);
    
    // Read debug, respawn, crafting (3 bytes)
    let mut flags = [0u8; 3];
    play_cursor.read_exact(&mut flags).unwrap();
    
    // Read Dimension Type (VarInt)
    let _dim_type = VarInt::read(&mut play_cursor).unwrap().0;
    
    // Read Dimension name (String)
    let _dim_name_2 = common::client::read_string(&mut play_cursor).unwrap();
    
    // Read Hashed seed (8 bytes)
    let mut seed = [0u8; 8];
    play_cursor.read_exact(&mut seed).unwrap();
    
    // Read Game mode (1 byte)
    let mut gm = [0u8; 1];
    play_cursor.read_exact(&mut gm).unwrap();
    let game_mode_decoded = gm[0];
    assert_eq!(game_mode_decoded, 0, "Game mode should be survival (0)");

    // Read player info update
    let player_info = client.read_packet().await.expect("Failed to read player info update");
    assert_eq!(player_info.id, play_cb::PLAYER_INFO_UPDATE);

    // Clean up temporary config file
    let _ = std::fs::remove_file(&config_path);
}

