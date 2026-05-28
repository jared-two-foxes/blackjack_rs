# blackjack-rs — Playability Sprint Backlog

Generated from planning session. Implement phases roughly in order; Phase 2 (Split)
can be done in parallel with Phase 3 (TUI Client).

---

## Phase 0 — Workspace Conversion

- [x] **Add `[workspace]` to root `Cargo.toml`**
  Insert at the top of the file:
  ```toml
  [workspace]
  members = [".", "client"]
  resolver = "2"
  ```
  Server stays at root — no files move. Run `cargo build` to confirm the workspace
  resolves before adding the client crate.

- [x] **Create `client/` crate skeleton**
  Create `client/Cargo.toml`:
  ```toml
  [package]
  name = "blackjack-client"
  version = "0.1.0"
  edition = "2021"

  [dependencies]
  ratatui    = "0.28"
  crossterm  = "0.28"
  reqwest    = { version = "0.12", features = ["json"] }
  tokio      = { version = "1", features = ["full"] }
  serde      = { version = "1", features = ["derive"] }
  serde_json = "1"
  uuid       = { version = "1", features = ["v4", "serde"] }
  clap       = { version = "4", features = ["derive"] }
  ```
  Create `client/src/main.rs` with a `fn main() {}` stub so `cargo build -p
  blackjack-client` succeeds before any real code is written.

---

## Phase 1 — Server: Bug Fix + Double Down + Surrender

### 1a. `src/types.rs`

- [ ] **Add `Action::DoubleDown`, `Action::Split`, `Action::Surrender`**
  ```rust
  pub enum Action { Hit, Hold, DoubleDown, Split, Surrender }
  ```

- [ ] **Add `State::Surrendered`**
  ```rust
  pub enum State { Active, Holding(u8), Bust(u8), BlackJack, Surrendered }
  ```

- [ ] **Add `Outcome::Surrendered`**
  ```rust
  pub enum Outcome { Won(u8), Lost(u8), Push, Surrendered }
  ```

### 1b. `src/data_source.rs`

- [ ] **Fix deck exhaustion bug in `reset_game`**
  Current code only clears player hand allocations and player hand_states, leaving
  the dealer's entries accumulating across rounds.

  Replace:
  ```rust
  self.allocations.retain(|a| !player_hand_ids.contains(&a.hand));
  self.hand_states.retain(|hs| !player_hand_ids.contains(&hs.0));
  ```
  With:
  ```rust
  self.allocations.retain(|a| a.dealer != game_id);
  self.hand_states.retain(|hs| hs.1 != game_id);
  ```
  Also update the existing `reset_game` test: the comment
  *"Player hand allocations cleared (dealer allocations may remain)"* is wrong after
  this fix — assert that ALL allocations for the game are cleared.

- [ ] **Add `pending_actions: HashMap<Uuid, Action>` to `DataSource`**
  One pending action per hand_id. Replaces the shared `Vec<HandAction>` in `AppState`
  for the per-hand queuing contract. Note: the `Arc<Mutex<...>>` for the action queue
  lives in `lib.rs`/`AppState` — this field is for documentation of the logical model;
  see the `lib.rs` tasks for the actual struct change.

- [ ] **Clear pending actions in `reset_game`**
  After the existing clear operations, remove pending action entries for all hands
  belonging to this game (both player and dealer hands).

- [ ] **Add `Outcome::Surrendered` case to `apply_betting_outcomes`**
  ```rust
  Outcome::Surrendered => player.balance += amount / 2,
  ```

