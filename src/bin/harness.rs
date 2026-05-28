use serde_json::json;
use std::time::Duration;

const COUNTDOWN_SECS: &str = "2";
const RESOLVING_SECS: &str = "1";
const POLL_INTERVAL_MS: u64 = 200;
const POLL_TIMEOUT_SECS: u64 = 10;

#[tokio::main]
async fn main() {
    // Set short timers so scenarios complete quickly
    unsafe {
        std::env::set_var("BLACKJACK_COUNTDOWN_SECS", COUNTDOWN_SECS);
        std::env::set_var("BLACKJACK_RESOLVING_SECS", RESOLVING_SECS);
    }

    let mut passed = 0u32;
    let mut failed = 0u32;

    macro_rules! run {
        ($name:expr, $fut:expr) => {
            print!("  {} ... ", $name);
            match $fut.await {
                Ok(()) => {
                    println!("PASS");
                    passed += 1;
                }
                Err(e) => {
                    println!("FAIL: {}", e);
                    failed += 1;
                }
            }
        };
    }

    println!("blackjack harness");
    run!(
        "scenario 1: happy path (dealer busts, player wins)",
        scenario_happy_path()
    );
    run!(
        "scenario 2: mid-game join rejection",
        scenario_mid_game_join()
    );
    run!(
        "scenario 3: insufficient funds",
        scenario_insufficient_funds()
    );
    run!("scenario 4: push (tied hands)", scenario_push());
    run!("scenario 5: blackjack payout (3:2)", scenario_blackjack());
    run!("scenario 6: multi-player table", scenario_multi_player());
    run!(
        "scenario 7: double down win (dealer busts)",
        scenario_double_down_win()
    );
    run!(
        "scenario 8: double down bust (player busts, dealer holds)",
        scenario_double_down_bust()
    );
    run!(
        "scenario 9: surrender (half-bet refund quirk)",
        scenario_surrender()
    );
    run!(
        "scenario 10: split (two 8s, both hands win)",
        scenario_split()
    );
    run!(
        "scenario 11: multi-round stability (3 rounds, no crash)",
        scenario_multi_round_no_crash()
    );

    println!("\n{passed} passed, {failed} failed");
    if failed > 0 {
        std::process::exit(1);
    }
}

// ── helpers ──────────────────────────────────────────────────────────────────

async fn spawn_server() -> (String, reqwest::Client) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://{}", addr);
    let app = blackjack::app_and_state();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let client = reqwest::Client::new();
    // Brief pause to let the server bind and the backend thread start
    tokio::time::sleep(Duration::from_millis(50)).await;
    (base_url, client)
}

/// GET /tables → pick the first table in "waiting" state.
async fn first_waiting_table(base: &str, client: &reqwest::Client) -> Result<String, String> {
    let tables: serde_json::Value = client
        .get(format!("{base}/tables"))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    tables
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .find(|t| t["state"] == "waiting")
        .and_then(|t| t["id"].as_str().map(str::to_owned))
        .ok_or_else(|| "no waiting table found".to_string())
}

/// POST /player → player_id
async fn register_player(base: &str, client: &reqwest::Client) -> Result<String, String> {
    let resp: serde_json::Value = client
        .post(format!("{base}/player"))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    resp["player_id"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| "missing player_id".to_string())
}

/// POST /table/:id/join → hand_id
async fn join_table(
    base: &str,
    client: &reqwest::Client,
    table_id: &str,
    player_id: &str,
) -> Result<(u16, serde_json::Value), String> {
    let resp = client
        .post(format!("{base}/table/{table_id}/join"))
        .json(&json!({"player_id": player_id}))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = resp.status().as_u16();
    let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    Ok((status, body))
}

/// POST /table/:id/bet
async fn place_bet(
    base: &str,
    client: &reqwest::Client,
    table_id: &str,
    player_id: &str,
    amount: u32,
) -> Result<(u16, serde_json::Value), String> {
    let resp = client
        .post(format!("{base}/table/{table_id}/bet"))
        .json(&json!({"player_id": player_id, "amount": amount}))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = resp.status().as_u16();
    let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    Ok((status, body))
}

