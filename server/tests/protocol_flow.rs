mod common;

use common::{retry_test, TestClient, TestServer};
use rustmc_server::protocol::packet_ids;
use serial_test::serial;
use std::io::{Cursor, Read};
use uuid::Uuid;

use packet_ids::configuration::clientbound as config_cb;
use packet_ids::login::clientbound as login_cb;
use packet_ids::play::clientbound as play_cb;
use packet_ids::status::clientbound as status_cb;

#[tokio::test]
#[serial]
async fn test_status_flow() {
    retry_test("test_status_flow", 3, || async {
        let server = TestServer::spawn().await?;
        let mut client = TestClient::connect(server.port()).await?;

        client.send_handshake(775, 1).await?;
        client.send_status_request().await?;

        let response = client.read_packet().await?;
        assert_eq!(
            response.id,
            status_cb::STATUS_RESPONSE,
            "Expected status response packet"
        );

        let mut cursor = Cursor::new(&response.data);
        let json_str =
            common::client::read_string(&mut cursor).expect("Failed to read JSON string");
        let json: serde_json::Value =
            serde_json::from_str(&json_str).expect("Failed to parse JSON");

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

        let ping_time = std::time::Instant::now();
        let payload = 12345i64;
        client.send_ping(payload).await?;

        let pong = client.read_packet().await?;
        let elapsed = ping_time.elapsed();
        assert_eq!(pong.id, status_cb::PONG_RESPONSE, "Expected pong packet");
        assert!(elapsed.as_millis() < 100, "Ping should be under 100ms");

        let pong_payload = i64::from_be_bytes(pong.data.try_into().expect("Invalid pong data"));
        assert_eq!(
            pong_payload, payload,
            "Pong payload should match ping payload"
        );

        Ok(())
    })
    .await;
}

