mod common;

use common::{retry_test, TestClient, TestServer};
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
    retry_test("test_transfer_sends_cookie_token", 3, || async {
        let op_uuid = Uuid::parse_str(OP_UUID).unwrap();
        let ops = ops_config(OP_UUID);

        let server_a = TestServer::spawn_with_env_and_ops_config(
            &[("RUSTMC_TRANSFER_SECRET", "integration-test-secret")],
            Some(&ops),
        )
        .await?;

        let server_b = TestServer::spawn_with_env_and_ops_config(
            &[("RUSTMC_TRANSFER_SECRET", "integration-test-secret")],
            Some(&ops),
        )
        .await?;

        let mut client = TestClient::connect(server_a.port()).await?;

        try_complete_login_flow_with_uuid(&mut client, op_uuid).await?;
        try_drain_initial_play_packets(&mut client).await?;

        let target_host = "127.0.0.1";
        let target_port = server_b.port() as i32;
        client
            .send_chat_command(&format!("transfer {} {}", target_host, target_port))
            .await?;

        let cookie_packet = client.read_packet().await?;
        assert_eq!(
            cookie_packet.id,
            play_cb::STORE_COOKIE,
            "Expected store cookie packet before transfer"
        );

        let transfer_packet = client.read_packet().await?;
        assert_eq!(
            transfer_packet.id,
            play_cb::TRANSFER,
            "Expected transfer packet"
        );

        Ok(())
    })
    .await;
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
    retry_test("test_transfer_without_secret_skips_token", 3, || async {
        let op_uuid = Uuid::parse_str(OP_UUID).unwrap();
        let ops = ops_config(OP_UUID);

        let server_a = TestServer::spawn_with_ops(Some(&ops)).await?;
        let server_b = TestServer::spawn_with_ops(Some(&ops)).await?;

        let mut client = TestClient::connect(server_a.port()).await?;

        try_complete_login_flow_with_uuid(&mut client, op_uuid).await?;
        try_drain_initial_play_packets(&mut client).await?;

        let target_host = "127.0.0.1";
        let target_port = server_b.port() as i32;
        client
            .send_chat_command(&format!("transfer {} {}", target_host, target_port))
            .await?;

        let packet = client.read_packet().await?;
        assert_eq!(
            packet.id,
            play_cb::TRANSFER,
            "Expected transfer packet directly without secret"
        );

        Ok(())
    })
    .await;
}

async fn try_complete_login_flow_with_uuid(
    client: &mut TestClient,
    uuid: Uuid,
) -> anyhow::Result<()> {
    client.send_handshake(775, 2).await?;
    client.send_login_start("TransferAuthTest", uuid).await?;

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

async fn try_drain_initial_play_packets(client: &mut TestClient) -> anyhow::Result<()> {
    loop {
        let packet = client.read_packet().await?;
        if packet.id == play_cb::CHUNK_BATCH_FINISHED {
            break;
        }
    }
    Ok(())
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

    let _compression = client
        .read_packet()
        .await
        .expect("Failed to read compression");
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

    client
        .send_acknowledge_finish_configuration()
        .await
        .expect("Failed to send acknowledge finish configuration");

    let join_game = client
        .read_packet()
        .await
        .expect("Failed to read join game");
    assert_eq!(
        join_game.id,
        play_cb::LOGIN_PLAY,
        "Expected join game packet"
    );

    let cookie_request = client
        .read_packet()
        .await
        .expect("Failed to read cookie request");
    assert_eq!(
        cookie_request.id,
        play_cb::COOKIE_REQUEST,
        "Expected cookie request packet after join game"
    );
}
