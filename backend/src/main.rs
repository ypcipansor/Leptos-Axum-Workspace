use backend::{app, init_db};
use std::env;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5432/postgres".to_string());

    println!("Connecting to database and running migrations...");
    let pool = init_db(&database_url).await;
    println!("Database connected and initialized successfully.");

    let listener = TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("failed to bind backend server to 0.0.0.0:3000");

    println!(
        "backend listening on http://{}",
        listener.local_addr().expect("listener address unavailable")
    );

    axum::serve(listener, app(pool))
        .await
        .expect("backend server failed");
}