/// POST /debug/set-deck/:table_id
async fn set_deck(
    base: &str,
    client: &reqwest::Client,
    table_id: &str,
    deck: &serde_json::Value,
) -> Result<(), String> {
    let status = client
        .post(format!("{base}/debug/set-deck/{table_id}"))
        .json(deck)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .status();
    if status.is_success() {
        Ok(())
    } else {
        Err(format!("set-deck returned {}", status))
    }
}

/// POST /action with Hold
async fn hold(base: &str, client: &reqwest::Client, hand_id: &str) -> Result<(), String> {
    client
        .post(format!("{base}/action"))
        .json(&json!({"hand_id": hand_id, "action": "Hold"}))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// POST /action with any action name. Returns the HTTP status code.
async fn post_action(
    base: &str,
    client: &reqwest::Client,
    hand_id: &str,
    action: &str,
) -> Result<u16, String> {
    let resp = client
        .post(format!("{base}/action"))
        .json(&json!({"hand_id": hand_id, "action": action}))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    Ok(resp.status().as_u16())
}

/// GET /table/:id → poll until predicate is satisfied. Returns final state JSON.
async fn poll_state(
    base: &str,
    client: &reqwest::Client,
    table_id: &str,
    predicate: impl Fn(&serde_json::Value) -> bool,
) -> Result<serde_json::Value, String> {
    let deadline = std::time::Instant::now() + Duration::from_secs(POLL_TIMEOUT_SECS);
    loop {
        if std::time::Instant::now() > deadline {
            return Err("poll_state timed out".to_string());
        }
        let body: serde_json::Value = client
            .get(format!("{base}/table/{table_id}"))
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())?;
        if predicate(&body) {
            return Ok(body);
        }
        tokio::time::sleep(Duration::from_millis(POLL_INTERVAL_MS)).await;
    }
}

/// GET /player/:id → balance
async fn get_balance(base: &str, client: &reqwest::Client, player_id: &str) -> Result<u32, String> {
    let resp: serde_json::Value = client
        .get(format!("{base}/player/{player_id}"))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    resp["balance"]
        .as_u64()
        .map(|v| v as u32)
        .ok_or_else(|| "missing balance field".to_string())
}

// ── deck constructors ──────────────────────────────────────────────────────

fn card_val(n: u8) -> serde_json::Value {
    json!({"suit": "Spades", "value": {"Value": n}})
}
fn card_king() -> serde_json::Value {
    json!({"suit": "Hearts", "value": "King"})
}
fn card_jack() -> serde_json::Value {
    json!({"suit": "Clubs", "value": "Jack"})
}
fn card_ace() -> serde_json::Value {
    json!({"suit": "Diamonds", "value": "Ace"})
}
/// Build a full 52-card deck; first `prefix` are provided, rest are Jacks.
fn make_deck(prefix: Vec<serde_json::Value>) -> serde_json::Value {
    let mut d = prefix;
    while d.len() < 52 {
        d.push(card_jack());
    }
    serde_json::Value::Array(d)
}

// ── scenarios ─────────────────────────────────────────────────────────────

/// Scenario 1: happy path — player wins (dealer busts).
///
/// Deck layout (dealer-first allocation):
///   [0] 7  dealer card 1
///   [1] 8  dealer card 2  → dealer: 15, must hit
///   [2] 9  player card 1
///   [3] 9  player card 2  → player: 18 (Active)
///   [4] J  dealer hit     → dealer: 25 (Bust) → Won(22) → balance +100
async fn scenario_happy_path() -> Result<(), String> {
    let (base, client) = spawn_server().await;
    let table_id = first_waiting_table(&base, &client).await?;
    let player_id = register_player(&base, &client).await?;

    let (_, join_body) = join_table(&base, &client, &table_id, &player_id).await?;
    let hand_id = join_body["hand_id"]
        .as_str()
        .ok_or("missing hand_id")?
        .to_owned();

    // Set deterministic deck during countdown window
    let deck = make_deck(vec![
        card_val(7),
        card_val(8),
        card_val(9),
        card_val(9),
        card_jack(),
    ]);
    set_deck(&base, &client, &table_id, &deck).await?;

    // Bet 100 (within balance of 1000)
    let (bet_status, _) = place_bet(&base, &client, &table_id, &player_id, 100).await?;
    if bet_status != 200 {
        return Err(format!("bet failed with status {bet_status}"));
    }

    // Wait for Active
    poll_state(&base, &client, &table_id, |s| s["game_state"] == "active").await?;

    // Player holds
    hold(&base, &client, &hand_id).await?;

    // Wait for waiting (game complete + resolving expired)
    poll_state(&base, &client, &table_id, |s| s["game_state"] == "waiting").await?;

    let balance = get_balance(&base, &client, &player_id).await?;
    if balance != 1100 {
        return Err(format!("expected balance 1100, got {balance}"));
    }
    Ok(())
}