#[tokio::test]
#[serial]
async fn test_login_flow() {
    retry_test("test_login_flow", 3, || async {
        let server = TestServer::spawn().await?;
        let mut client = TestClient::connect(server.port()).await?;

        client.send_handshake(775, 2).await?;

        let uuid = Uuid::new_v4();
        let username = "TestPlayer";
        client.send_login_start(username, uuid).await?;

        let compression = client.read_packet().await?;
        assert_eq!(
            compression.id,
            login_cb::SET_COMPRESSION,
            "Expected set compression packet"
        );

        client.enable_compression(256);

        let login_success = client.read_packet().await?;
        assert_eq!(
            login_success.id,
            login_cb::LOGIN_SUCCESS,
            "Expected login success packet"
        );

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

        client.send_login_acknowledged().await?;

        let known_packs = client.read_packet().await?;
        assert_eq!(
            known_packs.id,
            config_cb::KNOWN_PACKS,
            "Expected known packs packet"
        );

        client.send_known_packs_response().await?;

        let got_finish;
        loop {
            let packet = client.read_packet().await?;
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

        client.send_acknowledge_finish_configuration().await?;

        let join_game = client.read_packet().await?;
        assert_eq!(
            join_game.id,
            play_cb::LOGIN_PLAY,
            "Expected join game packet"
        );
        assert!(!join_game.data.is_empty(), "Join game should have data");

        let player_info = client.read_packet().await?;
        assert_eq!(
            player_info.id,
            play_cb::PLAYER_INFO_UPDATE,
            "Expected player info update packet"
        );

        let sync_pos = client.read_packet().await?;
        assert_eq!(
            sync_pos.id,
            play_cb::SYNCHRONIZE_PLAYER_POSITION,
            "Expected synchronize player position packet"
        );

        Ok(())
    })
    .await;
}

#[tokio::test]
#[serial]
async fn test_play_basic() {
    retry_test("test_play_basic", 3, || async {
        let server = TestServer::spawn().await?;
        let mut client = TestClient::connect(server.port()).await?;

        complete_login_flow(&mut client).await;

        client
            .send_player_position(100.0, 64.0, 200.0, true)
            .await?;

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        Ok(())
    })
    .await;
}

#[tokio::test]
#[serial]
async fn test_full_protocol_sequence() {
    retry_test("test_full_protocol_sequence", 3, || async {
        let server = TestServer::spawn().await?;

        {
            let mut client_a = TestClient::connect(server.port()).await?;
            client_a.send_handshake(775, 1).await?;
            client_a.send_status_request().await?;
            let _status = client_a.read_packet().await?;
        }

        {
            let mut client_b = TestClient::connect(server.port()).await?;
            complete_login_flow_with_client(&mut client_b, "PlayerB").await;
        }

        {
            let mut client_c = TestClient::connect(server.port()).await?;
            complete_login_flow_with_client(&mut client_c, "PlayerC").await;

            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        Ok(())
    })
    .await;
}

#[tokio::test]
#[serial]
async fn test_concurrent_clients() {
    retry_test("test_concurrent_clients", 3, || async {
        let server = TestServer::spawn().await?;

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

        for handle in handles {
            handle.await.expect("Client task panicked");
        }

        Ok(())
    })
    .await;
}

#[tokio::test]
#[serial]
async fn test_error_handling() {
    retry_test("test_error_handling", 3, || async {
        let server = TestServer::spawn().await?;

        // Test 1: Send login start before handshake (should disconnect)
        {
            let mut client = TestClient::connect(server.port()).await?;
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
            let mut client = TestClient::connect(server.port()).await?;

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

        Ok(())
    })
    .await;
}

#[tokio::test]
#[serial]
async fn test_configuration_phase() {
    retry_test("test_configuration_phase", 3, || async {
        let server = TestServer::spawn().await?;
        let mut client = TestClient::connect(server.port()).await?;

        client.send_handshake(775, 2).await?;

        let uuid = Uuid::new_v4();
        client.send_login_start("ConfigTest", uuid).await?;

        let compression = client.read_packet().await?;
        assert_eq!(compression.id, login_cb::SET_COMPRESSION);
        client.enable_compression(256);

        let login_success = client.read_packet().await?;
        assert_eq!(login_success.id, login_cb::LOGIN_SUCCESS);

        client.send_login_acknowledged().await?;

        let known_packs = client.read_packet().await?;
        assert_eq!(
            known_packs.id,
            config_cb::KNOWN_PACKS,
            "Expected Known Packs"
        );

        client.send_known_packs_response().await?;

        let mut registry_count = 0;
        let mut got_tags = false;
        loop {
            let packet = client.read_packet().await?;
            match packet.id {
                id if id == config_cb::REGISTRY_DATA => registry_count += 1,
                id if id == config_cb::UPDATE_TAGS => got_tags = true,
                id if id == config_cb::FINISH_CONFIGURATION => break,
                other => panic!("Unexpected config packet: {other:#04x}"),
            }
        }

        assert_eq!(
            registry_count, 28,
            "Should receive 28 registry data packets (got {registry_count})"
        );
        assert!(got_tags, "Should receive Update Tags packet");

        client.send_acknowledge_finish_configuration().await?;

        let join_game = client.read_packet().await?;
        assert_eq!(
            join_game.id,
            play_cb::LOGIN_PLAY,
            "Expected LOGIN_PLAY packet after configuration phase completes (got {:#04x})",
            join_game.id
        );

        Ok(())
    })
    .await;
}

#[tokio::test]
#[serial]
async fn test_chunk_batching() {
    retry_test("test_chunk_batching", 3, || async {
        let server = TestServer::spawn().await?;
        let mut client = TestClient::connect(server.port()).await?;

        complete_login_flow(&mut client).await;

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
            "Expected set center chunk packet before chunk data"
        );

        let batch_start = client.read_packet().await?;
        assert_eq!(
            batch_start.id,
            play_cb::CHUNK_BATCH_START,
            "Expected chunk batch start"
        );

        let mut chunk_count = 0;
        loop {
            let packet = client.read_packet().await?;
            if packet.id == play_cb::LEVEL_CHUNK_WITH_LIGHT {
                chunk_count += 1;
            } else if packet.id == play_cb::CHUNK_BATCH_FINISHED {
                break;
            } else {
                anyhow::bail!("Unexpected packet during chunk batch: {:#04x}", packet.id);
            }
        }

        assert!(
            chunk_count > 0,
            "Should receive at least one chunk data packet"
        );
        assert_eq!(chunk_count, 289, "Should receive 17x17 chunks");

        Ok(())
    })
    .await;
}

#[tokio::test]
#[serial]
async fn test_chunk_throttling_via_batch_received() {
    retry_test("test_chunk_throttling_via_batch_received", 3, || async {
        use tokio::time::{timeout, Duration};

        let server = TestServer::spawn().await?;
        let mut client = TestClient::connect(server.port()).await?;

        complete_login_flow(&mut client).await;

        let _game_event = client.read_packet().await?;
        let _center_chunk = client.read_packet().await?;
        let _batch_start = client.read_packet().await?;
        loop {
            let packet = client.read_packet().await?;
            if packet.id == play_cb::CHUNK_BATCH_FINISHED {
                break;
            }
        }

        client.send_chunk_batch_received(3.0).await?;

        client.send_player_position(256.0, 64.0, 0.0, true).await?;

        let mut total_chunks = 0;
        let mut batch_count = 0;
        let mut max_batch_size = 0;

        loop {
            let packet = match timeout(Duration::from_secs(2), client.read_packet()).await {
                Ok(Ok(p)) => p,
                _ => break,
            };
            if packet.id == play_cb::CHUNK_BATCH_START {
                let mut batch_size = 0;
                loop {
                    let inner = client.read_packet().await?;
                    if inner.id == play_cb::LEVEL_CHUNK_WITH_LIGHT {
                        batch_size += 1;
                    } else if inner.id == play_cb::CHUNK_BATCH_FINISHED {
                        break;
                    } else if inner.id == play_cb::UNLOAD_CHUNK || inner.id == play_cb::KEEP_ALIVE {
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
            } else if packet.id == play_cb::UNLOAD_CHUNK
                || packet.id == play_cb::KEEP_ALIVE
                || packet.id == play_cb::SET_CENTER_CHUNK
            {
                continue;
            } else {
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

        Ok(())
    })
    .await;
}

#[tokio::test]
#[serial]
async fn test_client_tick_end_drains_chunks() {
    retry_test("test_client_tick_end_drains_chunks", 3, || async {
        let server = TestServer::spawn().await?;
        let mut client = TestClient::connect(server.port()).await?;

        complete_login_flow(&mut client).await;

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
            assert_eq!(
                packet.id,
                play_cb::LEVEL_CHUNK_WITH_LIGHT,
                "Expected chunk data or batch finished"
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
                    .map_err(|_| anyhow::anyhow!("Timed out reading position response"))?
                    .map_err(|e| anyhow::anyhow!("Failed to read position response packet: {e}"))?;

            match packet.id {
                id if id == play_cb::UNLOAD_CHUNK || id == play_cb::SET_CENTER_CHUNK => {}
                id if id == play_cb::KEEP_ALIVE => {}
                id if id == play_cb::CHUNK_BATCH_START => {}
                id if id == play_cb::LEVEL_CHUNK_WITH_LIGHT => position_chunks += 1,
                id if id == play_cb::CHUNK_BATCH_FINISHED => {
                    break;
                }
                other => anyhow::bail!("Unexpected packet during position response: {other:#04x}"),
            }
        }
        assert!(
            position_chunks > 0 && position_chunks <= 25,
            "Position handler should drain at most 25 chunks (got {position_chunks})"
        );

        client.send_client_tick_end().await?;

        let response = loop {
            let pkt =
                tokio::time::timeout(tokio::time::Duration::from_secs(5), client.read_packet())
                    .await
                    .map_err(|_| {
                        anyhow::anyhow!("Timed out waiting for chunk response after tick end")
                    })?
                    .map_err(|e| anyhow::anyhow!("Failed to read packet after tick end: {e}"))?;
            if pkt.id != play_cb::KEEP_ALIVE {
                break pkt;
            }
        };

        assert_eq!(
            response.id,
            play_cb::CHUNK_BATCH_START,
            "Expected chunk batch start after client tick end"
        );

        let mut chunk_count = 0;
        loop {
            let packet = client.read_packet().await?;
            if packet.id == play_cb::LEVEL_CHUNK_WITH_LIGHT {
                chunk_count += 1;
            } else if packet.id == play_cb::CHUNK_BATCH_FINISHED {
                break;
            } else if packet.id == play_cb::KEEP_ALIVE {
                continue;
            } else {
                anyhow::bail!("Unexpected packet during chunk batch: {:#04x}", packet.id);
            }
        }

        assert!(
            chunk_count > 0,
            "Client Tick End should have drained pending chunks"
        );

        Ok(())
    })
    .await;
}
#[tokio::test]
#[serial]
async fn test_configuration_timeout() {
    retry_test("test_configuration_timeout", 3, || async {
        let server = TestServer::spawn_with_env(&[("RUSTMC_NON_PLAY_TIMEOUT", "3")]).await?;
        let mut client = TestClient::connect(server.port()).await?;

        client.send_handshake(775, 2).await?;
        let uuid = Uuid::new_v4();
        client.send_login_start("TimeoutTest", uuid).await?;

        let compression = client.read_packet().await?;
        assert_eq!(compression.id, login_cb::SET_COMPRESSION);
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

        Ok(())
    })
    .await;
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
#[serial]
async fn test_custom_gameplay_configuration() {
    use rustmc_server::protocol::types::VarInt;
    use std::fs::File;
    use std::io::Write;

    retry_test("test_custom_gameplay_configuration", 3, || async {
        let config_dir = std::env::temp_dir();
        let config_path = config_dir.join(format!("test_config_{}.yaml", uuid::Uuid::new_v4()));
        let mut config_file =
            File::create(&config_path).expect("Failed to create temp config file");

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
        config_file
            .write_all(yaml_content.as_bytes())
            .expect("Failed to write temp config");

        let config_path_str = config_path.to_string_lossy().into_owned();
        let server = TestServer::spawn_with_env(&[("RUSTMC_CONFIG", &config_path_str)]).await?;

        let mut client = TestClient::connect(server.port()).await?;

        client.send_handshake(775, 1).await?;
        client.send_status_request().await?;

        let response = client.read_packet().await?;
        assert_eq!(response.id, status_cb::STATUS_RESPONSE);

        let mut cursor = Cursor::new(&response.data);
        let json_str =
            common::client::read_string(&mut cursor).expect("Failed to read JSON string");
        let json: serde_json::Value =
            serde_json::from_str(&json_str).expect("Failed to parse JSON");

        assert_eq!(
            json["description"]["text"].as_str().unwrap(),
            "Configured Test MOTD"
        );
        assert_eq!(json["players"]["max"].as_i64().unwrap(), 77);

        drop(client);

        let mut client = TestClient::connect(server.port()).await?;

        client.send_handshake(775, 2).await?;
        client
            .send_login_start("TestConfigPlayer", Uuid::new_v4())
            .await?;

        let comp_packet = client.read_packet().await?;
        assert_eq!(comp_packet.id, login_cb::SET_COMPRESSION);
        let mut comp_cursor = Cursor::new(&comp_packet.data);
        let threshold = VarInt::read(&mut comp_cursor).unwrap().0;
        client.enable_compression(threshold);

        let success = client.read_packet().await?;
        assert_eq!(success.id, login_cb::LOGIN_SUCCESS);

        client.send_login_acknowledged().await?;

        let packs = client.read_packet().await?;
        assert_eq!(packs.id, config_cb::KNOWN_PACKS);

        client.send_known_packs_response().await?;

        loop {
            let packet = client.read_packet().await?;
            if packet.id == config_cb::FINISH_CONFIGURATION {
                break;
            }
        }

        client.send_acknowledge_finish_configuration().await?;

        let join_game = client.read_packet().await?;
        assert_eq!(join_game.id, play_cb::LOGIN_PLAY);

        let mut play_cursor = Cursor::new(&join_game.data);

        let mut entity_id_bytes = [0u8; 4];
        play_cursor.read_exact(&mut entity_id_bytes).unwrap();

        let mut hardcore_bytes = [0u8; 1];
        play_cursor.read_exact(&mut hardcore_bytes).unwrap();
        let is_hardcore = hardcore_bytes[0] != 0;
        assert!(is_hardcore, "Hardcore should be true");

        let _dim_count = VarInt::read(&mut play_cursor).unwrap().0;
        let _dim_name = common::client::read_string(&mut play_cursor).unwrap();

        let max_players_decoded = VarInt::read(&mut play_cursor).unwrap().0;
        assert_eq!(max_players_decoded, 77);

        let view_dist = VarInt::read(&mut play_cursor).unwrap().0;
        assert_eq!(view_dist, 6);

        let sim_dist = VarInt::read(&mut play_cursor).unwrap().0;
        assert_eq!(sim_dist, 5);

        let mut flags = [0u8; 3];
        play_cursor.read_exact(&mut flags).unwrap();

        let _dim_type = VarInt::read(&mut play_cursor).unwrap().0;
        let _dim_name_2 = common::client::read_string(&mut play_cursor).unwrap();

        let mut seed = [0u8; 8];
        play_cursor.read_exact(&mut seed).unwrap();

        let mut gm = [0u8; 1];
        play_cursor.read_exact(&mut gm).unwrap();
        let game_mode_decoded = gm[0];
        assert_eq!(game_mode_decoded, 0, "Game mode should be survival (0)");

        let player_info = client.read_packet().await?;
        assert_eq!(player_info.id, play_cb::PLAYER_INFO_UPDATE);

        let _ = std::fs::remove_file(&config_path);

        Ok(())
    })
    .await;
}
