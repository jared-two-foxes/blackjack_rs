/// Number of tables to generate at startup
pub const DEFAULT_TABLE_COUNT: usize = 8;
pub const STARTING_BALANCE: u32 = 1000;
pub const MIN_BET: u32 = 1;
pub const MAX_PLAYERS_PER_TABLE: usize = 6;
pub const COUNTDOWN_DURATION_SECS: u64 = 30;
pub const RESOLVING_DISPLAY_SECS: u64 = 10;

pub fn effective_countdown_secs() -> u64 {
    std::env::var("BLACKJACK_COUNTDOWN_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(COUNTDOWN_DURATION_SECS)
}

pub fn effective_resolving_secs() -> u64 {
    std::env::var("BLACKJACK_RESOLVING_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(RESOLVING_DISPLAY_SECS)
}

use log::warn;
use std::collections::HashMap;
use uuid::Uuid;

use crate::types::*;
use crate::utils::*;

#[derive(Debug, Clone)]
pub enum GameState {
    Waiting,
    Countdown { started_at: std::time::Instant },
    Active,
    Resolving { started_at: std::time::Instant },
}

#[derive(Default, Clone)]
pub struct DataSource {
    pub hands: Vec<Hand>,
    pub decks: HashMap<Uuid, Deck>, // map of game_id to Deck for a given game
    game_states: HashMap<Uuid, GameState>,
    pub allocations: Vec<CardAllocation>,
    pub hand_states: Vec<HandState>,
    pub outcomes: Vec<HandOutcome>,
    sequence: Vec<Sequence>,
    pub active_hands: Vec<Uuid>,
    pub players: HashMap<Uuid, Player>,
    pub bets: Vec<Bet>,
}

impl DataSource {
    /// Generate N tables (games) at startup, returning their IDs
    pub fn generate_tables(&mut self, count: usize) -> Vec<Uuid> {
        let mut ids = Vec::with_capacity(count);
        for _ in 0..count {
            ids.push(self.add_game());
        }
        ids
    }

    /// Removes a player from a table. Returns the hand id if found and removed.
    pub fn remove_player(&mut self, dealer_id: Uuid, player_id: Uuid) -> Option<Uuid> {
        if let Some(pos) = self
            .hands
            .iter()
            .position(|h| h.dealer == dealer_id && h.player == player_id)
        {
            let hand_id = self.hands[pos].id;
            self.hands.remove(pos);
            // Also remove from active_hands if present
            self.active_hands.retain(|&hid| hid != hand_id);
            Some(hand_id)
        } else {
            None
        }
    }

    pub fn add_game(&mut self) -> Uuid {
        let dealer_id = Uuid::new_v4();
        let mut deck = new_deck();
        shuffle_deck(&mut deck);
        self.decks.insert(dealer_id, deck);
        self.game_states.insert(dealer_id, GameState::Waiting);
        self.hands.push(Hand {
            id: dealer_id,
            player: dealer_id,
            dealer: dealer_id,
        });
        dealer_id
    }

    //@todo: This needs to return something I guess to indicate success or failure.
    pub fn set_deck(&mut self, game_id: Uuid, deck: Deck) {
        self.decks
            .entry(game_id)
            .and_modify(|current| *current = deck);
    }

    //@todo: this is a little awkward.  We should potentially have a second function to
    // create a hand which returns the hand_id else how does the client know how to add
    // an action?  Yes this for sure.  id & player should be different id's
    /// Attempts to seat a player at a table (dealer_id). Returns Some(hand_id) if seated, None if already seated.
    pub fn add_player(&mut self, dealer_id: Uuid, player_id: Uuid) -> Option<Uuid> {
        // Check if player is already seated at this table
        if self
            .hands
            .iter()
            .any(|h| h.dealer == dealer_id && h.player == player_id)
        {
            return None;
        }
        let hand_id = Uuid::new_v4();
        self.hands.push(Hand {
            id: hand_id,
            player: player_id,
            dealer: dealer_id,
        });
        Some(hand_id)
    }

    pub fn allocate_cards(&mut self, hands: &[Hand], count: usize) -> Vec<CardAllocation> {
        let mut new_allocations: Vec<CardAllocation> = Vec::new();
        for h in hands {
            // Pass both existing allocations and those already assigned this
            // batch so each hand gets a unique, non-overlapping card index.
            let combined: Vec<CardAllocation> = self
                .allocations
                .iter()
                .chain(new_allocations.iter())
                .cloned()
                .collect();
            let cards = draw_cards(h, &combined, count);
            new_allocations.extend(cards);
        }
        new_allocations
    }

    //@todo: I think this should this return a uuid; reasons 2 fold, we probably
    //       should have a means to identify the action, and we dont want methods
    //       with no return type.
    /*pub fn add_action(&mut self, hand_id: Uuid, action: Action) {
        match action {
            Action::Hit => trace!("server: Adding Hit Action for {}", hand_id),
            Action::Hold => trace!("server: Adding Hold Action for {}", hand_id),
        };
        self.actions.push((hand_id, action));
    }*/

    //
    // These are domain specific rather than Data based.  This feels like it should
    // reside elsewhere and that object or whatever should have a reference/own the
    // DataSource....
    //

    pub fn get_hand(&self, hand: &Hand) -> Vec<Card> {
        let deck = self
            .decks
            .get(&hand.dealer)
            .expect("Unable to find deck for table");

        self.allocations
            .iter()
            .filter(|a| a.hand == hand.id)
            .map(|a| deck[a.card_idx].clone())
            .collect::<Vec<_>>()
    }

    pub fn process_action(&mut self, action: Action, hand: &Hand, value: u8) -> State {
        match action {
            Action::Hit => {
                let hands = vec![hand.clone()];
                self.allocate_cards(&hands, 1);
                let cards = self.get_hand(hand);
                let new_value = hand_value(&cards);
                if new_value > 21 {
                    State::Bust(new_value)
                } else if new_value == 21 {
                    State::BlackJack
                } else {
                    State::Active
                }
            }
            Action::Hold => State::Holding(value),
            // DoubleDown, Split, and Surrender require additional context (balance, second hand, etc.)
            // and are handled directly in the backend loop; these arms are unreachable via process_action.
            Action::DoubleDown | Action::Split | Action::Surrender => {
                unreachable!("DoubleDown/Split/Surrender must be handled by the backend loop, not process_action")
            }
        }
    }

    pub fn start_game(&mut self, game_id: Uuid) {
        // Grab the hands for the given game: dealer self-hand always included,
        // player hands only if they have a corresponding bet.
        let hands = self
            .hands
            .iter()
            .filter(|h| {
                h.dealer == game_id
                    && (h.id == h.dealer
                        || self
                            .bets
                            .iter()
                            .any(|b| b.dealer_id == game_id && b.hand_id == h.id))
            })
            .cloned()
            .collect::<Vec<_>>();

        // Every hand gets dealt 2 cards.
        let allocations = self.allocate_cards(&hands, 2);

        // Combine the allocations into the master allocation list
        self.allocations.extend(allocations);

        // We now need to check the hand states incase anything interesting has
        // resolved from that.
        let resulting_states = process_hand_states(&hands, &self.allocations, &self.decks);

        // Merge any hand_states into the master state list
        self.hand_states.extend(resulting_states);

        // Determine turn order from the already-filtered hands.
        let mut sequence = hands
            .iter()
            .map(|h| Sequence {
                game_id,
                hand_id: h.id,
            })
            .collect::<Vec<_>>();

        //@todo: Sort so that the dealer is last in this list.
        sequence.sort_unstable_by(|a, b| {
            let a_is_dealer = a.game_id == a.hand_id;
            let b_is_dealer = b.game_id == b.hand_id;
            if a_is_dealer {
                std::cmp::Ordering::Greater
            } else if b_is_dealer {
                std::cmp::Ordering::Less
            } else {
                a.hand_id.cmp(&b.hand_id)
            }
        });

        // And push the first starting hand
        match sequence.first() {
            Some(s) => {
                //assert!(!is_dealer(get_hand(s.hand_id)));
                self.active_hands.push(s.hand_id)
            }
            _ => warn!("This should be an error, the sequence vec is empty"),
        };

        // Push the sequence onto the master list.
        self.sequence.extend(sequence);

        // Flag the game as active
        self.game_states
            .entry(game_id)
            .and_modify(|gs| *gs = GameState::Active);
    }

    pub fn resolve_turn(&mut self) {
        let new_outcomes = resolve_outcomes(&self.hand_states, &self.outcomes);
        self.outcomes.extend(new_outcomes);

        self.active_hands = self
            .active_hands
            .iter()
            .filter_map(|current_hand_id| {
                determine_next_hand(
                    *current_hand_id,
                    &self.sequence,
                    &self.hands,
                    &self.hand_states,
                )
            })
            .collect::<Vec<_>>();
    }

    /// Returns the number of non-dealer player seats at the given table.
    pub fn player_count(&self, dealer_id: Uuid) -> usize {
        self.hands
            .iter()
            .filter(|h| h.dealer == dealer_id && h.id != h.dealer)
            .count()
    }

    /// Creates a new Player with STARTING_BALANCE chips, inserts into self.players, and returns the new Uuid.
    pub fn register_player(&mut self) -> Uuid {
        let id = Uuid::new_v4();
        self.players.insert(
            id,
            Player {
                id,
                balance: STARTING_BALANCE,
            },
        );
        id
    }

    /// Place a bet for a player at a table.
    ///
    /// # Errors
    /// - `BetError::PlayerNotFound`    — player_id not in self.players
    /// - `BetError::BelowMinimum`      — amount < MIN_BET
    /// - `BetError::InsufficientFunds` — amount > player.balance
    /// - `BetError::WrongGameState`    — table is not in Countdown state
    /// - `BetError::DuplicateBet`      — player already has a bet at this dealer this round
    pub fn place_bet(
        &mut self,
        player_id: Uuid,
        dealer_id: Uuid,
        amount: u32,
    ) -> Result<(), BetError> {
        // 1. Check player exists
        let player = self
            .players
            .get(&player_id)
            .ok_or(BetError::PlayerNotFound)?;
        // 2. Validate amount
        if amount < MIN_BET {
            return Err(BetError::BelowMinimum);
        }
        if amount > player.balance {
            return Err(BetError::InsufficientFunds);
        }
        // 3. Check game state is Countdown
        let state = self
            .game_states
            .get(&dealer_id)
            .ok_or(BetError::WrongGameState)?;
        if !matches!(state, GameState::Countdown { .. }) {
            return Err(BetError::WrongGameState);
        }
        // 4. No duplicate bet for this player at this table
        if self
            .bets
            .iter()
            .any(|b| b.player_id == player_id && b.dealer_id == dealer_id)
        {
            return Err(BetError::DuplicateBet);
        }
        // 5. Find hand_id for this player at this table
        let hand_id = self
            .hands
            .iter()
            .find(|h| h.player == player_id && h.dealer == dealer_id)
            .map(|h| h.id)
            .unwrap_or_else(Uuid::new_v4);
        // 6. Store the bet
        self.bets.push(Bet {
            hand_id,
            player_id,
            dealer_id,
            amount,
        });
        Ok(())
    }

    /// Applies all resolved hand outcomes to player balances and clears the outcomes list.
    ///
    /// Payout rules:
    /// - Won (non-blackjack, i.e. Won(v) where v < 21): balance += bet.amount
    /// - Won (blackjack, Won(21)):                       balance += (bet.amount * 3) / 2  (integer floor)
    /// - Lost (any):                                     balance -= bet.amount  (saturating — floor at 0)
    /// - Push:                                           no change
    ///
    /// After processing, self.outcomes is cleared.
    pub fn apply_betting_outcomes(&mut self) {
        for (hand_id, outcome) in &self.outcomes {
            if let Some(bet) = self.bets.iter().find(|b| b.hand_id == *hand_id) {
                let amount = bet.amount;
                let player_id = bet.player_id;
                if let Some(player) = self.players.get_mut(&player_id) {
                    match outcome {
                        Outcome::Won(21) => player.balance += (amount * 3) / 2,
                        Outcome::Won(_) => player.balance += amount,
                        Outcome::Lost(_) => player.balance = player.balance.saturating_sub(amount),
                        Outcome::Push => {}
                        Outcome::Surrendered => player.balance += amount / 2,
                    }
                }
            }
        }
        self.outcomes.clear();
    }

    /// Resets a completed table for the next round.
    ///
    /// What is cleared:
    /// - All non-dealer hands for this table (hands where hand.dealer == game_id && hand.id != hand.dealer)
    /// - All card allocations for this table (allocations where allocation.hand is one of the above)
    /// - All hand_states for those same hand IDs
    /// - All sequence entries for this game (Sequence.game_id == game_id)
    /// - All active_hands entries for this game
    /// - All bets for this table (bet.dealer_id == game_id)
    /// - All outcomes for this table (outcomes keys that match the removed hand IDs)
    ///
    /// The dealer's self-referential hand (hand.id == hand.dealer) is kept.
    ///
    /// After clearing: reshuffles the deck for this table (using shuffle_deck), then
    /// sets game_states[game_id] back to GameState::Waiting.
    pub fn reset_game(&mut self, game_id: Uuid) {
        // Collect all non-dealer hand IDs for this table
        let player_hand_ids: Vec<Uuid> = self
            .hands
            .iter()
            .filter(|h| h.dealer == game_id && h.id != h.dealer)
            .map(|h| h.id)
            .collect();

        // Remove those hands
        self.hands
            .retain(|h| !(h.dealer == game_id && h.id != h.dealer));

        // Remove allocations for those hands AND the dealer's own allocations
        self.allocations.retain(|a| a.dealer != game_id);

        // Remove hand_states for those hands AND the dealer hand_state
        self.hand_states.retain(|hs| hs.1 != game_id);

        // Remove sequence entries for this game
        self.sequence.retain(|s| s.game_id != game_id);

        // Remove active_hands for those hands
        self.active_hands.retain(|id| !player_hand_ids.contains(id));

        // Remove bets for this table
        self.bets.retain(|b| b.dealer_id != game_id);

        // Remove outcomes for those hands
        self.outcomes.retain(|o| !player_hand_ids.contains(&o.0));

        // Reshuffle the deck
        if let Some(deck) = self.decks.get_mut(&game_id) {
            shuffle_deck(deck);
        }

        // Reset game state
        self.game_states
            .entry(game_id)
            .and_modify(|gs| *gs = GameState::Waiting);
    }

    /// Returns a read-only reference to the game_states map (needed by the backend loop).
    pub fn get_game_states(&self) -> &HashMap<Uuid, GameState> {
        &self.game_states
    }

    /// Transitions a table from Active to Resolving, recording the current instant.
    pub fn transition_to_resolving(&mut self, game_id: Uuid) {
        self.game_states.entry(game_id).and_modify(|gs| {
            *gs = GameState::Resolving {
                started_at: std::time::Instant::now(),
            };
        });
    }

    /// Transitions a table from Waiting to Countdown, recording the current instant.
    /// Called by join_table when the first player joins.
    pub fn transition_to_countdown(&mut self, game_id: Uuid) {
        self.game_states.entry(game_id).and_modify(|gs| {
            if matches!(gs, GameState::Waiting) {
                *gs = GameState::Countdown {
                    started_at: std::time::Instant::now(),
                };
            }
        });
    }

    /// Splits a hand mid-game. Returns the new hand's UUID on success, or None if the
    /// split is not valid (insufficient funds, wrong card count, etc.).
    ///
    /// Steps:
    /// 1. Validate player has enough balance; deduct immediately.
    /// 2. Create a new Hand for the same player and push to self.hands.
    /// 3. Reassign the 2nd CardAllocation for `hand_id` to the new hand.
    /// 4. Insert a new Sequence entry at the position immediately after `hand_id`.
    /// 5. Push a new Bet for the new hand (same amount/player/dealer).
    /// 6. Deal 1 card to each of the two hands.
    /// 7. Compute new hand_states for both and push any non-Active results.
    /// 8. Return Some(new_hand_id).
    pub fn split_hand(
        &mut self,
        hand_id: Uuid,
        player_id: Uuid,
        game_id: Uuid,
        original_bet_amount: u32,
    ) -> Option<Uuid> {
        // 1. Deduct the extra bet immediately
        let player = self.players.get_mut(&player_id)?;
        if player.balance < original_bet_amount {
            return None;
        }
        player.balance -= original_bet_amount;

        // 2. Create the new hand
        let new_hand_id = Uuid::new_v4();
        self.hands.push(Hand {
            id: new_hand_id,
            player: player_id,
            dealer: game_id,
        });

        // 3. Reassign the 2nd allocation for hand_id to the new hand
        let second_alloc_pos = self
            .allocations
            .iter()
            .enumerate()
            .filter(|(_, a)| a.hand == hand_id)
            .nth(1)
            .map(|(i, _)| i);
        if let Some(pos) = second_alloc_pos {
            self.allocations[pos].hand = new_hand_id;
        }

        // 4. Insert a new Sequence entry immediately after hand_id's position
        let seq_pos = self
            .sequence
            .iter()
            .position(|s| s.game_id == game_id && s.hand_id == hand_id);
        let insert_at = seq_pos.map(|p| p + 1).unwrap_or(self.sequence.len());
        self.sequence.insert(
            insert_at,
            Sequence {
                game_id,
                hand_id: new_hand_id,
            },
        );

        // 5. Push a new Bet for the new hand
        self.bets.push(Bet {
            hand_id: new_hand_id,
            player_id,
            dealer_id: game_id,
            amount: original_bet_amount,
        });

        // 6. Deal 1 card to each hand
        let original_hand = self.hands.iter().find(|h| h.id == hand_id).cloned()?;
        let new_hand = self.hands.iter().find(|h| h.id == new_hand_id).cloned()?;
        let orig_allocs = self.allocate_cards(std::slice::from_ref(&original_hand), 1);
        self.allocations.extend(orig_allocs);
        let new_allocs = self.allocate_cards(std::slice::from_ref(&new_hand), 1);
        self.allocations.extend(new_allocs);

        // 7. Compute new hand_states for both hands
        let both = vec![original_hand, new_hand];
        let new_states = crate::utils::process_hand_states(&both, &self.allocations, &self.decks);
        self.hand_states.extend(new_states);

        Some(new_hand_id)
    }

    #[cfg(test)]
    pub(crate) fn set_game_state_countdown(&mut self, dealer_id: Uuid) {
        self.game_states.insert(
            dealer_id,
            GameState::Countdown {
                started_at: std::time::Instant::now(),
            },
        );
    }

    #[cfg(test)]
    pub(crate) fn sequence_contains(&self, game_id: Uuid, hand_id: Uuid) -> bool {
        self.sequence
            .iter()
            .any(|s| s.game_id == game_id && s.hand_id == hand_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_count_returns_zero_for_new_table_and_correct_count_after_adding_players() {
        let mut ds = DataSource::default();
        let dealer_id = ds.add_game();
        // Freshly added table: only the dealer's self-hand exists, count must be 0
        assert_eq!(ds.player_count(dealer_id), 0);
        // Add two players
        let p1 = Uuid::new_v4();
        let p2 = Uuid::new_v4();
        ds.add_player(dealer_id, p1);
        ds.add_player(dealer_id, p2);
        assert_eq!(ds.player_count(dealer_id), 2);
        // Dealer's self-referential hand is NOT counted
        assert_eq!(
            ds.hands
                .iter()
                .filter(|h| h.dealer == dealer_id && h.id == h.dealer)
                .count(),
            1,
            "dealer self-hand should still exist"
        );
    }

    #[test]
    fn game_state_countdown_and_resolving_can_be_constructed() {
        let countdown = GameState::Countdown {
            started_at: std::time::Instant::now(),
        };
        let resolving = GameState::Resolving {
            started_at: std::time::Instant::now(),
        };
        // Verify Debug formatting works (derives Debug)
        assert!(format!("{:?}", countdown).contains("Countdown"));
        assert!(format!("{:?}", resolving).contains("Resolving"));
    }

    #[test]
    fn register_player_creates_two_distinct_players_with_starting_balance() {
        let mut ds = DataSource::default();
        let id1 = ds.register_player();
        let id2 = ds.register_player();
        assert_ne!(id1, id2);
        let p1 = ds.players.get(&id1).expect("player 1 not found");
        let p2 = ds.players.get(&id2).expect("player 2 not found");
        assert_eq!(p1.balance, 1000);
        assert_eq!(p2.balance, 1000);
    }

    // ── place_bet helpers ──────────────────────────────────────────────────────

    /// Set up a DataSource with one registered player and one table in Countdown state.
    fn setup_countdown() -> (DataSource, Uuid, Uuid) {
        let mut ds = DataSource::default();
        let player_id = ds.register_player();
        let dealer_id = ds.add_game();
        // Transition the table to Countdown
        ds.game_states.insert(
            dealer_id,
            GameState::Countdown {
                started_at: std::time::Instant::now(),
            },
        );
        (ds, player_id, dealer_id)
    }

    #[test]
    fn place_bet_success_appends_bet() {
        let (mut ds, player_id, dealer_id) = setup_countdown();
        let result = ds.place_bet(player_id, dealer_id, MIN_BET);
        assert!(result.is_ok());
        assert_eq!(ds.bets.len(), 1);
        let bet = &ds.bets[0];
        assert_eq!(bet.player_id, player_id);
        assert_eq!(bet.dealer_id, dealer_id);
        assert_eq!(bet.amount, MIN_BET);
    }

    #[test]
    fn place_bet_player_not_found() {
        let (mut ds, _, dealer_id) = setup_countdown();
        let unknown = Uuid::new_v4();
        assert_eq!(
            ds.place_bet(unknown, dealer_id, MIN_BET),
            Err(BetError::PlayerNotFound)
        );
    }

    #[test]
    fn place_bet_below_minimum() {
        let (mut ds, player_id, dealer_id) = setup_countdown();
        // MIN_BET is 1, so 0 is below minimum
        assert_eq!(
            ds.place_bet(player_id, dealer_id, 0),
            Err(BetError::BelowMinimum)
        );
    }

    #[test]
    fn place_bet_insufficient_funds() {
        let (mut ds, player_id, dealer_id) = setup_countdown();
        let over_balance = STARTING_BALANCE + 1;
        assert_eq!(
            ds.place_bet(player_id, dealer_id, over_balance),
            Err(BetError::InsufficientFunds)
        );
    }

    #[test]
    fn place_bet_wrong_game_state_waiting() {
        let mut ds = DataSource::default();
        let player_id = ds.register_player();
        let dealer_id = ds.add_game(); // starts in Waiting state
        assert_eq!(
            ds.place_bet(player_id, dealer_id, MIN_BET),
            Err(BetError::WrongGameState)
        );
    }

    #[test]
    fn place_bet_wrong_game_state_unknown_dealer() {
        let (mut ds, player_id, _) = setup_countdown();
        let unknown_dealer = Uuid::new_v4();
        assert_eq!(
            ds.place_bet(player_id, unknown_dealer, MIN_BET),
            Err(BetError::WrongGameState)
        );
    }

    #[test]
    fn place_bet_duplicate_bet() {
        let (mut ds, player_id, dealer_id) = setup_countdown();
        assert!(ds.place_bet(player_id, dealer_id, MIN_BET).is_ok());
        assert_eq!(
            ds.place_bet(player_id, dealer_id, MIN_BET),
            Err(BetError::DuplicateBet)
        );
    }

    #[test]
    fn start_game_only_deals_to_betting_players() {
        let mut ds = DataSource::default();
        let dealer_id = ds.add_game();

        // Register two players and seat them
        let p1 = ds.register_player();
        let p2 = ds.register_player();
        let hand1 = ds.add_player(dealer_id, p1).expect("p1 seated");
        let hand2 = ds.add_player(dealer_id, p2).expect("p2 seated");

        // Transition to Countdown
        ds.set_game_state_countdown(dealer_id);

        // Only p1 places a bet
        ds.place_bet(p1, dealer_id, MIN_BET).expect("bet placed");

        // Start the game
        ds.start_game(dealer_id);

        // Dealer hand is always included
        let dealer_in_sequence = ds.sequence_contains(dealer_id, dealer_id);
        assert!(dealer_in_sequence, "dealer hand must be in sequence");

        // p1 (betting) hand must be in sequence
        let p1_in_sequence = ds.sequence_contains(dealer_id, hand1);
        assert!(p1_in_sequence, "betting player hand must be in sequence");

        // p2 (non-betting) hand must NOT be in sequence
        let p2_in_sequence = ds.sequence_contains(dealer_id, hand2);
        assert!(
            !p2_in_sequence,
            "non-betting player hand must NOT be in sequence"
        );

        // p2 must have no card allocations
        let p2_cards = ds.allocations.iter().filter(|a| a.hand == hand2).count();
        assert_eq!(p2_cards, 0, "non-betting player must have no cards");

        // p1 must have 2 card allocations
        let p1_cards = ds.allocations.iter().filter(|a| a.hand == hand1).count();
        assert_eq!(p1_cards, 2, "betting player must have 2 cards");

        // active_hands must contain hand1 (betting player goes first; dealer is sorted last)
        assert!(
            ds.active_hands.contains(&hand1),
            "betting player hand must be in active_hands"
        );
        // non-betting player must not be in active_hands
        assert!(
            !ds.active_hands.contains(&hand2),
            "non-betting player hand must NOT be in active_hands"
        );
    }

    // ── apply_betting_outcomes ────────────────────────────────────────────────

    /// Helper: create a DataSource with one player (balance = `balance`), one bet
    /// of `amount`, and one outcome, then call apply_betting_outcomes.
    fn run_outcome(balance: u32, amount: u32, outcome: Outcome) -> u32 {
        let mut ds = DataSource::default();
        let player_id = Uuid::new_v4();
        let hand_id = Uuid::new_v4();
        let dealer_id = Uuid::new_v4();
        ds.players.insert(
            player_id,
            Player {
                id: player_id,
                balance,
            },
        );
        ds.bets.push(Bet {
            hand_id,
            player_id,
            dealer_id,
            amount,
        });
        ds.outcomes.push((hand_id, outcome));
        ds.apply_betting_outcomes();
        ds.players[&player_id].balance
    }

    #[test]
    fn apply_betting_outcomes_won_normal_adds_bet() {
        // Won(5) — non-blackjack: balance += bet
        let result = run_outcome(1000, 100, Outcome::Won(5));
        assert_eq!(result, 1100);
    }

    #[test]
    fn apply_betting_outcomes_won_blackjack_pays_3_to_2() {
        // Won(21) — blackjack: balance += floor(bet * 3 / 2)
        // 100 * 3 / 2 = 150
        let result = run_outcome(1000, 100, Outcome::Won(21));
        assert_eq!(result, 1150);
    }

    #[test]
    fn apply_betting_outcomes_won_blackjack_floor_odd_bet() {
        // Odd bet: 101 * 3 / 2 = 303 / 2 = 151 (integer floor)
        let result = run_outcome(1000, 101, Outcome::Won(21));
        assert_eq!(result, 1151);
    }

    #[test]
    fn apply_betting_outcomes_lost_subtracts_bet() {
        let result = run_outcome(1000, 200, Outcome::Lost(18));
        assert_eq!(result, 800);
    }

    #[test]
    fn apply_betting_outcomes_lost_at_zero_stays_zero() {
        // saturating_sub: 0 - 100 must not underflow
        let result = run_outcome(0, 100, Outcome::Lost(18));
        assert_eq!(result, 0);
    }

    #[test]
    fn apply_betting_outcomes_push_no_change() {
        let result = run_outcome(1000, 100, Outcome::Push);
        assert_eq!(result, 1000);
    }

    #[test]
    fn apply_betting_outcomes_clears_outcomes() {
        let mut ds = DataSource::default();
        let player_id = Uuid::new_v4();
        let hand_id = Uuid::new_v4();
        let dealer_id = Uuid::new_v4();
        ds.players.insert(
            player_id,
            Player {
                id: player_id,
                balance: 1000,
            },
        );
        ds.bets.push(Bet {
            hand_id,
            player_id,
            dealer_id,
            amount: 50,
        });
        ds.outcomes.push((hand_id, Outcome::Won(10)));
        ds.apply_betting_outcomes();
        assert!(
            ds.outcomes.is_empty(),
            "outcomes must be cleared after apply"
        );
    }

    #[test]
    fn reset_game_clears_round_state_and_keeps_dealer_hand() {
        let mut ds = DataSource::default();
        let game_id = ds.add_game();

        // Register two players and seat them
        let p1 = ds.register_player();
        let p2 = ds.register_player();
        ds.add_player(game_id, p1).expect("p1 seated");
        ds.add_player(game_id, p2).expect("p2 seated");

        // Transition to Countdown
        ds.set_game_state_countdown(game_id);

        // Both players place bets
        ds.place_bet(p1, game_id, MIN_BET).expect("p1 bet placed");
        ds.place_bet(p2, game_id, MIN_BET).expect("p2 bet placed");

        // Start the game to populate allocations, hand_states, sequence, active_hands
        ds.start_game(game_id);

        // Sanity: key things were populated
        assert!(!ds.allocations.is_empty());
        assert!(!ds.sequence.is_empty());
        assert!(!ds.active_hands.is_empty());
        assert!(!ds.bets.is_empty());

        // Reset the game
        ds.reset_game(game_id);

        // Dealer self-hand still present
        let dealer_hands: Vec<_> = ds
            .hands
            .iter()
            .filter(|h| h.dealer == game_id && h.id == h.dealer)
            .collect();
        assert_eq!(dealer_hands.len(), 1, "dealer self-hand must be retained");

        // No player hands remain for this table
        let player_hands: Vec<_> = ds
            .hands
            .iter()
            .filter(|h| h.dealer == game_id && h.id != h.dealer)
            .collect();
        assert!(player_hands.is_empty(), "no player hands should remain");

        // All allocations for this table (player and dealer) must be cleared
        let all_allocs = ds
            .allocations
            .iter()
            .filter(|a| a.dealer == game_id)
            .count();
        assert_eq!(all_allocs, 0, "all allocations (incl. dealer) must be cleared");
        assert!(ds.hand_states.is_empty(), "hand_states must be empty");
        assert!(ds.sequence.is_empty(), "sequence must be empty");
        assert!(ds.active_hands.is_empty(), "active_hands must be empty");
        assert!(ds.bets.is_empty(), "bets must be empty");
        assert!(ds.outcomes.is_empty(), "outcomes must be empty");

        // Deck still exists and has the expected number of cards after reshuffle
        let deck = ds.decks.get(&game_id).expect("deck must still exist");
        assert!(!deck.is_empty(), "deck must not be empty after reshuffle");

        // Game state reset to Waiting
        let state = ds.game_states.get(&game_id).expect("game state must exist");
        assert!(
            matches!(state, GameState::Waiting),
            "game state must be Waiting"
        );
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    fn all_nines_deck() -> Deck {
        (0..52)
            .map(|_| Card::new(Suit::Hearts, CardValue::Value(9)))
            .collect()
    }

    #[test]
    fn full_game_loop_two_players_push() {
        let mut ds = DataSource::default();
        let game_id = ds.add_game();
        let p1 = ds.register_player();
        let p2 = ds.register_player();
        let hand1 = ds.add_player(game_id, p1).expect("p1 seated");
        let hand2 = ds.add_player(game_id, p2).expect("p2 seated");

        // Use a rigged deck so all hands get 9+9=18 (Active on deal, no instant BlackJack/Bust)
        ds.set_deck(game_id, all_nines_deck());
        ds.set_game_state_countdown(game_id);
        ds.place_bet(p1, game_id, 100).expect("p1 bet");
        ds.place_bet(p2, game_id, 50).expect("p2 bet");
        ds.start_game(game_id);

        // After deal: all hands Active, hand_states empty, active_hands = [hand1]
        assert!(
            ds.hand_states.is_empty(),
            "no instant states on deal with all-9s deck"
        );

        // p1 holds at 18
        ds.hand_states.push((hand1, game_id, State::Holding(18)));
        ds.resolve_turn(); // advances active_hands to hand2

        // p2 holds at 18
        ds.hand_states.push((hand2, game_id, State::Holding(18)));
        ds.resolve_turn(); // advances active_hands to dealer (game_id)

        // Dealer's turn
        assert!(
            ds.active_hands.contains(&game_id),
            "active_hands should contain dealer after both players hold"
        );

        // Dealer has 18 >= 17, stands immediately
        ds.hand_states.push((game_id, game_id, State::Holding(18)));
        ds.resolve_turn(); // computes outcomes, active_hands becomes empty

        // Outcomes should be non-empty (at least p1 and p2 got Push)
        assert!(
            !ds.outcomes.is_empty(),
            "outcomes must be non-empty after dealer resolves"
        );

        let p1_outcome = ds.outcomes.iter().find(|(id, _)| *id == hand1);
        let p2_outcome = ds.outcomes.iter().find(|(id, _)| *id == hand2);
        assert!(p1_outcome.is_some(), "p1 must have an outcome");
        assert!(p2_outcome.is_some(), "p2 must have an outcome");
        assert_eq!(p1_outcome.unwrap().1, Outcome::Push);
        assert_eq!(p2_outcome.unwrap().1, Outcome::Push);

        // Record balances before payout
        let p1_balance_before = ds.players[&p1].balance;
        let p2_balance_before = ds.players[&p2].balance;

        ds.apply_betting_outcomes();

        // Push: balances unchanged
        assert_eq!(ds.players[&p1].balance, p1_balance_before);
        assert_eq!(ds.players[&p2].balance, p2_balance_before);

        // Reset
        ds.reset_game(game_id);
        assert!(ds.bets.is_empty(), "bets cleared after reset");
        assert!(ds.outcomes.is_empty(), "outcomes cleared after reset");
        assert_eq!(ds.player_count(game_id), 0, "no players after reset");
        let state = ds
            .get_game_states()
            .get(&game_id)
            .expect("game state exists");
        assert!(
            matches!(state, GameState::Waiting),
            "game state is Waiting after reset"
        );
    }

    #[test]
    fn sit_out_no_bet_player_excluded_from_deal() {
        let mut ds = DataSource::default();
        let game_id = ds.add_game();
        let p1 = ds.register_player();
        let p2 = ds.register_player();
        let hand1 = ds.add_player(game_id, p1).expect("p1 seated");
        let hand2 = ds.add_player(game_id, p2).expect("p2 seated");

        ds.set_game_state_countdown(game_id);
        ds.place_bet(p1, game_id, 100).expect("p1 bet");
        // p2 does NOT place a bet

        ds.start_game(game_id);

        // p2's hand must not be in active_hands
        assert!(
            !ds.active_hands.contains(&hand2),
            "non-betting player hand must not be in active_hands"
        );

        // p2's hand must have no card allocations
        let p2_cards = ds.allocations.iter().filter(|a| a.hand == hand2).count();
        assert_eq!(p2_cards, 0, "non-betting player must have no cards");

        // p1's hand must be in active_hands (betting player goes first)
        assert!(
            ds.active_hands.contains(&hand1),
            "betting player hand must be in active_hands"
        );
    }

    #[test]
    fn zero_balance_player_cannot_bet() {
        let mut ds = DataSource::default();
        let game_id = ds.add_game();
        let p1 = ds.register_player();
        ds.add_player(game_id, p1).expect("p1 seated");

        // Drain the player's balance
        ds.players.get_mut(&p1).unwrap().balance = 0;

        ds.set_game_state_countdown(game_id);

        let result = ds.place_bet(p1, game_id, 1);
        assert_eq!(result, Err(BetError::InsufficientFunds));
    }

    #[test]
    fn push_outcome_leaves_balance_unchanged() {
        let mut ds = DataSource::default();
        let game_id = ds.add_game();
        let p1 = ds.register_player();
        let hand1 = ds.add_player(game_id, p1).expect("p1 seated");

        // Rig the deck so no instant BlackJack/Bust on deal
        ds.set_deck(game_id, all_nines_deck());
        ds.set_game_state_countdown(game_id);
        ds.place_bet(p1, game_id, 100).expect("p1 bet");
        ds.start_game(game_id);

        // Directly push both hand states (bypasses card allocation)
        ds.hand_states.push((hand1, game_id, State::Holding(18)));
        ds.hand_states.push((game_id, game_id, State::Holding(18)));
        ds.resolve_turn();

        // Assert Push outcome for p1
        let p1_outcome = ds
            .outcomes
            .iter()
            .find(|(id, _)| *id == hand1)
            .expect("p1 must have an outcome");
        assert_eq!(p1_outcome.1, Outcome::Push);

        let initial_balance = ds.players[&p1].balance;
        ds.apply_betting_outcomes();
        assert_eq!(
            ds.players[&p1].balance, initial_balance,
            "Push must not change balance"
        );
    }

    #[test]
    fn blackjack_payout_is_floor_bet_times_3_over_2() {
        let mut ds = DataSource::default();
        let game_id = ds.add_game();
        let p1 = ds.register_player();
        let hand1 = ds.add_player(game_id, p1).expect("p1 seated");

        // Rig the deck so no instant BlackJack on deal (all 9s → value 18, Active)
        ds.set_deck(game_id, all_nines_deck());
        ds.set_game_state_countdown(game_id);
        ds.place_bet(p1, game_id, 100).expect("p1 bet");
        ds.start_game(game_id);

        // Directly push BlackJack for player and Holding(17) for dealer
        ds.hand_states.push((hand1, game_id, State::BlackJack));
        ds.hand_states.push((game_id, game_id, State::Holding(17)));
        ds.resolve_turn();

        // Assert Won(21) outcome for p1
        let p1_outcome = ds
            .outcomes
            .iter()
            .find(|(id, _)| *id == hand1)
            .expect("p1 must have an outcome");
        assert_eq!(p1_outcome.1, Outcome::Won(21));

        let initial_balance = ds.players[&p1].balance;
        ds.apply_betting_outcomes();
        // floor(100 * 3 / 2) = 150
        assert_eq!(
            ds.players[&p1].balance,
            initial_balance + 150,
            "Blackjack payout must be floor(bet * 3 / 2) = 150"
        );
    }
}