/// Scenario 2: mid-game join rejection — player2 attempts to join an Active table.
async fn scenario_mid_game_join() -> Result<(), String> {
    let (base, client) = spawn_server().await;
    let table_id = first_waiting_table(&base, &client).await?;
    let player1 = register_player(&base, &client).await?;

    join_table(&base, &client, &table_id, &player1).await?;
    place_bet(&base, &client, &table_id, &player1, 1).await?;

    // Wait for Active (countdown expires)
    poll_state(&base, &client, &table_id, |s| s["game_state"] == "active").await?;

    // Player2 attempts to join while Active → 409
    let player2 = register_player(&base, &client).await?;
    let (status, body) = join_table(&base, &client, &table_id, &player2).await?;
    if status != 409 {
        return Err(format!("expected 409, got {status}"));
    }
    if body["rejected"] != true {
        return Err("expected rejected=true".to_string());
    }
    Ok(())
}

/// Scenario 3: insufficient funds — bet exceeds balance.
async fn scenario_insufficient_funds() -> Result<(), String> {
    let (base, client) = spawn_server().await;
    let table_id = first_waiting_table(&base, &client).await?;
    let player_id = register_player(&base, &client).await?;

    join_table(&base, &client, &table_id, &player_id).await?;

    // Attempt bet of 1001 with balance of 1000
    let (status, body) = place_bet(&base, &client, &table_id, &player_id, 1001).await?;
    if status != 400 {
        return Err(format!("expected 400, got {status}"));
    }
    if body["success"] != false {
        return Err("expected success=false".to_string());
    }
    let err_msg = body["error"].as_str().unwrap_or("");
    if !err_msg.contains("insufficient") {
        return Err(format!("expected 'insufficient' in error, got '{err_msg}'"));
    }
    Ok(())
}

/// Scenario 4: push (tied hands) — both player and dealer end at 20.
///
/// Deck layout (dealer-first):
///   [0] K  dealer card 1 (10)
///   [1] K  dealer card 2 (10)  → dealer: 20, stands → Holding(20)
///   [2] K  player card 1 (10)
///   [3] K  player card 2 (10)  → player: 20 (Active, holds)
///   Push → balance unchanged: 1000
async fn scenario_push() -> Result<(), String> {
    let (base, client) = spawn_server().await;
    let table_id = first_waiting_table(&base, &client).await?;
    let player_id = register_player(&base, &client).await?;

    let (_, join_body) = join_table(&base, &client, &table_id, &player_id).await?;
    let hand_id = join_body["hand_id"]
        .as_str()
        .ok_or("missing hand_id")?
        .to_owned();

    let deck = make_deck(vec![card_king(), card_king(), card_king(), card_king()]);
    set_deck(&base, &client, &table_id, &deck).await?;

    let (bet_status, _) = place_bet(&base, &client, &table_id, &player_id, 100).await?;
    if bet_status != 200 {
        return Err(format!("bet failed with status {bet_status}"));
    }

    poll_state(&base, &client, &table_id, |s| s["game_state"] == "active").await?;

    hold(&base, &client, &hand_id).await?;

    poll_state(&base, &client, &table_id, |s| s["game_state"] == "waiting").await?;

    let balance = get_balance(&base, &client, &player_id).await?;
    if balance != 1000 {
        return Err(format!("expected balance 1000 (push), got {balance}"));
    }
    Ok(())
}