- [ ] **Add `split_hand` method to `DataSource`**
  Signature:
  ```rust
  pub fn split_hand(
      &mut self,
      hand_id: Uuid,
      player_id: Uuid,
      game_id: Uuid,
      original_bet_amount: u32,
  ) -> Option<Uuid>
  ```
  Steps:
  1. Validate player.balance >= original_bet_amount; deduct immediately.
  2. Create `Hand { id: new_hand_id, player: player_id, dealer: game_id }` and push to `self.hands`.
  3. Find the 2nd `CardAllocation` for `hand_id` (second by insertion order); reassign its `.hand` to `new_hand_id`.
  4. Find position of `hand_id` in `self.sequence` for this game; insert a new `Sequence { game_id, hand_id: new_hand_id }` at `pos + 1`.
  5. Push a new `Bet` for `new_hand_id` with the same `amount`, `player_id`, `dealer_id`.
  6. Deal 1 card to `hand_id`, 1 card to `new_hand_id` (via `allocate_cards`).
  7. Run `process_hand_states` on both new hands; push any resulting states to `self.hand_states`.
  8. Return `Some(new_hand_id)`.

### 1c. `src/utils.rs`

- [ ] **Add `card_split_value` helper**
  Returns a comparable u8 "category" for Split eligibility:
  ```rust
  pub fn card_split_value(card: &Card) -> u8 {
      match &card.value {
          CardValue::Ace => 11,
          CardValue::King | CardValue::Queen | CardValue::Jack => 10,
          CardValue::Value(v) => *v,
      }
  }
  ```
  Two cards are splittable when `card_split_value(a) == card_split_value(b)`.

- [ ] **Update `resolve_outcomes` unreachable arms to name `State::Surrendered`**
  The outer match on the dealer's state and the inner match on the player's state both
  have `_ => unreachable!(...)` catch-alls. Surrendered hands are pre-filtered (outcome
  pre-inserted before `resolve_turn`) so these arms will never be hit in practice, but
  update the patterns to be explicit:
  ```rust
  State::Active | State::Surrendered => unreachable!("...")
  ```

### 1d. `src/lib.rs`

- [ ] **Add new `ActionMsg` variants**
  ```rust
  pub enum ActionMsg { Hit, Hold, DoubleDown, Split, Surrender }
  ```

- [ ] **Redesign shared action queue: `Vec<HandAction>` → `HashMap<Uuid, Action>`**
  In `AppState`:
  ```rust
  pub actions: Arc<Mutex<HashMap<Uuid, crate::types::Action>>>,
  ```
  Update `app_and_state` to initialise `Arc::new(Mutex::new(HashMap::new()))`.
  Update `start_backend` signature accordingly.

- [ ] **Update `submit_action` handler — validation + 202/400 response**
  New logic:
  1. Lock `ds` (read-only borrow is fine).
  2. Find hand by `msg.hand_id` in `ds.hands`; return `400 "hand not found"` if missing.
  3. Check `ds.game_states[hand.dealer]` is `GameState::Active`; return `400 "game not active"` if not.
  4. Drop ds lock.
  5. Map `ActionMsg` → `Action`; insert into `state.actions` HashMap (overwrites any prior pending action for this hand).
  6. Return `(StatusCode::ACCEPTED, "Action queued")`.

- [ ] **Update backend loop Step 1 — drain with HashMap**
  Replace the Vec drain+partition with:
  ```rust
  if let Ok(mut actions_guard) = actions.try_lock() {
      let active_hands_snapshot = ds_guard.active_hands.clone();
      for hand_id in &active_hands_snapshot {
          if let Some(action) = actions_guard.remove(hand_id) {
              to_process.push((*hand_id, action));
          }
      }
  }
  ```

- [ ] **Update backend loop Step 2 — add DoubleDown arm**
  After the `Action::Hold` arm:
  ```
  Action::DoubleDown:
    1. Count ds_guard.allocations where .hand == hand_id — must be exactly 2.
    2. Find bet: ds_guard.bets.iter_mut().find(|b| b.hand_id == hand_id).
    3. original_amount = bet.amount.
    4. Find player: ds_guard.players.get_mut(&hand.player).
    5. Validate player.balance >= original_amount; skip (log warning) if not.
    6. player.balance -= original_amount.
    7. bet.amount *= 2.
    8. allocate_cards(1) — deal 1 card.
    9. Compute hand value; push State::Holding(v) or State::Bust(v).
       IMPORTANT: never push State::BlackJack here — DoubleDown always pays 1:1.
   10. resolve_turn().
  ```

