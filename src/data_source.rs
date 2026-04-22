use log::warn;
use std::collections::HashMap;
use uuid::Uuid;

use crate::types::*;
use crate::utils::*;

#[derive(Debug, Clone)]
pub enum GameState {
    Waiting,
    Active,
    Finished,
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
}

impl DataSource {
    pub fn add_game(&mut self) -> Uuid {
        let dealer_id = Uuid::new_v4();
        self.decks.insert(dealer_id, new_deck());
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
    pub fn add_player(&mut self, dealer_id: Uuid) -> Uuid {
        let player_id = Uuid::new_v4();
        self.hands.push(Hand {
            id: player_id,
            player: player_id,
            dealer: dealer_id,
        });

        player_id
    }

    pub fn allocate_cards(&mut self, hands: &[Hand], count: usize) -> Vec<CardAllocation> {
        hands
            .iter()
            .flat_map(|h| draw_cards(h, &self.allocations, count))
            .collect()
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
        }
    }

    pub fn start_game(&mut self, game_id: Uuid) {
        // Grab the hands for the given game.
        let hands = self
            .hands
            .iter()
            .filter(|h| h.dealer == game_id)
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

        // Determine turn order, currently just extracts all of the hands associated with a dealer.
        let mut sequence = self
            .hands
            .iter()
            .filter(|h| h.dealer == game_id)
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
}