/// Scenario 5: blackjack payout (3:2) — player gets 21 on deal, dealer stands at 17.
///
/// Deck layout (dealer-first):
///   [0] 7  dealer card 1
///   [1] J  dealer card 2 (10)  → dealer: 17, stands → Holding(17)
///   [2] A  player card 1 (11)
///   [3] K  player card 2 (10)  → player: 21, State::BlackJack on deal
///   Player holds → Holding(21) (overwrites BlackJack state)
///   Dealer holds at 17 (7+J=17, stands)
///   resolve_outcomes: Holding(21) > Holding(17) → Won(21) → balance += 150
async fn scenario_blackjack() -> Result<(), String> {
    let (base, client) = spawn_server().await;
    let table_id = first_waiting_table(&base, &client).await?;
    let player_id = register_player(&base, &client).await?;

    let (_, join_body) = join_table(&base, &client, &table_id, &player_id).await?;
    let hand_id = join_body["hand_id"]
        .as_str()
        .ok_or("missing hand_id")?
        .to_owned();

    let deck = make_deck(vec![card_val(7), card_jack(), card_ace(), card_king()]);
    set_deck(&base, &client, &table_id, &deck).await?;

    let (bet_status, _) = place_bet(&base, &client, &table_id, &player_id, 100).await?;
    if bet_status != 200 {
        return Err(format!("bet failed with status {bet_status}"));
    }

    poll_state(&base, &client, &table_id, |s| s["game_state"] == "active").await?;

    hold(&base, &client, &hand_id).await?;

    poll_state(&base, &client, &table_id, |s| s["game_state"] == "waiting").await?;

    let balance = get_balance(&base, &client, &player_id).await?;
    if balance != 1150 {
        return Err(format!("expected balance 1150 (3:2 payout), got {balance}"));
    }
    Ok(())
}

/// Scenario 6: multi-player — two players at same table, dealer busts, both win.
///
/// Deck layout (dealer-first, then player1, then player2 in join order):
///   [0] 7  dealer card 1
///   [1] 8  dealer card 2     → dealer: 15, must hit
///   [2] 9  player1 card 1
///   [3] 9  player1 card 2   → player1: 18 (Active)
///   [4] 9  player2 card 1
///   [5] 9  player2 card 2   → player2: 18 (Active)
///   [6] J  dealer hit       → dealer: 25 (Bust) → both Won(22) → both +100
async fn scenario_multi_player() -> Result<(), String> {
    let (base, client) = spawn_server().await;
    let table_id = first_waiting_table(&base, &client).await?;
    let player1 = register_player(&base, &client).await?;
    let player2 = register_player(&base, &client).await?;

    let (_, j1) = join_table(&base, &client, &table_id, &player1).await?;
    let hand1 = j1["hand_id"]
        .as_str()
        .ok_or("missing hand_id p1")?
        .to_owned();

    let (_, j2) = join_table(&base, &client, &table_id, &player2).await?;
    let hand2 = j2["hand_id"]
        .as_str()
        .ok_or("missing hand_id p2")?
        .to_owned();

    let deck = make_deck(vec![
        card_val(7),
        card_val(8),
        card_val(9),
        card_val(9),
        card_val(9),
        card_val(9),
        card_jack(),
    ]);
    set_deck(&base, &client, &table_id, &deck).await?;

    place_bet(&base, &client, &table_id, &player1, 100).await?;
    place_bet(&base, &client, &table_id, &player2, 100).await?;

    poll_state(&base, &client, &table_id, |s| s["game_state"] == "active").await?;

    // Submit Hold for both; the backend processes in turn order
    hold(&base, &client, &hand1).await?;
    hold(&base, &client, &hand2).await?;

    poll_state(&base, &client, &table_id, |s| s["game_state"] == "waiting").await?;

    let bal1 = get_balance(&base, &client, &player1).await?;
    let bal2 = get_balance(&base, &client, &player2).await?;

    if bal1 != 1100 {
        return Err(format!("player1 expected 1100, got {bal1}"));
    }
    if bal2 != 1100 {
        return Err(format!("player2 expected 1100, got {bal2}"));
    }
    Ok(())
}