- [ ] **Update backend loop Step 2 — add Surrender arm**
  ```
  Action::Surrender:
    1. Count allocations for hand_id — must be exactly 2.
    2. Push hand_state: State::Surrendered.
    3. Pre-insert outcome: ds_guard.outcomes.push((hand_id, Outcome::Surrendered)).
    4. resolve_turn().
  ```

- [ ] **Update backend loop Step 2 — add Split arm**
  ```
  Action::Split:
    1. Count allocations for hand_id — must be exactly 2.
    2. Get both cards; check card_split_value equality — skip if not splittable.
    3. Find bet for hand_id → original_amount.
    4. Call ds_guard.split_hand(hand_id, hand.player, hand.dealer, original_amount).
    5. Do NOT call resolve_turn() — player continues playing the original hand.
  ```

- [ ] **Add `active_hands` to `TableState` response struct**
  ```rust
  pub struct TableState {
      // ... existing fields ...
      pub active_hands: Vec<Uuid>,
  }
  ```
  Populate in `get_table_state`: filter `ds.active_hands` to hands whose dealer == table_id.

- [ ] **Hide dealer hole card in `get_table_state`**
  When building `HandInfo` for a dealer hand (`hand.id == hand.dealer`) and the game
  state is `GameState::Active`, truncate the cards vec to 1 (face-up card only):
  ```rust
  let cards = if hand.id == hand.dealer
      && matches!(game_state_enum, GameState::Active)
  {
      let mut v = ds.get_hand(h);
      v.truncate(1);
      v
  } else {
      ds.get_hand(h)
  };
  ```

- [ ] **Serialize new `State::Surrendered` in `get_table_state`**
  ```rust
  State::Surrendered => "surrendered".to_string(),
  ```

- [ ] **Serialize new `Outcome::Surrendered` in `get_table_state`**
  ```rust
  Outcome::Surrendered => "surrendered".to_string(),
  ```

### 1e. `src/bin/harness.rs`

- [ ] **Add harness scenario: `test_double_down_win`**
  Rig deck so player gets 10+2 (12), dealer gets low cards. Player submits DoubleDown.
  Assert: one card dealt, bet doubled, player ends with a Holding state, outcome is Won,
  player balance reflects 1:1 payout on doubled bet.

- [ ] **Add harness scenario: `test_double_down_bust`**
  Rig deck so player gets 10+6 (16), 3rd card is a 10 → bust.
  Assert: State::Bust, Outcome::Lost, balance deducted for doubled bet.

- [ ] **Add harness scenario: `test_surrender`**
  Player receives 2 cards, immediately submits Surrender.
  Assert: Outcome::Surrendered, balance increased by half the original bet,
  round resolves without waiting for the dealer.

- [ ] **Add harness scenario: `test_split`**
  Rig deck to deal player two 8s. Player submits Split.
  Assert: two hands in sequence after split, each receives a new card, both play to
  completion, outcomes assigned to both hands, balances adjusted.

- [ ] **Add harness scenario: `test_multi_round_no_crash`**
  Play 3 consecutive rounds on the same table (same dealer/game_id).
  Assert: no panic, all rounds complete, deck is reshuffled between rounds,
  outcomes and bets are cleared between rounds.
  (This is the deck exhaustion regression test.)

---

## Phase 2 — Server: Split (can run in parallel with Phase 3)

Split is covered by the tasks in Phase 1 above (`split_hand` method in data_source.rs,
`Action::Split` arm in the backend loop, `test_split` harness scenario). No additional
tasks — it was grouped here to signal it can be deferred if TUI work needs to start
before Split is complete.

---

## Phase 3 — TUI Client (`client/src/`)

### File structure to create

```
client/src/
├── main.rs      — tokio::main, terminal setup, event loop
├── api.rs       — reqwest wrappers for all server endpoints
├── state.rs     — App struct and Screen enum
├── types.rs     — local JSON structs mirroring server responses
└── ui.rs        — ratatui rendering functions (one per screen)
```

