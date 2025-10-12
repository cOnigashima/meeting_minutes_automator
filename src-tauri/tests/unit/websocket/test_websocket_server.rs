// Unit Tests for WebSocket Server
// TDD Red Phase: Tests that will drive the implementation

use meeting_minutes_automator_lib::websocket::WebSocketServer;

#[tokio::test]
async fn ut_6_1_1_websocket_server_port_assignment() {
    // Test: Should assign a port in range 9001-9100
    let mut server = WebSocketServer::new();

    let port = server.start().await.expect("Should start successfully");

    assert!(
        port >= 9001 && port <= 9100,
        "Port should be in range 9001-9100, got {}",
        port
    );

    server.stop().await.expect("Should stop successfully");
}

#[tokio::test]
async fn ut_6_1_2_websocket_server_fallback_on_conflict() {
    // Test: Should try next port if first port is occupied
    let mut server1 = WebSocketServer::new();
    let mut server2 = WebSocketServer::new();

    let port1 = server1.start().await.expect("First server should start");
    let port2 = server2.start().await.expect("Second server should start");

    assert_ne!(port1, port2, "Should use different ports");
    assert!(port1 >= 9001 && port1 <= 9100, "Port1 should be in range");
    assert!(port2 >= 9001 && port2 <= 9100, "Port2 should be in range");

    server1.stop().await.expect("Should stop server1");
    server2.stop().await.expect("Should stop server2");
}

#[tokio::test]
async fn ut_6_1_3_websocket_server_restart() {
    // Test: Should be able to start, stop, and start again
    let mut server = WebSocketServer::new();

    let port1 = server.start().await.expect("Should start first time");
    server.stop().await.expect("Should stop");

    let port2 = server.start().await.expect("Should start second time");

    assert!(port1 >= 9001 && port1 <= 9100);
    assert!(port2 >= 9001 && port2 <= 9100);

    server.stop().await.expect("Should stop again");
}