// ── Phase 1e scenarios ────────────────────────────────────────────────────────

/// Scenario 7: double down win — player doubles on 12, draws to 20; dealer busts.
///
/// Deck layout (dealer-first allocation):
///   [0] 7   dealer card 1
///   [1] 8   dealer card 2  → dealer: 15, must hit
///   [2] 10  player card 1
///   [3] 2   player card 2  → player: 12 (Active, exactly 2 cards)
///   [4] 8   player DoubleDown card → player: 20, Holding(20)
///   [5] J   dealer hit     → dealer: 25 (Bust) → Won(22)
///
/// Balance quirk (existing engine behaviour):
///   place_bet(100) does NOT deduct. DoubleDown deducts extra 100 immediately (balance
///   900) and doubles bet to 200. Won(22): balance += 200 → 1100.
async fn scenario_double_down_win() -> Result<(), String> {
    let (base, client) = spawn_server().await;
    let table_id = first_waiting_table(&base, &client).await?;
    let player_id = register_player(&base, &client).await?;

    let (_, join_body) = join_table(&base, &client, &table_id, &player_id).await?;
    let hand_id = join_body["hand_id"]
        .as_str()
        .ok_or("missing hand_id")?
        .to_owned();

    let deck = make_deck(vec![
        card_val(7),
        card_val(8),
        card_val(10),
        card_val(2),
        card_val(8),
        card_jack(),
    ]);
    set_deck(&base, &client, &table_id, &deck).await?;

    let (bet_status, _) = place_bet(&base, &client, &table_id, &player_id, 100).await?;
    if bet_status != 200 {
        return Err(format!("bet failed with status {bet_status}"));
    }

    poll_state(&base, &client, &table_id, |s| s["game_state"] == "active").await?;

    let status = post_action(&base, &client, &hand_id, "DoubleDown").await?;
    if status != 202 {
        return Err(format!("DoubleDown expected 202, got {status}"));
    }

    poll_state(&base, &client, &table_id, |s| s["game_state"] == "waiting").await?;

    let balance = get_balance(&base, &client, &player_id).await?;
    if balance != 1100 {
        return Err(format!(
            "expected balance 1100 (double-down win), got {balance}"
        ));
    }
    Ok(())
}

/// Scenario 8: double down bust — player doubles on 16, draws to 26 (bust); dealer holds at 17.
///
/// Deck layout (dealer-first allocation):
///   [0] 7   dealer card 1
///   [1] 8   dealer card 2  → dealer: 15, must hit
///   [2] 10  player card 1
///   [3] 6   player card 2  → player: 16 (Active, exactly 2 cards)
///   [4] 10  player DoubleDown card → player: 26 (Bust)
///   [5] 2   dealer hit     → dealer: 17 (Holding) → player Lost(26)
///
/// Balance quirk:
///   DoubleDown deducts 100 → 900, doubles bet to 200.
///   Lost(26): balance -= 200 → 700.
async fn scenario_double_down_bust() -> Result<(), String> {
    let (base, client) = spawn_server().await;
    let table_id = first_waiting_table(&base, &client).await?;
    let player_id = register_player(&base, &client).await?;

    let (_, join_body) = join_table(&base, &client, &table_id, &player_id).await?;
    let hand_id = join_body["hand_id"]
        .as_str()
        .ok_or("missing hand_id")?
        .to_owned();

    let deck = make_deck(vec![
        card_val(7),
        card_val(8),
        card_val(10),
        card_val(6),
        card_val(10),
        card_val(2),
    ]);
    set_deck(&base, &client, &table_id, &deck).await?;

    let (bet_status, _) = place_bet(&base, &client, &table_id, &player_id, 100).await?;
    if bet_status != 200 {
        return Err(format!("bet failed with status {bet_status}"));
    }

    poll_state(&base, &client, &table_id, |s| s["game_state"] == "active").await?;

    let status = post_action(&base, &client, &hand_id, "DoubleDown").await?;
    if status != 202 {
        return Err(format!("DoubleDown expected 202, got {status}"));
    }

    poll_state(&base, &client, &table_id, |s| s["game_state"] == "waiting").await?;

    let balance = get_balance(&base, &client, &player_id).await?;
    if balance != 700 {
        return Err(format!(
            "expected balance 700 (double-down bust loss), got {balance}"
        ));
    }
    Ok(())
}

