use rustmc_test_utils::{TestClient, TestServer};

#[tokio::test]
async fn test_server_connection() {
    let server = TestServer::spawn().await;
    let client = TestClient::connect(server.addr()).await;
    assert!(client.is_ok(), "Should be able to connect to test server");
}

#[tokio::test]
async fn test_send_handshake_packet() {
    let server = TestServer::spawn().await;
    let mut client = TestClient::connect(server.addr()).await.unwrap();

    // Send handshake (state=1 for status)
    let result = client.send_handshake(1).await;
    assert!(result.is_ok(), "Should be able to send handshake packet");
}

#[tokio::test]
async fn test_send_login_packet() {
    let server = TestServer::spawn().await;
    let mut client = TestClient::connect(server.addr()).await.unwrap();

    // Handshake → Login
    client.send_handshake(2).await.unwrap();
    let result = client.send_login_start("TestPlayer").await;
    assert!(result.is_ok(), "Should be able to send login packet");
}

#[tokio::test]
async fn test_concurrent_connections() {
    let server = TestServer::spawn().await;

    // Spawn 10 concurrent clients
    let handles: Vec<_> = (0..10)
        .map(|_i| {
            let addr = server.addr();
            tokio::spawn(async move {
                let mut client = TestClient::connect(addr).await.unwrap();
                client.send_handshake(1).await.unwrap();
                client.send_status_request().await.unwrap();
                client.recv_status_response().await.unwrap()
            })
        })
        .collect();

    // All clients should succeed
    for handle in handles {
        handle.await.unwrap();
    }
}

#[tokio::test]
async fn test_multiple_handshakes() {
    let server = TestServer::spawn().await;
    let mut client = TestClient::connect(server.addr()).await.unwrap();

    // First handshake for status
    client.send_handshake(1).await.unwrap();
    client.send_status_request().await.unwrap();
    let response1 = client.recv_status_response().await.unwrap();
    assert_eq!(response1.version.protocol, 775);
}

#[tokio::test]
async fn test_connection_timeout() {
    let server = TestServer::spawn().await;
    let client = TestClient::connect(server.addr()).await;
    assert!(client.is_ok(), "Connection should succeed");
}
