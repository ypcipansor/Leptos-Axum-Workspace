use backend::app;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("failed to bind backend server to 0.0.0.0:3000");

    println!(
        "backend listening on http://{}",
        listener.local_addr().expect("listener address unavailable")
    );

    axum::serve(listener, app())
        .await
        .expect("backend server failed");
}
