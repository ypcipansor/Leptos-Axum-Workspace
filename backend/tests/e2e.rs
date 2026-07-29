use backend::app;
use shared_lib::HealthStatus;
use tokio::net::TcpListener;

#[tokio::test]
async fn health_endpoint_returns_ok_status() {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("failed to bind ephemeral test port");
    let address = listener
        .local_addr()
        .expect("failed to read test listener address");

    let server = tokio::spawn(async move {
        axum::serve(listener, app())
            .await
            .expect("test server failed while serving app");
    });

    let response = reqwest::get(format!("http://{address}/api/health"))
        .await
        .expect("request to /api/health failed");

    assert_eq!(response.status(), reqwest::StatusCode::OK);

    let body: HealthStatus = response
        .json()
        .await
        .expect("failed to deserialize /api/health response body");

    assert_eq!(body, HealthStatus::ok("backend"));

    server.abort();
    let _ = server.await;
}
