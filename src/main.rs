//! Blackjack server main entry point using Axum.
/// Exposes HTTP endpoints for submitting actions and retrieving table state.
// --- External Crates ---
use std::net::SocketAddr;
use tokio::net::TcpListener;

/// Main entry point: sets up state, backend, and Axum server.
#[tokio::main]
async fn main() {
    let app = blackjack::app_and_state();
    let addr = SocketAddr::from(([127, 0, 0, 1], 4000));
    println!("Axum server listening on {}", addr);
    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}
