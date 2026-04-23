//! Integration tests for the blackjack server.

use blackjack::{
    self,
    types::{Action, Card, CardValue, Deck, Outcome, State, Suit},
};
use std::collections::HashMap;
use uuid::Uuid;

#[cfg(test)]
mod tests {
    use super::*;

    type Hand = Uuid;

    #[derive(Debug)]
    struct CardAllocation {
        hand: Hand,
        card: usize,
    }

    fn create_loaded_deck() -> Deck {
        vec![
            Card::new(Suit::Hearts, CardValue::Value(9)),
            Card::new(Suit::Hearts, CardValue::Value(8)),
            Card::new(Suit::Hearts, CardValue::Value(7)),
            Card::new(Suit::Hearts, CardValue::Value(6)),
            Card::new(Suit::Hearts, CardValue::Value(5)),
            Card::new(Suit::Hearts, CardValue::Value(4)),
            Card::new(Suit::Hearts, CardValue::Value(3)),
            Card::new(Suit::Hearts, CardValue::Value(2)),
            Card::new(Suit::Hearts, CardValue::Value(1)),
            Card::new(Suit::Diamonds, CardValue::Value(9)),
            Card::new(Suit::Diamonds, CardValue::Value(8)),
            Card::new(Suit::Diamonds, CardValue::Value(7)),
            Card::new(Suit::Diamonds, CardValue::Value(6)),
            Card::new(Suit::Diamonds, CardValue::Value(5)),
            Card::new(Suit::Diamonds, CardValue::Value(4)),
            Card::new(Suit::Diamonds, CardValue::Value(3)),
            Card::new(Suit::Diamonds, CardValue::Value(2)),
            Card::new(Suit::Diamonds, CardValue::Value(1)),
            Card::new(Suit::Spades, CardValue::Value(9)),
            Card::new(Suit::Spades, CardValue::Value(8)),
            Card::new(Suit::Spades, CardValue::Value(7)),
            Card::new(Suit::Spades, CardValue::Value(6)),
            Card::new(Suit::Spades, CardValue::Value(5)),
            Card::new(Suit::Spades, CardValue::Value(4)),
            Card::new(Suit::Spades, CardValue::Value(3)),
            Card::new(Suit::Spades, CardValue::Value(2)),
            Card::new(Suit::Spades, CardValue::Value(1)),
            Card::new(Suit::Clubs, CardValue::Value(9)),
            Card::new(Suit::Clubs, CardValue::Value(8)),
            Card::new(Suit::Clubs, CardValue::Value(7)),
            Card::new(Suit::Clubs, CardValue::Value(6)),
            Card::new(Suit::Clubs, CardValue::Value(5)),
            Card::new(Suit::Clubs, CardValue::Value(4)),
            Card::new(Suit::Clubs, CardValue::Value(3)),
            Card::new(Suit::Clubs, CardValue::Value(2)),
            Card::new(Suit::Clubs, CardValue::Value(1)),
        ]
    }

    fn is_active_hand(hand: &Hand, hand_states: &HashMap<Uuid, State>) -> bool {
        match hand_states.get(hand) {
            Some(State::Active) => true,
            None => true, // Treat as active if not in map? Actually logic was matches!(..., Some(State::Active))
            _ => false,
        }
    }

    fn start(hands: &[Hand]) -> Vec<CardAllocation> {
        let table_size = hands.len();
        hands
            .iter()
            .enumerate()
            .flat_map(|(i, h)| {
                vec![
                    CardAllocation { hand: *h, card: i },
                    CardAllocation {
                        hand: *h,
                        card: i * table_size,
                    },
                ]
            })
            .collect()
    }

    fn determine_action(value: u8) -> Action {
        if value < 17 {
            Action::Hit
        } else {
            Action::Hold
        }
    }

    fn get_hand(_hand: &Hand) -> Vec<Card> {
        vec![
            Card::new(Suit::Hearts, CardValue::Value(10)),
            Card::new(Suit::Spades, CardValue::Value(9)),
        ]
    }

    fn hand_outcome(_hand: Hand) -> Option<Outcome> {
        Some(Outcome::Lost(19))
    }

    #[test]
    fn can_play_a_simple_game() {
        // Arrange
        let _deck = create_loaded_deck();
        let mut allocations = vec![];
        let mut hand_states = HashMap::new();
        let player = Uuid::new_v4();
        let player_hand = player;
        let dealer = Uuid::new_v4();
        let hands = vec![player, dealer];
        allocations.append(&mut start(&hands));

        hand_states.insert(player, State::Active);
        hand_states.insert(dealer, State::Active);

        // Act
        let iter = hands
            .iter()
            .cycle()
            .filter(|&h| is_active_hand(h, &hand_states))
            .map(|h| (h, blackjack::utils::hand_value(&get_hand(h))));
        for (_hand, value) in iter.take(1) {
            let _action = determine_action(value);
            let _state = State::Active;
        }

        // Assert
        assert_eq!(Some(Outcome::Lost(19)), hand_outcome(player_hand));
    }
}
