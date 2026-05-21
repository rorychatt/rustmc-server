mod common;

use common::{
    retry_test, try_complete_login_flow, try_complete_login_flow_with_uuid,
    try_drain_initial_play_packets, TestClient, TestServer,
};
use rustmc_server::protocol::packet_ids;
use uuid::Uuid;

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

        try_complete_login_flow_with_uuid(&mut client, "TransferTest", op_uuid).await?;
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
        try_complete_login_flow(&mut client_b, "TransferTest").await?;

        Ok(())
    })
    .await;
}

#[tokio::test]
async fn test_transfer_denied_without_permission() {
    retry_test("test_transfer_denied_without_permission", 3, || async {
        let server = TestServer::spawn().await?;

        let mut client = TestClient::connect(server.port()).await?;

        try_complete_login_flow(&mut client, "TransferTest").await?;
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

        try_complete_login_flow_with_uuid(&mut client, "TransferTest", op_uuid).await?;
        try_drain_initial_play_packets(&mut client).await?;

        client.send_chat_command("transfer localhost").await?;

        client.send_player_position(0.0, 64.0, 0.0, true).await?;

        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        client.send_player_position(1.0, 64.0, 1.0, true).await?;

        Ok(())
    })
    .await;
}
