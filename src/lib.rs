// --- Module declarations ---
pub mod data_source;
pub mod types;
pub mod utils;

// --- External Crates and Prelude Imports ---
use crate::data_source::{
    effective_countdown_secs, effective_resolving_secs, GameState, MAX_PLAYERS_PER_TABLE,
};
use crate::types::{Action, Outcome, State};
use crate::utils::hand_value;
use axum::{
    extract::{Json as AxumJson, Path, State as AxumState},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use uuid::Uuid;

// --- Type Definitions ---

#[derive(Clone)]
struct AppState {
    pub actions: Arc<Mutex<Vec<crate::types::HandAction>>>,
    pub ds: Arc<Mutex<crate::data_source::DataSource>>,
}

#[derive(Serialize)]
pub struct HandInfo {
    pub hand: crate::types::Hand,
    pub cards: Vec<crate::types::Card>,
    pub state: Option<String>,
}

#[derive(Serialize)]
pub struct TableState {
    pub game_state: String,
    pub seconds_remaining: Option<u64>,
    pub hands: Vec<HandInfo>,
    pub outcomes: Vec<(Uuid, String)>,
    pub bets: Vec<crate::types::Bet>,
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
    pub rejected: bool,
    pub reason: Option<String>,
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

#[derive(Serialize)]
struct CreatePlayerResponse {
    player_id: Uuid,
    balance: u32,
}

#[derive(Serialize)]
struct TableInfo {
    id: Uuid,
    state: String,
    player_count: usize,
    seconds_remaining: Option<u64>,
}

#[derive(Deserialize)]
struct PlaceBetRequest {
    player_id: Uuid,
    amount: u32,
}

#[derive(Serialize)]
struct PlaceBetResponse {
    success: bool,
    balance_remaining: Option<u32>,
    error: Option<String>,
}

// --- Handler Functions ---

/// Handler for GET /table/:table_id: get full state for a table.
async fn get_table_state(
    AxumState(state): AxumState<AppState>,
    Path(table_id): Path<Uuid>,
) -> Json<TableState> {
    let ds = state.ds.lock().unwrap();

    // Game state string + seconds_remaining
    let (game_state_str, seconds_remaining) = match ds.get_game_states().get(&table_id) {
        Some(GameState::Waiting) => ("waiting".to_string(), None),
        Some(GameState::Countdown { started_at }) => {
            let elapsed = started_at.elapsed().as_secs();
            let remaining = effective_countdown_secs().saturating_sub(elapsed);
            ("countdown".to_string(), Some(remaining))
        }
        Some(GameState::Active) => ("active".to_string(), None),
        Some(GameState::Resolving { .. }) => ("resolving".to_string(), None),
        None => ("unknown".to_string(), None),
    };

    // Build HandInfo for each hand at this table
    let hands: Vec<HandInfo> = ds
        .hands
        .iter()
        .filter(|h| h.dealer == table_id)
        .map(|h| {
            let cards = ds.get_hand(h);
            let state_str = ds
                .hand_states
                .iter()
                .find(|hs| hs.0 == h.id)
                .map(|hs| match &hs.2 {
                    State::BlackJack => "blackjack".to_string(),
                    State::Bust(v) => format!("bust:{}", v),
                    State::Holding(v) => format!("holding:{}", v),
                    State::Active => "active".to_string(),
                });
            HandInfo {
                hand: h.clone(),
                cards,
                state: state_str,
            }
        })
        .collect();

    // Outcomes for this table
    let table_hand_ids: std::collections::HashSet<Uuid> = ds
        .hands
        .iter()
        .filter(|h| h.dealer == table_id)
        .map(|h| h.id)
        .collect();
    let outcomes: Vec<(Uuid, String)> = ds
        .outcomes
        .iter()
        .filter(|(hid, _)| table_hand_ids.contains(hid))
        .map(|(hid, o)| {
            let s = match o {
                Outcome::Won(v) => format!("won:{}", v),
                Outcome::Lost(v) => format!("lost:{}", v),
                Outcome::Push => "push".to_string(),
            };
            (*hid, s)
        })
        .collect();

    // Bets for this table
    let bets: Vec<crate::types::Bet> = ds
        .bets
        .iter()
        .filter(|b| b.dealer_id == table_id)
        .cloned()
        .collect();

    Json(TableState {
        game_state: game_state_str,
        seconds_remaining,
        hands,
        outcomes,
        bets,
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
) -> (axum::http::StatusCode, Json<JoinTableResponse>) {
    use axum::http::StatusCode;
    let mut ds = state.ds.lock().unwrap();

    // Reject mid-game joins
    let state_snapshot = ds.get_game_states().get(&table_id).cloned();
    match state_snapshot {
        Some(GameState::Active) | Some(GameState::Resolving { .. }) => {
            return (
                StatusCode::CONFLICT,
                Json(JoinTableResponse {
                    hand_id: None,
                    already_seated: false,
                    rejected: true,
                    reason: Some("game in progress".to_string()),
                }),
            );
        }
        _ => {}
    }

    let was_empty = ds.player_count(table_id) == 0;
    let result = ds.add_player(table_id, req.player_id);

    // First player triggers countdown
    if result.is_some() && was_empty {
        ds.transition_to_countdown(table_id);
    }

    (
        StatusCode::OK,
        Json(JoinTableResponse {
            hand_id: result,
            already_seated: result.is_none(),
            rejected: false,
            reason: None,
        }),
    )
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

/// Handler for POST /player: register a new player.
async fn create_player(AxumState(state): AxumState<AppState>) -> Json<CreatePlayerResponse> {
    let mut ds = state.ds.lock().unwrap();
    let player_id = ds.register_player();
    let balance = ds.players[&player_id].balance;
    Json(CreatePlayerResponse { player_id, balance })
}

/// Handler for GET /player/:id: get player info.
async fn get_player(
    AxumState(state): AxumState<AppState>,
    Path(player_id): Path<Uuid>,
) -> Result<Json<CreatePlayerResponse>, axum::http::StatusCode> {
    let ds = state.ds.lock().unwrap();
    match ds.players.get(&player_id) {
        Some(p) => Ok(Json(CreatePlayerResponse {
            player_id: p.id,
            balance: p.balance,
        })),
        None => Err(axum::http::StatusCode::NOT_FOUND),
    }
}

/// Handler for GET /tables: list all tables.
async fn get_tables(AxumState(state): AxumState<AppState>) -> Json<Vec<TableInfo>> {
    let ds = state.ds.lock().unwrap();
    let tables: Vec<TableInfo> = ds
        .get_game_states()
        .iter()
        .map(|(id, gs)| {
            let (state_str, seconds_remaining) = match gs {
                GameState::Waiting => ("waiting".to_string(), None),
                GameState::Countdown { started_at } => {
                    let remaining = crate::utils::countdown_seconds_remaining(started_at);
                    ("countdown".to_string(), Some(remaining))
                }
                GameState::Active => ("active".to_string(), None),
                GameState::Resolving { .. } => ("resolving".to_string(), None),
            };
            TableInfo {
                id: *id,
                state: state_str,
                player_count: ds.player_count(*id),
                seconds_remaining,
            }
        })
        .collect();
    Json(tables)
}

/// Handler for POST /table/:id/bet: place a bet.
async fn place_bet_handler(
    AxumState(state): AxumState<AppState>,
    Path(table_id): Path<Uuid>,
    AxumJson(req): AxumJson<PlaceBetRequest>,
) -> (axum::http::StatusCode, Json<PlaceBetResponse>) {
    let mut ds = state.ds.lock().unwrap();
    match ds.place_bet(req.player_id, table_id, req.amount) {
        Ok(()) => {
            let balance = ds.players.get(&req.player_id).map(|p| p.balance);
            (
                axum::http::StatusCode::OK,
                Json(PlaceBetResponse {
                    success: true,
                    balance_remaining: balance,
                    error: None,
                }),
            )
        }
        Err(e) => (
            axum::http::StatusCode::BAD_REQUEST,
            Json(PlaceBetResponse {
                success: false,
                balance_remaining: None,
                error: Some(e.to_string()),
            }),
        ),
    }
}

// --- Backend Processing ---

pub fn start_backend(
    actions: Arc<Mutex<Vec<crate::types::HandAction>>>,
    ds: Arc<Mutex<crate::data_source::DataSource>>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        loop {
            // ── Step 1: Drain active player actions from the queue ──────────────────
            let mut to_process: Vec<crate::types::HandAction> = Vec::new();
            if let Ok(mut actions_guard) = actions.try_lock() {
                let active_hands_snapshot = {
                    let ds_guard = ds.lock().unwrap();
                    ds_guard.active_hands.clone()
                };
                let (active, remaining): (Vec<_>, Vec<_>) = actions_guard
                    .drain(..)
                    .partition(|a| active_hands_snapshot.contains(&a.0));
                *actions_guard = remaining;
                to_process = active;
            }

            // ── Step 2: Process player actions ──────────────────────────────────────
            // Hold the ds lock for the rest of the tick (Steps 2–6).
            {
                let mut ds_guard = ds.lock().unwrap();

                for (hand_id, action) in &to_process {
                    let hand = match ds_guard.hands.iter().find(|h| h.id == *hand_id).cloned() {
                        Some(h) => h,
                        None => continue,
                    };

                    match action {
                        Action::Hit => {
                            // Allocate 1 card
                            let new_allocs =
                                ds_guard.allocate_cards(std::slice::from_ref(&hand), 1);
                            ds_guard.allocations.extend(new_allocs);
                            // Compute new hand state
                            let cards = ds_guard.get_hand(&hand);
                            let v = hand_value(&cards);
                            let new_state = if v > 21 {
                                Some(State::Bust(v))
                            } else if v == 21 {
                                Some(State::BlackJack)
                            } else {
                                None // still active — no hand_state entry needed
                            };
                            if let Some(s) = new_state {
                                if let Some(existing) =
                                    ds_guard.hand_states.iter_mut().find(|hs| hs.0 == *hand_id)
                                {
                                    *existing = (*hand_id, hand.dealer, s);
                                } else {
                                    ds_guard.hand_states.push((*hand_id, hand.dealer, s));
                                }
                            }
                        }
                        Action::Hold => {
                            let cards = ds_guard.get_hand(&hand);
                            let v = hand_value(&cards);
                            let s = State::Holding(v);
                            if let Some(existing) =
                                ds_guard.hand_states.iter_mut().find(|hs| hs.0 == *hand_id)
                            {
                                *existing = (*hand_id, hand.dealer, s);
                            } else {
                                ds_guard.hand_states.push((*hand_id, hand.dealer, s));
                            }
                        }
                    }

                    // Advance to the next hand after each action
                    ds_guard.resolve_turn();
                }

                // ── Step 3: Dealer AI ────────────────────────────────────────────────
                // For each Active table, if the dealer's hand is the only active hand, run
                // dealer auto-play.
                //
                // The dealer's hand_id == game_id (they are the same UUID).
                // So: if active_hands contains game_id, it's the dealer's turn.
                let game_ids: Vec<Uuid> = ds_guard.get_game_states().keys().cloned().collect();
                for game_id in &game_ids {
                    let is_active = matches!(
                        ds_guard.get_game_states().get(game_id),
                        Some(GameState::Active)
                    );
                    if !is_active {
                        continue;
                    }
                    if !ds_guard.active_hands.contains(game_id) {
                        continue; // not the dealer's turn yet
                    }

                    // Dealer auto-play: hit until value >= 17
                    let dealer_hand =
                        match ds_guard.hands.iter().find(|h| h.id == *game_id).cloned() {
                            Some(h) => h,
                            None => continue,
                        };

                    loop {
                        let cards = ds_guard.get_hand(&dealer_hand);
                        let v = hand_value(&cards);
                        if v < 17 {
                            // Dealer must hit
                            let new_allocs =
                                ds_guard.allocate_cards(std::slice::from_ref(&dealer_hand), 1);
                            ds_guard.allocations.extend(new_allocs);
                        } else {
                            // Dealer stands or busts
                            let final_state = if v > 21 {
                                State::Bust(v)
                            } else {
                                State::Holding(v)
                            };
                            if let Some(existing) =
                                ds_guard.hand_states.iter_mut().find(|hs| hs.0 == *game_id)
                            {
                                *existing = (*game_id, *game_id, final_state);
                            } else {
                                ds_guard.hand_states.push((*game_id, *game_id, final_state));
                            }
                            // Resolve outcomes + empty active_hands
                            ds_guard.resolve_turn();
                            // Transition to Resolving
                            ds_guard.transition_to_resolving(*game_id);
                            break;
                        }
                    }
                }

                // ── Step 4: Countdown check ──────────────────────────────────────────
                let game_states_snap: Vec<(Uuid, GameState)> = ds_guard
                    .get_game_states()
                    .iter()
                    .map(|(k, v)| (*k, v.clone()))
                    .collect();
                for (game_id, state) in &game_states_snap {
                    if let GameState::Countdown { started_at } = state {
                        let elapsed = started_at.elapsed().as_secs();
                        if elapsed >= effective_countdown_secs()
                            || ds_guard.player_count(*game_id) >= MAX_PLAYERS_PER_TABLE
                        {
                            ds_guard.start_game(*game_id);
                        }
                    }
                }

                // ── Step 5: Resolving check ──────────────────────────────────────────
                for (game_id, state) in &game_states_snap {
                    if let GameState::Resolving { started_at } = state {
                        if started_at.elapsed().as_secs() >= effective_resolving_secs() {
                            ds_guard.apply_betting_outcomes();
                            ds_guard.reset_game(*game_id);
                        }
                    }
                }
            } // ds_guard released

            // ── Step 6: Sleep ────────────────────────────────────────────────────────
            thread::sleep(Duration::from_millis(100));
        }
    })
}

// --- App Construction ---

#[cfg(feature = "harness")]
async fn debug_set_deck(
    AxumState(state): AxumState<AppState>,
    Path(table_id): Path<Uuid>,
    AxumJson(deck): AxumJson<crate::types::Deck>,
) -> axum::http::StatusCode {
    let mut ds = state.ds.lock().unwrap();
    ds.set_deck(table_id, deck);
    axum::http::StatusCode::OK
}

pub fn app_and_state() -> Router<()> {
    let actions = Arc::new(Mutex::new(Vec::new()));
    let mut ds_inner = crate::data_source::DataSource::default();

    // Generate tables at startup
    ds_inner.generate_tables(crate::data_source::DEFAULT_TABLE_COUNT);
    let ds = Arc::new(Mutex::new(ds_inner));

    // Start backend processing — pass Arc clone so backend shares the same DataSource.
    let actions_clone = actions.clone();
    start_backend(actions_clone, ds.clone());

    let state = AppState { actions, ds };

    // Build Axum app
    let router = Router::new()
        .route("/action", post(submit_action))
        .route("/table/:table_id", get(get_table_state))
        .route("/table/:table_id/join", post(join_table))
        .route("/table/:table_id/leave", post(leave_table))
        .route("/player", post(create_player))
        .route("/player/:id", get(get_player))
        .route("/tables", get(get_tables))
        .route("/table/:id/bet", post(place_bet_handler));

    #[cfg(feature = "harness")]
    let router = router.route("/debug/set-deck/:table_id", post(debug_set_deck));

    router.with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_create_player() {
        let app = app_and_state();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/player")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["balance"], 1000);
        assert!(json["player_id"].is_string());
    }

    #[tokio::test]
    async fn test_get_player_not_found() {
        let app = app_and_state();
        let random_id = Uuid::new_v4();
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/player/{random_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_get_player_found() {
        let app = app_and_state();
        // Register a player
        let reg_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/player")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(reg_resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let player_id = json["player_id"].as_str().unwrap().to_string();

        // Look them up
        let get_resp = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/player/{player_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(get_resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_get_tables() {
        let app = app_and_state();
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/tables")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let arr = json.as_array().unwrap();
        assert_eq!(arr.len(), 8); // DEFAULT_TABLE_COUNT
        let first = &arr[0];
        assert!(first["id"].is_string());
        assert!(first["state"].is_string());
        assert!(first["player_count"].is_number());
    }

    #[tokio::test]
    async fn test_place_bet_player_not_found() {
        let app = app_and_state();
        // Get a real table ID first
        let tables_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/tables")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(tables_resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let tables: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let table_id = tables[0]["id"].as_str().unwrap().to_string();

        let fake_player = Uuid::new_v4();
        let bet_body = serde_json::json!({ "player_id": fake_player, "amount": 10 });
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/table/{table_id}/bet"))
                    .header("content-type", "application/json")
                    .body(Body::from(bet_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["success"], false);
        assert!(json["error"].is_string());
    }

    // ── join_table new behaviour ──────────────────────────────────────────────

    /// Helper: get the first table ID from the /tables endpoint.
    async fn first_table_id(app: &Router<()>) -> String {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/tables")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let tables: serde_json::Value = serde_json::from_slice(&body).unwrap();
        tables[0]["id"].as_str().unwrap().to_string()
    }

    /// Helper: register a new player and return their UUID string.
    async fn register_player(app: &Router<()>) -> String {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/player")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        json["player_id"].as_str().unwrap().to_string()
    }

    /// Helper: POST /table/:id/join and return the full JSON response + status.
    async fn join(
        app: &Router<()>,
        table_id: &str,
        player_id: &str,
    ) -> (StatusCode, serde_json::Value) {
        let body = serde_json::json!({ "player_id": player_id });
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/table/{table_id}/join"))
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = resp.status();
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        (status, json)
    }

    #[tokio::test]
    async fn test_join_table_response_has_rejected_and_reason_fields() {
        let app = app_and_state();
        let table_id = first_table_id(&app).await;
        let player_id = register_player(&app).await;

        let (status, json) = join(&app, &table_id, &player_id).await;
        assert_eq!(status, StatusCode::OK);
        // New fields must be present
        assert_eq!(json["rejected"], false);
        assert!(json["reason"].is_null());
        // Existing fields still present
        assert!(json["hand_id"].is_string());
        assert_eq!(json["already_seated"], false);
    }

    #[tokio::test]
    async fn test_join_table_first_player_triggers_countdown() {
        let app = app_and_state();
        let table_id = first_table_id(&app).await;
        let player_id = register_player(&app).await;

        // Join as first player
        let (status, json) = join(&app, &table_id, &player_id).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["rejected"], false);

        // Table state should now be "countdown"
        let state_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/table/{table_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let bytes = axum::body::to_bytes(state_resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let state_json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(state_json["game_state"], "countdown");
        assert!(state_json["seconds_remaining"].is_number());
    }

    #[tokio::test]
    async fn test_join_table_rejected_when_active() {
        // Build a DataSource directly to force Active state, then test via the handler
        use crate::data_source::DataSource;
        use std::sync::{Arc, Mutex};

        let mut ds_inner = DataSource::default();
        let table_ids = ds_inner.generate_tables(1);
        let table_id = table_ids[0];

        // Register a player and seat them, then force Active state
        let player_id = ds_inner.register_player();
        ds_inner.add_player(table_id, player_id);
        ds_inner.set_game_state_countdown(table_id);
        ds_inner.place_bet(player_id, table_id, 1).unwrap();
        ds_inner.start_game(table_id); // transitions to Active

        let actions = Arc::new(Mutex::new(Vec::new()));
        let ds = Arc::new(Mutex::new(ds_inner));

        let state = AppState {
            actions: actions.clone(),
            ds: ds.clone(),
        };

        let app = Router::new()
            .route("/table/:table_id/join", post(join_table))
            .with_state(state);

        let new_player = Uuid::new_v4();
        let body = serde_json::json!({ "player_id": new_player });
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/table/{table_id}/join"))
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::CONFLICT);
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["rejected"], true);
        assert_eq!(json["reason"], "game in progress");
        assert!(json["hand_id"].is_null());
        assert_eq!(json["already_seated"], false);
    }

    #[tokio::test]
    async fn test_get_table_state_new_fields() {
        let app = app_and_state();
        let table_id = first_table_id(&app).await;

        let resp = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/table/{table_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

        // New required fields
        assert!(json["game_state"].is_string());
        assert_eq!(json["game_state"], "waiting");
        // seconds_remaining is null for Waiting state
        assert!(json["seconds_remaining"].is_null());
        // hands is an array
        assert!(json["hands"].is_array());
        // outcomes is an array
        assert!(json["outcomes"].is_array());
        // bets is an array
        assert!(json["bets"].is_array());
    }

    #[tokio::test]
    async fn test_get_table_state_hand_info_fields() {
        let app = app_and_state();
        let table_id = first_table_id(&app).await;
        let player_id = register_player(&app).await;

        // Join so there's a player hand
        join(&app, &table_id, &player_id).await;

        let resp = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/table/{table_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

        let hands = json["hands"].as_array().unwrap();
        // At least the dealer self-hand + player hand
        assert!(hands.len() >= 2);

        // Each HandInfo must have hand, cards, state fields
        for h in hands {
            assert!(h["hand"].is_object(), "hand field must be an object");
            assert!(h["cards"].is_array(), "cards field must be an array");
            // state is either null or a string
            assert!(
                h["state"].is_null() || h["state"].is_string(),
                "state must be null or string"
            );
        }
    }
}
