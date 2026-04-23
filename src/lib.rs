// --- Module declarations ---
mod data_source;
pub mod types;
pub mod utils;

// --- External Crates and Prelude Imports ---
use axum::{
    extract::{Json as AxumJson, Path, State as AxumState},
    routing::{get, post},
    Json, Router,
};
use log::trace;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use uuid::Uuid;

// --- Type Definitions ---

#[derive(Clone)]
struct AppState {
    pub actions: Arc<Mutex<Vec<crate::types::HandAction>>>,
    pub ds: Arc<Mutex<crate::data_source::DataSource>>,
}

#[derive(Serialize)]
pub struct TableState {
    pub hands: Vec<crate::types::Hand>,
    pub allocations: Vec<crate::types::CardAllocation>,
    pub hand_states: Vec<crate::types::HandState>,
}

#[derive(Serialize, Deserialize)]
pub struct HandActionMsg {
    pub hand_id: Uuid,
    pub action: ActionMsg,
}

#[derive(Serialize, Deserialize)]
pub enum ActionMsg {
    Hit,
    Hold,
}

#[derive(Deserialize)]
pub struct JoinTableRequest {
    pub player_id: Uuid,
}

#[derive(Serialize)]
pub struct JoinTableResponse {
    pub hand_id: Option<Uuid>,
    pub already_seated: bool,
}

#[derive(Deserialize)]
pub struct LeaveTableRequest {
    pub player_id: Uuid,
}

#[derive(Serialize)]
pub struct LeaveTableResponse {
    pub hand_id: Option<Uuid>,
    pub was_active: bool,
    pub left: bool,
}

// --- Handler Functions ---

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

/// Handler for POST /action: submit a player action.
async fn submit_action(
    AxumState(state): AxumState<AppState>,
    Json(msg): Json<HandActionMsg>,
) -> &'static str {
    let action = match msg.action {
        ActionMsg::Hit => crate::types::Action::Hit,
        ActionMsg::Hold => crate::types::Action::Hold,
    };
    let hand_action = (msg.hand_id, action);
    if let Ok(mut queue) = state.actions.lock() {
        queue.push(hand_action);
    }
    "Action received"
}

/// Handler for POST /table/:table_id/join: seat a player at a table if not already seated.
async fn join_table(
    AxumState(state): AxumState<AppState>,
    Path(table_id): Path<Uuid>,
    AxumJson(req): AxumJson<JoinTableRequest>,
) -> Json<JoinTableResponse> {
    let mut ds = state.ds.lock().unwrap();
    let result = ds.add_player(table_id, req.player_id);
    Json(JoinTableResponse {
        hand_id: result,
        already_seated: result.is_none(),
    })
}

/// Handler for POST /table/:table_id/leave: remove a player from a table, submit Hold if midgame.
async fn leave_table(
    AxumState(state): AxumState<AppState>,
    Path(table_id): Path<Uuid>,
    AxumJson(req): AxumJson<LeaveTableRequest>,
) -> Json<LeaveTableResponse> {
    let mut ds = state.ds.lock().unwrap();
    // Find the hand for this player at this table
    let hand = ds
        .hands
        .iter()
        .find(|h| h.dealer == table_id && h.player == req.player_id)
        .cloned();
    let mut was_active = false;
    let mut left = false;
    let hand_id = hand.as_ref().map(|h| h.id);
    if let Some(h) = hand {
        // If hand is still active, submit Hold action
        if ds.active_hands.contains(&h.id) {
            was_active = true;
            // Remove from active_hands and treat as Hold
            ds.active_hands.retain(|&hid| hid != h.id);
            // Optionally, you could push a Hold action to the action queue here if needed
        }
        // Remove the hand from the table
        ds.remove_player(table_id, req.player_id);
        left = true;
    }
    Json(LeaveTableResponse {
        hand_id,
        was_active,
        left,
    })
}

// --- Backend Processing ---

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

// --- App Construction ---

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
    Router::new()
        .route("/action", post(submit_action))
        .route("/table/:table_id", get(get_table_state))
        .route("/table/:table_id/join", post(join_table))
        .route("/table/:table_id/leave", post(leave_table))
        .with_state(state)
}
