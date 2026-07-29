use backend::{app, init_db};
use std::env;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let database_url = env::var("DATABASE_URL")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "postgresql://postgres:postgres@localhost:5432/postgres".to_string());

    println!("Connecting to database and running migrations...");
    let pool = init_db(&database_url).await;
    println!("Database connected and initialized successfully.");

    let port = env::var("PORT")
        .ok()
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(3000);

    let listener = TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .unwrap_or_else(|_| panic!("failed to bind backend server to 0.0.0.0:{}", port));

    println!(
        "backend listening on http://{}",
        listener.local_addr().expect("listener address unavailable")
    );

    axum::serve(listener, app(pool))
        .await
        .expect("backend server failed");
}
