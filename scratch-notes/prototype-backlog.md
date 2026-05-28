# blackjack-rs — Prototype Backlog

Generated from planning session. Implement in order — later steps depend on earlier ones.

---

## Phase 1 — Critical Bug Fixes (nothing works without these)

- [x] **Fix backend Arc leak**
  `start_backend` currently receives a clone of `DataSource`, not a shared reference.
  Change signature to accept `Arc<Mutex<DataSource>>`. Remove the `.clone()` call in
  `app_and_state`. All state mutations in the loop must now lock and commit back to
  the shared store.

- [x] **Wire action results back into DataSource**
  `process_user_actions` returns `(Vec<CardAllocation>, Vec<HandState>)` but the
  caller discards them with `let _ = ...`. Save both vecs back into the locked
  DataSource after every loop iteration.

- [x] **Fix inverted partition logic in backend loop**
  The `partition_point` call splits the action queue the wrong way — active hands
  go into the skipped bucket and inactive ones into `to_process`. Reverse the logic.

- [x] **Fix busy-spin**
  Add `thread::sleep(Duration::from_millis(100))` at the end of the backend loop.

---

## Phase 2 — Dependency: Types & Cargo

- [x] **Add `rand` to Cargo.toml**
  Required for deck shuffling.
  ```toml
  rand = "0.8"
  ```

- [x] **Add `Outcome::Push` variant to `types.rs`**
  Tie resolution is not currently possible. Add `Push` as a third variant alongside
  `Won(u8)` and `Lost(u8)`.

- [x] **Add `Serialize`/`Deserialize` derives to `Card`, `Suit`, `CardValue`**
  These are currently missing. Required so card data can be included in HTTP
  responses for the CLI client.

- [x] **Add `Player` struct to `types.rs`**
  ```rust
  pub struct Player { pub id: Uuid, pub balance: u32 }
  ```

- [x] **Add `Bet` struct to `types.rs`**
  ```rust
  pub struct Bet {
      pub hand_id: Uuid,
      pub player_id: Uuid,
      pub dealer_id: Uuid,
      pub amount: u32,
  }
  ```

---

## Phase 3 — DataSource Extensions

- [x] **Extend `GameState` with countdown and resolving variants**
  ```rust
  pub enum GameState {
      Waiting,
      Countdown { started_at: std::time::Instant },
      Active,
      Resolving { started_at: std::time::Instant },
  }
  ```

- [x] **Add `players` and `bets` fields to `DataSource`**
  ```rust
  pub players: HashMap<Uuid, Player>,
  pub bets: Vec<Bet>,
  ```

- [x] **Add `register_player()` method**
  Creates a `Player` with `balance = 1000`, inserts into `self.players`, returns `Uuid`.

- [x] **Add `place_bet()` method**
  Signature: `place_bet(player_id, dealer_id, amount) -> Result<(), BetError>`
  Validation: amount >= MIN_BET, amount <= player.balance, game is in Countdown state,
  no existing bet for this player at this table this round. Store in `self.bets`.

- [x] **Add `player_count(dealer_id)` method**
  Returns count of non-dealer hands at the given table (hands where `hand.id != hand.dealer`).

- [x] **Shuffle deck in `add_game()`**
  After `new_deck()`, call `deck.shuffle(&mut rng)` before inserting into `self.decks`.

- [x] **Update `start_game()` to filter by bet**
  Only deal to and sequence hands that have a corresponding entry in `self.bets`.
  Players seated but without a bet are silently excluded for this round.

- [x] **Add `apply_betting_outcomes()` method**
  After `resolve_turn`, iterate `self.outcomes`. For each outcome, look up the
  corresponding bet and adjust the player's balance:
  - `Won` (non-blackjack): `balance += bet`
  - `Won` (blackjack): `balance += (bet * 3) / 2`  (integer floor)
  - `Lost`: `balance -= bet`
  - `Push`: no change
  Players at zero balance are NOT removed — they remain seated and spectate.

