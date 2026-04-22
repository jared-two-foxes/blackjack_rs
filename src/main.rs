//! Blackjack server main entry point using Axum.
/// Exposes HTTP endpoints for submitting actions and retrieving table state.

// --- External Crates ---
use axum::{extract::State, extract::Path, routing::{post, get}, Json, Router};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use uuid::Uuid;

// --- Internal Crates ---
use blackjack::{DataSource, HandAction};

/// Table state returned to clients.
#[derive(Serialize)]
struct TableState {
    hands: Vec<blackjack::Hand>,
    allocations: Vec<blackjack::CardAllocation>,
    hand_states: Vec<blackjack::HandState>,
}

/// Handler for GET /table/:table_id: get full state for a table.
async fn get_table_state(
    State(state): State<AppState>,
    Path(table_id): Path<Uuid>,
) -> Json<TableState> {
    let ds = state.ds.lock().unwrap();
    let hands = ds.hands.iter().filter(|h| h.dealer == table_id).cloned().collect();
    let allocations = ds.allocations.iter().filter(|a| a.dealer == table_id).cloned().collect();
    let hand_states = ds.hand_states.iter().filter(|hs| {
        let hand_id = hs.0;
        ds.hands.iter().any(|h| h.id == hand_id && h.dealer == table_id)
    }).cloned().collect();
    Json(TableState { hands, allocations, hand_states })
}

/// Message sent by clients to submit an action for a hand.
#[derive(Serialize, Deserialize)]
struct HandActionMsg {
    hand_id: Uuid,
    action: ActionMsg,
}

/// Supported player actions.
#[derive(Serialize, Deserialize)]
enum ActionMsg {
    Hit,
    Hold,
}

/// Shared application state for Axum handlers.
#[derive(Clone)]
struct AppState {
    actions: Arc<Mutex<Vec<HandAction>>>,
    ds: Arc<Mutex<DataSource>>,
}

/// Handler for POST /action: submit a player action.
async fn submit_action(
    State(state): State<AppState>,
    Json(msg): Json<HandActionMsg>,
) -> &'static str {
    let action = match msg.action {
        ActionMsg::Hit => blackjack::Action::Hit,
        ActionMsg::Hold => blackjack::Action::Hold,
    };
    let hand_action = (msg.hand_id, action);
    if let Ok(mut queue) = state.actions.lock() {
        queue.push(hand_action);
    }
    "Action received"
}

/// Main entry point: sets up state, backend, and Axum server.
#[tokio::main]
async fn main() {
    // Shared state
    let actions = Arc::new(Mutex::new(Vec::new()));
    let ds = Arc::new(Mutex::new(DataSource::default()));
    let state = AppState {
        actions: actions.clone(),
        ds: ds.clone(),
    };

    // Start backend processing (spawns its own thread)
    let actions_clone = actions.clone();
    let ds_for_thread = ds.lock().unwrap().clone();
    blackjack::start_backend(actions_clone, ds_for_thread);

    // Build Axum app
    let app = Router::new()
        .route("/action", post(submit_action))
        .route("/table/:table_id", get(get_table_state))
        .with_state(state);

    // Start server
    let addr = SocketAddr::from(([127, 0, 0, 1], 4000));
    println!("Axum server listening on {}", addr);
    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}