/// Scenario 9: surrender — player surrenders on 15; receives half-bet refund.
///
/// Deck layout (dealer-first allocation):
///   [0] 7   dealer card 1
///   [1] 8   dealer card 2  → dealer: 15, must hit
///   [2] 9   player card 1
///   [3] 6   player card 2  → player: 15 (Active, exactly 2 cards)
///   [4+] J  dealer hit     → dealer: 25 (Bust) — irrelevant, Surrendered already resolved
///
/// Balance quirk:
///   Outcome::Surrendered → balance += bet/2 = 50 → 1050.
///   (place_bet does not deduct; only apply_betting_outcomes acts on it.)
async fn scenario_surrender() -> Result<(), String> {
    let (base, client) = spawn_server().await;
    let table_id = first_waiting_table(&base, &client).await?;
    let player_id = register_player(&base, &client).await?;

    let (_, join_body) = join_table(&base, &client, &table_id, &player_id).await?;
    let hand_id = join_body["hand_id"]
        .as_str()
        .ok_or("missing hand_id")?
        .to_owned();

    let deck = make_deck(vec![
        card_val(7),
        card_val(8),
        card_val(9),
        card_val(6),
        card_jack(),
    ]);
    set_deck(&base, &client, &table_id, &deck).await?;

    let (bet_status, _) = place_bet(&base, &client, &table_id, &player_id, 100).await?;
    if bet_status != 200 {
        return Err(format!("bet failed with status {bet_status}"));
    }

    poll_state(&base, &client, &table_id, |s| s["game_state"] == "active").await?;

    let status = post_action(&base, &client, &hand_id, "Surrender").await?;
    if status != 202 {
        return Err(format!("Surrender expected 202, got {status}"));
    }

    poll_state(&base, &client, &table_id, |s| s["game_state"] == "waiting").await?;

    let balance = get_balance(&base, &client, &player_id).await?;
    if balance != 1050 {
        return Err(format!(
            "expected balance 1050 (surrender half-refund), got {balance}"
        ));
    }
    Ok(())
}