- [x] **Add `reset_game(game_id)` method**
  Clears all player hands (not the dealer's self-referential hand), allocations,
  hand_states, outcomes, bets, and sequence entries for the given table. Reshuffles
  the deck. Sets state back to `GameState::Waiting`. Players must re-join and re-bet
  for the next round.

---

## Phase 4 — Utils Updates

- [x] **Update `resolve_outcomes()` to handle `Push`**
  When both player and dealer are `State::Holding(v)` and `v == dealer_value`, emit
  `Outcome::Push` instead of `Outcome::Lost`.

- [x] **Add `shuffle_deck(deck: &mut Deck)` helper**
  Wraps `deck.shuffle(&mut thread_rng())` from the `rand` crate.

---

## Phase 5 — Backend Loop Rewrite

- [x] **Rewrite backend loop body**
  On each 100ms tick, lock DataSource and run the following in order:

  1. **Countdown check**: for each table in `Countdown` state —
     if `elapsed >= COUNTDOWN_DURATION_SECS` OR `player_count >= MAX_PLAYERS_PER_TABLE`,
     call `start_game`.

  2. **Action processing**: drain the action queue for hands currently in `active_hands`.
     For each `Hit`: allocate 1 card, compute new hand state, save `CardAllocation` and
     updated `HandState` back to DataSource.
     For each `Hold`: record `State::Holding(value)` in `hand_states`.

  3. **Turn advancement**: after processing each action, call `resolve_turn()` to
     advance `active_hands` to the next hand in sequence.

  4. **Dealer AI trigger**: when `active_hands` contains only the dealer's hand (i.e.
     `hand.id == hand.dealer`), run dealer auto-play:
     while dealer hand value < 17, allocate 1 card and recompute.
     Then mark dealer as `Holding(value)` or `Bust(value)` in `hand_states`.

  5. **Outcome resolution**: once the dealer's hand is terminal, call `resolve_turn()`
     to compute all `HandOutcome`s. Transition table to
     `Resolving { started_at: Instant::now() }`.

  6. **Resolving check**: for each table in `Resolving` state —
     if `elapsed >= RESOLVING_DISPLAY_SECS`, call `apply_betting_outcomes()` then
     `reset_game()`.

  7. **Sleep**: `thread::sleep(Duration::from_millis(100))`.

---

## Phase 6 — New HTTP Endpoints

- [x] **`POST /player`**
  Calls `ds.register_player()`. Returns `{ player_id, balance: 1000 }`.

- [x] **`GET /player/:id`**
  Looks up player in `ds.players`. Returns `{ player_id, balance }` or 404.

- [x] **`GET /tables`**
  Iterates `ds.game_states`. Returns an array of:
  ```json
  {
    "id": "...",
    "state": "countdown",
    "player_count": 2,
    "seconds_remaining": 17
  }
  ```
  `seconds_remaining` is `null` when state is not `Countdown`.

- [x] **`POST /table/:id/bet`**
  Body: `{ player_id, amount }`. Calls `ds.place_bet()`.
  Returns `{ success: true, balance_remaining }` on success, or an error string on
  failure (insufficient funds, wrong game state, duplicate bet, etc.).

---

## Phase 7 — Update Existing Endpoints

- [x] **Update `join_table` handler**
  - Return 409 if table state is `Active` or `Resolving`.
  - After successful seat: if table was `Waiting` and this is the first player,
    transition to `Countdown { started_at: Instant::now() }`.

- [x] **Update `GET /table/:id` response**
  Extend `TableState` with:
  - `game_state: String`
  - `seconds_remaining: Option<u64>`
  - `outcomes: Vec<(Uuid, Outcome)>`
  - `bets: Vec<Bet>`
  - Replace plain `hands: Vec<Hand>` with a `HandInfo` wrapper that includes the
    hand, its current cards (resolved from the deck+allocations), and its current
    state so a CLI client can render the full table without secondary requests.

---

## Phase 8 — Testing

- [x] **Full game loop test**
  register → join → bet → wait for start → submit Hit/Hold actions → dealer auto-plays
  → assert outcomes → assert balances updated correctly → assert table resets.

- [x] **Sit-out test**
  Player joins but does not place a bet before countdown expires. Assert their hand
  is excluded from the deal and they are not in `active_hands`.

- [x] **Zero-balance spectate test**
  Player loses enough hands to reach 0 chips. Assert they remain in table state,
  `GET /player/:id` returns `balance: 0`, and `POST /table/:id/bet` with any amount
  is rejected.

- [x] **Push test**
  Load a deck so player and dealer end with the same value. Assert `Outcome::Push`
  is returned and player balance is unchanged.

- [x] **Blackjack payout test**
  Load a deck to deal a natural blackjack. Assert payout is `floor(bet * 3 / 2)`.

- [x] **Mid-game join rejection test**
  Assert `POST /table/:id/join` returns 409 during `Active` and `Resolving` states.
  (Covered by handler-level unit tests added in Phase 7.)

---

## Constants (define once at top of `lib.rs` or a new `config.rs`)

```rust
pub const MAX_PLAYERS_PER_TABLE: usize = 6;
pub const COUNTDOWN_DURATION_SECS: u64 = 30;
pub const RESOLVING_DISPLAY_SECS: u64 = 10;
pub const STARTING_BALANCE: u32 = 1000;
pub const MIN_BET: u32 = 1;
```
