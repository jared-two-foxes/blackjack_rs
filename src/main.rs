use axum::{extract::State, routing::post, Json, Router};
use blackjack::{DataSource, HandAction};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use tokio::net::TcpListener;

#[derive(Serialize, Deserialize)]
struct HandActionMsg {
    hand_id: Uuid,
    action: ActionMsg,
}

#[derive(Serialize, Deserialize)]
enum ActionMsg {
    Hit,
    Hold,
}

#[derive(Clone)]
struct AppState {
    actions: Arc<Mutex<Vec<HandAction>>>,
    ds: Arc<Mutex<DataSource>>,
}

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

#[tokio::main]
async fn main() {
    let actions = Arc::new(Mutex::new(Vec::new()));
    let ds = Arc::new(Mutex::new(DataSource::default()));
    let state = AppState {
        actions: actions.clone(),
        ds: ds.clone(),
    };

    // Start backend processing thread
    let actions_clone = actions.clone();
    let ds_for_thread = ds.lock().unwrap().clone();
    std::thread::spawn(move || {
        blackjack::start_backend(actions_clone, ds_for_thread);
    });

    let app = Router::new()
        .route("/action", post(submit_action))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 4000));
    println!("Axum server listening on {}", addr);
    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}