### `client/src/types.rs`

- [ ] **Define local JSON types** (mirror server API — no shared crate dependency)
  ```rust
  pub struct TableState {
      pub game_state: String,
      pub seconds_remaining: Option<u64>,
      pub hands: Vec<HandInfo>,
      pub outcomes: Vec<(Uuid, String)>,
      pub bets: Vec<Bet>,
      pub active_hands: Vec<Uuid>,
  }
  pub struct HandInfo { pub hand: Hand, pub cards: Vec<Card>, pub state: Option<String> }
  pub struct Hand     { pub id: Uuid, pub player: Uuid, pub dealer: Uuid }
  pub struct Card     { pub suit: String, pub value: String }
  pub struct Bet      { pub hand_id: Uuid, pub player_id: Uuid, pub dealer_id: Uuid, pub amount: u32 }
  pub struct TableInfo { pub id: Uuid, pub state: String, pub player_count: usize, pub seconds_remaining: Option<u64> }
  pub enum ActionMsg  { Hit, Hold, DoubleDown, Split, Surrender }
  ```

### `client/src/api.rs`

- [ ] **Implement all HTTP client functions** (all async, return `Result<T, reqwest::Error>`)
  ```rust
  pub async fn create_player(base: &str) -> Result<(Uuid, u32)>
  pub async fn get_player(base: &str, id: Uuid) -> Result<(Uuid, u32)>
  pub async fn get_tables(base: &str) -> Result<Vec<TableInfo>>
  pub async fn join_table(base: &str, table_id: Uuid, player_id: Uuid) -> Result<JoinResponse>
  pub async fn leave_table(base: &str, table_id: Uuid, player_id: Uuid) -> Result<()>
  pub async fn place_bet(base: &str, table_id: Uuid, player_id: Uuid, amount: u32) -> Result<()>
  pub async fn get_table(base: &str, table_id: Uuid) -> Result<TableState>
  pub async fn submit_action(base: &str, hand_id: Uuid, action: ActionMsg) -> Result<()>
  ```

### `client/src/state.rs`

- [ ] **Define `Screen` enum and `App` struct**
  ```rust
  pub enum Screen {
      Lobby,
      Betting { table_id: Uuid, hand_id: Uuid },
      Game    { table_id: Uuid, hand_id: Uuid },
      Result  { table_id: Uuid },
  }
  pub struct App {
      pub screen:         Screen,
      pub player_id:      Uuid,
      pub balance:        u32,
      pub tables:         Vec<TableInfo>,
      pub selected_table: usize,
      pub table_state:    Option<TableState>,
      pub bet_input:      String,
      pub status_msg:     Option<String>,
      pub action_staged:  Option<String>,  // shown as "★ [action] queued"
  }
  ```

- [ ] **Implement screen transition helpers**
  ```
  Lobby → Betting:    join_table succeeds
  Betting → Game:     poll sees game_state == "active"
  Game → Result:      poll sees game_state == "resolving"
  Result → Lobby:     Enter or Q keypress
  ```

- [ ] **Implement `available_actions` helper**
  Given the player's `HandInfo`, current bets, and balance — return which `ActionMsg`
  variants are eligible:
  - `Hit`, `Hold`: always if hand state is `None` (still active)
  - `DoubleDown`: active + exactly 2 cards + balance >= bet amount
  - `Split`: active + exactly 2 cards + both cards same split-category value + balance >= bet
  - `Surrender`: active + exactly 2 cards

- [ ] **Implement `card_display` helper**
  Format a card as inline text with Unicode suit symbol:
  ```
  Ace of Spades   → "A♠"
  King of Hearts  → "K♥"
  9 of Diamonds   → "9♦"
  hidden card     → "??"
  ```

### `client/src/ui.rs`

- [ ] **Implement `render_lobby` function**
  - Table list with id (truncated), state, player count, countdown timer.
  - Highlight selected row.
  - Bottom bar: `[↑↓] navigate  [Enter] join  [Q] quit`.

