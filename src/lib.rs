use axum::{extract::Path, extract::State as AxumState, Json};
use serde::{Deserialize, Serialize};
// use uuid::Uuid; // already imported above if needed

/// Table state returned to clients.
#[derive(Serialize)]
pub struct TableState {
    pub hands: Vec<crate::Hand>,
    pub allocations: Vec<crate::CardAllocation>,
    pub hand_states: Vec<crate::HandState>,
}

/// Handler for GET /table/:table_id: get full state for a table.
async fn get_table_state(
    AxumState(state): AxumState<AppState>,
    Path(table_id): Path<Uuid>,
) -> Json<TableState> {
    let ds = state.ds.lock().unwrap();
    let hands = ds
        .hands
        .iter()
        .filter(|h| h.dealer == table_id)
        .cloned()
        .collect();
    let allocations = ds
        .allocations
        .iter()
        .filter(|a| a.dealer == table_id)
        .cloned()
        .collect();
    let hand_states = ds
        .hand_states
        .iter()
        .filter(|hs| {
            let hand_id = hs.0;
            ds.hands
                .iter()
                .any(|h| h.id == hand_id && h.dealer == table_id)
        })
        .cloned()
        .collect();
    Json(TableState {
        hands,
        allocations,
        hand_states,
    })
}

/// Message sent by clients to submit an action for a hand.
#[derive(Serialize, Deserialize)]
pub struct HandActionMsg {
    pub hand_id: Uuid,
    pub action: ActionMsg,
}

/// Supported player actions.
#[derive(Serialize, Deserialize)]
pub enum ActionMsg {
    Hit,
    Hold,
}

/// Handler for POST /action: submit a player action.
async fn submit_action(
    AxumState(state): AxumState<AppState>,
    Json(msg): Json<HandActionMsg>,
) -> &'static str {
    let action = match msg.action {
        ActionMsg::Hit => crate::Action::Hit,
        ActionMsg::Hold => crate::Action::Hold,
    };
    let hand_action = (msg.hand_id, action);
    if let Ok(mut queue) = state.actions.lock() {
        queue.push(hand_action);
    }
    "Action received"
}

/// Library for blackjack_rs: core logic, types, and Axum app construction.
// --- External Crates ---
use log::trace;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use uuid::Uuid;

mod data_source;
mod types;
mod utils;

// --- Public API ---
// Only re-export if not defined in this file, and only if needed externally
pub use data_source::DataSource;
pub use types::{
    Action, Card, CardAllocation, CardValue, Deck, Hand, HandAction, HandState, Outcome, State,
    Suit,
};
pub use utils::hand_value;

// Export Axum handlers for use in main and tests
// (do not re-export here to avoid duplicate symbol errors)

// --- Axum App Construction (for main and tests) ---
use axum::{
    routing::{get, post},
    Router,
};

// --- Backend Processing ---

/// Start the backend processing thread for user actions and game state.
pub fn start_backend(
    actions: Arc<Mutex<Vec<crate::types::HandAction>>>,
    ds: crate::data_source::DataSource,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        loop {
            // @todo: introduce a time to batch & throttle the calls? Rather than do them one at a time?
            // ...existing code...
            // @todo: if there isnt an action for a particular user that should be acting then we should
            //        make them hold instead.

            // Grab the mutex lock for the data source.
            let mut to_process = Vec::new();
            if let Ok(mut actions) = actions.try_lock() {
                let pivot = actions.partition_point(|a| ds.active_hands.contains(&a.0));
                to_process = actions[pivot..].to_vec();
                actions.truncate(pivot);
            }

            let _ = process_user_actions(&to_process, &ds.hands, &ds.allocations, &ds.decks);

            // maps hand_states to hand_outcome
            //let hand_outcomes = update_hand_outcomes(&hand_states, &ds.allocations, &ds.decks);
        }
    })
}

/// Process user actions and update allocations and hand states.
fn process_user_actions(
    actions: &[crate::types::HandAction],
    hands: &[crate::types::Hand],
    allocations: &[crate::types::CardAllocation],
    decks: &HashMap<Uuid, crate::types::Deck>,
) -> (
    Vec<crate::types::CardAllocation>,
    Vec<crate::types::HandState>,
) {
    let new_allocations = actions
        .iter()
        .filter(|(_, action)| matches!(action, crate::types::Action::Hit))
        //@todo: What should we do here if we cant find the hand?  We currently dont
        // log this or anything, it just silently dies.
        .filter_map(|(hand_id, _)| hands.iter().find(|hand| hand.id == *hand_id))
        .map(|hand| {
            let card_idx = allocations
                .iter()
                .filter(|a| a.dealer == hand.dealer)
                .count();
            trace!("Adding card allocation: {},{}", hand.id, card_idx);
            crate::types::CardAllocation {
                card_idx,
                dealer: hand.dealer,
                hand: hand.id,
            }
        })
        .collect::<Vec<_>>();

    // Check for updates to the hand states.
    let updated_hands = allocations
        .iter()
        .filter_map(|ca| hands.iter().find(|&h| h.id == ca.hand))
        .cloned()
        .collect::<Vec<_>>();

    //@todo: Hmmm, this doesnt work because I havent added the new allocations into this list.
    //This needs to be incremental and pass new_allocations instead
    // Check if any of the new hands have busted or hit blackjack.
    let resulting_states = utils::process_hand_states(&updated_hands, allocations, decks);
    //@todo!("need to add a step here to iterate hand states to check for children that need to be added");

    (new_allocations, resulting_states)
}

/// Shared application state for Axum handlers.
#[derive(Clone)]
struct AppState {
    pub actions: Arc<Mutex<Vec<crate::types::HandAction>>>,
    pub ds: Arc<Mutex<crate::data_source::DataSource>>,
}

/// Helper to create the Axum app and shared state (useful for tests and main).
pub fn app_and_state() -> Router<()> {
    let actions = Arc::new(Mutex::new(Vec::new()));
    let ds = Arc::new(Mutex::new(crate::data_source::DataSource::default()));

    // Start backend processing
    let actions_clone = actions.clone();
    let ds_lock = ds.lock().unwrap().clone();
    start_backend(actions_clone, ds_lock);

    let state = AppState {
        actions: actions,
        ds: ds,
    };

    // Build Axum app
    let app = Router::new()
        .route("/action", post(submit_action))
        .route("/table/:table_id", get(get_table_state))
        .with_state(state);

    app
}