/// Scenario 10: split — player splits two 8s; both hands reach 17 and win when dealer busts.
///
/// Deck layout (dealer-first allocation):
///   [0] 7   dealer card 1
///   [1] 8   dealer card 2  → dealer: 15, must hit
///   [2] 8   player card 1  (hand1)
///   [3] 8   player card 2  (hand1: two 8s — split eligible)
///   [4] 9   hand1 extra card after split → 8+9=17
///   [5] 9   hand2 extra card after split → 8+9=17
///   [6] J   dealer hit     → dealer: 25 (Bust) → both Won(22)
///
/// Balance:
///   Split deducts 100 immediately → 900.
///   Both Won(22): +100, +100 → 1100.
///
/// Note: split_hand does NOT call resolve_turn, so active_hands stays on hand1 after
/// Split is processed. hand2's Hold is queued proactively and picked up once hand2
/// becomes active.
async fn scenario_split() -> Result<(), String> {
    let (base, client) = spawn_server().await;
    let table_id = first_waiting_table(&base, &client).await?;
    let player_id = register_player(&base, &client).await?;

    let (_, join_body) = join_table(&base, &client, &table_id, &player_id).await?;
    let hand_id = join_body["hand_id"]
        .as_str()
        .ok_or("missing hand_id")?
        .to_owned();

    let deck = make_deck(vec![
        card_val(7),
        card_val(8),
        card_val(8),
        card_val(8),
        card_val(9),
        card_val(9),
        card_jack(),
    ]);
    set_deck(&base, &client, &table_id, &deck).await?;

    let (bet_status, _) = place_bet(&base, &client, &table_id, &player_id, 100).await?;
    if bet_status != 200 {
        return Err(format!("bet failed with status {bet_status}"));
    }

    poll_state(&base, &client, &table_id, |s| s["game_state"] == "active").await?;

    // Submit Split
    let status = post_action(&base, &client, &hand_id, "Split").await?;
    if status != 202 {
        return Err(format!("Split expected 202, got {status}"));
    }

    // Poll until the split hand (hand2) appears in the table state
    let player_id_clone = player_id.clone();
    let hand_id_clone = hand_id.clone();
    let state_after_split = poll_state(&base, &client, &table_id, move |s| {
        s["hands"]
            .as_array()
            .map(|hands| {
                hands.iter().any(|h| {
                    h["hand"]["player"].as_str() == Some(player_id_clone.as_str())
                        && h["hand"]["id"].as_str() != Some(hand_id_clone.as_str())
                        // Exclude the dealer's self-referential hand
                        && h["hand"]["player"] != h["hand"]["dealer"]
                })
            })
            .unwrap_or(false)
    })
    .await?;

    // Discover hand2's UUID
    let hand2_id = state_after_split["hands"]
        .as_array()
        .and_then(|hands| {
            hands.iter().find(|h| {
                h["hand"]["player"].as_str() == Some(player_id.as_str())
                    && h["hand"]["id"].as_str() != Some(hand_id.as_str())
                    && h["hand"]["player"] != h["hand"]["dealer"]
            })
        })
        .and_then(|h| h["hand"]["id"].as_str().map(str::to_owned))
        .ok_or("hand2 not found after split")?;

    // Hold hand1 (currently active), and pre-queue Hold for hand2
    hold(&base, &client, &hand_id).await?;
    hold(&base, &client, &hand2_id).await?;

    poll_state(&base, &client, &table_id, |s| s["game_state"] == "waiting").await?;

    let balance = get_balance(&base, &client, &player_id).await?;
    if balance != 1100 {
        return Err(format!(
            "expected balance 1100 (split + both win), got {balance}"
        ));
    }
    Ok(())
}

/// Scenario 11: multi-round stability — plays 3 consecutive rounds on the same table
/// without a crash.
///
/// After each round, reset_game removes all player hands. The player re-joins (same
/// player_id), which triggers a fresh countdown because the table is empty again.
///
/// Deck each round (dealer busts, player wins with 18):
///   [0] 7  dealer card 1
///   [1] 8  dealer card 2  → 15, must hit
///   [2] 9  player card 1
///   [3] 9  player card 2  → 18 (Active)
///   [4] J  dealer hit     → 25 (Bust) → Won(22) → +100
///
/// Expected balance after 3 wins: 1000 + 3×100 = 1300.
async fn scenario_multi_round_no_crash() -> Result<(), String> {
    let (base, client) = spawn_server().await;
    let table_id = first_waiting_table(&base, &client).await?;
    let player_id = register_player(&base, &client).await?;

    let deck = make_deck(vec![
        card_val(7),
        card_val(8),
        card_val(9),
        card_val(9),
        card_jack(),
    ]);

    for round in 1u32..=3 {
        // Re-join each round (reset_game removed the previous hand)
        let (_, join_body) = join_table(&base, &client, &table_id, &player_id).await?;
        if join_body["rejected"] == true {
            return Err(format!("round {round}: join rejected unexpectedly"));
        }
        let hand_id = join_body["hand_id"]
            .as_str()
            .ok_or_else(|| format!("round {round}: missing hand_id"))?
            .to_owned();

        set_deck(&base, &client, &table_id, &deck).await?;

        let (bet_status, _) = place_bet(&base, &client, &table_id, &player_id, 100).await?;
        if bet_status != 200 {
            return Err(format!("round {round}: bet failed with status {bet_status}"));
        }

        poll_state(&base, &client, &table_id, |s| s["game_state"] == "active").await?;

        hold(&base, &client, &hand_id).await?;

        poll_state(&base, &client, &table_id, |s| s["game_state"] == "waiting").await?;
    }

    let balance = get_balance(&base, &client, &player_id).await?;
    if balance != 1300 {
        return Err(format!(
            "expected balance 1300 after 3 winning rounds, got {balance}"
        ));
    }
    Ok(())
}