- [ ] **Implement `render_betting` function**
  - Show table ID, countdown timer (`seconds_remaining`), player balance.
  - Text input field for bet amount (digits only, max = balance).
  - Bottom bar: `[Enter] confirm  [Esc] leave table`.

- [ ] **Implement `render_game` function**
  - Dealer panel: show face-up card + `??` (hole card) during active state;
    show both cards once game_state is no longer "active".
    Detect dealer hand by `hand.player == hand.dealer`.
  - Player panels: all hands at this table. Highlight the player's own hand(s).
  - My hand section: cards, current hand value (compute locally), state string.
  - Action bar: `[H]it [S]tand [D]ouble [P]split [X]surrender`.
    Gray out (or omit) actions that are not in `available_actions()`.
    Show `★ [action] staged` when `app.action_staged` is set.
  - Top-right: balance.

- [ ] **Implement `render_result` function**
  - Show outcome for the player's hand(s): "Won +150", "Lost -100", "Push", "Surrendered -50".
  - Show new balance.
  - Bottom bar: `[Enter] play again  [Q] leave table`.

- [ ] **Implement top-level `render` dispatcher**
  ```rust
  pub fn render(f: &mut Frame, app: &App) {
      match &app.screen {
          Screen::Lobby          => render_lobby(f, app),
          Screen::Betting { .. } => render_betting(f, app),
          Screen::Game { .. }    => render_game(f, app),
          Screen::Result { .. }  => render_result(f, app),
      }
  }
  ```

### `client/src/main.rs`

- [ ] **Parse `--server` CLI argument**
  Default: `http://127.0.0.1:3000`. Use `clap::Parser`.

- [ ] **Set up terminal (crossterm)**
  Enable raw mode, enter alternate screen. Register panic hook and `Drop` impl to
  restore terminal state so the shell is not broken on crash.

- [ ] **Register player on startup**
  Call `api::create_player` and store `player_id` and `balance` in `App`.

- [ ] **Implement main event loop**
  Use `tokio::select!` with two branches:
  1. **Tick (500ms)**: poll server for updated state depending on current screen.
     Auto-transition screens based on `game_state` changes.
  2. **Keyboard**: read crossterm event; dispatch to a `handle_input` function.

- [ ] **Implement `handle_input` function**
  Match on `(app.screen, key_event)`:
  - Lobby: ↑↓ change `selected_table`; Enter → `join_table` and transition.
  - Betting: digits append to `bet_input`; Backspace deletes; Enter → `place_bet`.
  - Game: H → Hit, S → Stand/Hold, D → DoubleDown, P → Split, X → Surrender.
    All submit via `api::submit_action` (whether or not it is currently the player's turn).
    On 202 response, set `app.action_staged`.
  - Result: Enter → leave_table + go to Lobby; Q → same.

---

## Constraints & Notes

- **DoubleDown always pays 1:1**: backend pushes `State::Holding(v)` not `State::BlackJack`
  even when v == 21, ensuring `apply_betting_outcomes` pays 1:1 on the doubled bet.
- **Surrender bypasses dealer comparison**: `Outcome::Surrendered` is pre-inserted into
  `ds.outcomes` before `resolve_turn()`, so `resolve_outcomes` filters it out.
- **Action pre-staging is safe**: `HashMap<Uuid, Action>` stores at most one pending
  action per hand. Submitting a second action overwrites the first. The backend loop
  dequeues and processes exactly one action per active hand per tick.
- **Split does not call `resolve_turn`**: after splitting, the player continues on the
  original hand. The split hand is inserted into `sequence` at position+1 so
  `determine_next_hand` naturally routes to it after the original hand is done.
- **Dealer hole card hiding**: server truncates dealer `cards` to 1 when `GameState::Active`.
  Client renders the 2nd slot as `??`. Once state transitions away from active, the full
  hand is returned.
- **No shared types between server and client**: client declares its own JSON structs.
  This avoids pulling axum/server dependencies into the client binary.
