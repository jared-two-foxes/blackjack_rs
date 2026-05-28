use std::collections::HashMap;
use uuid::Uuid;

use rand::seq::SliceRandom;
use rand::thread_rng;

use crate::types::*;

pub fn get_dealer(hand_id: Uuid, hands: &[Hand]) -> Uuid {
    hands
        .iter()
        .find(|h| hand_id == h.id)
        .expect("Unable to find Hand")
        .dealer
}

pub fn get_active_hand(game_id: Uuid, active_hands: &[Uuid], hands: &[Hand]) -> Option<Uuid> {
    active_hands
        .iter()
        .filter_map(|id| hands.iter().find(|h| h.id == *id))
        .find(|h| h.dealer == game_id)
        .map(|h| h.id)
}

/*pub fn get_hand_value(
    hand_id: Uuid,
    hands: &[Hand],
    allocations: &[CardAllocation],
    decks: &HashMap<Uuid, Deck>,
) -> u8 {
    let dealer = get_dealer(hand_id, hands);
    let deck = decks.get(&dealer).expect("Unable to find deck");
    let cards = allocations
        .iter()
        .filter(|a| a.hand == hand_id)
        .clone()
        .map(|a| &deck[a.card_idx])
        .collect::<Vec<_>>();

    trace!("cards in hand: {:?}", cards);

    hand_value(&cards)
}*/

pub fn hand_value(cards: &[Card]) -> u8 {
    let mut ace_count = 0;
    let mut value = cards
        .iter()
        .map(|c| match c.value {
            CardValue::Value(v) => v,
            CardValue::Ace => {
                ace_count += 1;
                11
            }
            _ => 10,
        })
        .sum::<u8>();
    for _ in 0..ace_count {
        if value > 21 {
            value -= 10;
        }
    }
    value
}

pub fn new_deck() -> Deck {
    vec![
        Card::new(Suit::Hearts, CardValue::Ace),
        Card::new(Suit::Hearts, CardValue::King),
        Card::new(Suit::Hearts, CardValue::Queen),
        Card::new(Suit::Hearts, CardValue::Jack),
        Card::new(Suit::Hearts, CardValue::Value(10)),
        Card::new(Suit::Hearts, CardValue::Value(9)),
        Card::new(Suit::Hearts, CardValue::Value(8)),
        Card::new(Suit::Hearts, CardValue::Value(7)),
        Card::new(Suit::Hearts, CardValue::Value(6)),
        Card::new(Suit::Hearts, CardValue::Value(5)),
        Card::new(Suit::Hearts, CardValue::Value(4)),
        Card::new(Suit::Hearts, CardValue::Value(3)),
        Card::new(Suit::Hearts, CardValue::Value(2)),
        Card::new(Suit::Hearts, CardValue::Value(1)),
        Card::new(Suit::Diamonds, CardValue::Ace),
        Card::new(Suit::Diamonds, CardValue::King),
        Card::new(Suit::Diamonds, CardValue::Queen),
        Card::new(Suit::Diamonds, CardValue::Jack),
        Card::new(Suit::Diamonds, CardValue::Value(10)),
        Card::new(Suit::Diamonds, CardValue::Value(9)),
        Card::new(Suit::Diamonds, CardValue::Value(8)),
        Card::new(Suit::Diamonds, CardValue::Value(7)),
        Card::new(Suit::Diamonds, CardValue::Value(6)),
        Card::new(Suit::Diamonds, CardValue::Value(5)),
        Card::new(Suit::Diamonds, CardValue::Value(4)),
        Card::new(Suit::Diamonds, CardValue::Value(3)),
        Card::new(Suit::Diamonds, CardValue::Value(2)),
        Card::new(Suit::Diamonds, CardValue::Value(1)),
        Card::new(Suit::Clubs, CardValue::Ace),
        Card::new(Suit::Clubs, CardValue::King),
        Card::new(Suit::Clubs, CardValue::Queen),
        Card::new(Suit::Clubs, CardValue::Jack),
        Card::new(Suit::Clubs, CardValue::Value(10)),
        Card::new(Suit::Clubs, CardValue::Value(9)),
        Card::new(Suit::Clubs, CardValue::Value(8)),
        Card::new(Suit::Clubs, CardValue::Value(7)),
        Card::new(Suit::Clubs, CardValue::Value(6)),
        Card::new(Suit::Clubs, CardValue::Value(5)),
        Card::new(Suit::Clubs, CardValue::Value(4)),
        Card::new(Suit::Clubs, CardValue::Value(3)),
        Card::new(Suit::Clubs, CardValue::Value(2)),
        Card::new(Suit::Clubs, CardValue::Value(1)),
        Card::new(Suit::Spades, CardValue::Ace),
        Card::new(Suit::Spades, CardValue::King),
        Card::new(Suit::Spades, CardValue::Queen),
        Card::new(Suit::Spades, CardValue::Jack),
        Card::new(Suit::Spades, CardValue::Value(10)),
        Card::new(Suit::Spades, CardValue::Value(9)),
        Card::new(Suit::Spades, CardValue::Value(8)),
        Card::new(Suit::Spades, CardValue::Value(7)),
        Card::new(Suit::Spades, CardValue::Value(6)),
        Card::new(Suit::Spades, CardValue::Value(5)),
        Card::new(Suit::Spades, CardValue::Value(4)),
        Card::new(Suit::Spades, CardValue::Value(3)),
        Card::new(Suit::Spades, CardValue::Value(2)),
        Card::new(Suit::Spades, CardValue::Value(1)),
    ]
}

/// Returns the number of seconds remaining in a countdown phase.
/// Returns 0 if the countdown has already elapsed.
pub fn countdown_seconds_remaining(started_at: &std::time::Instant) -> u64 {
    crate::data_source::effective_countdown_secs().saturating_sub(started_at.elapsed().as_secs())
}

/// Shuffles a deck in-place using a thread-local RNG.
pub fn shuffle_deck(deck: &mut Deck) {
    deck.shuffle(&mut thread_rng());
}

pub fn is_hand_active(hand_id: Uuid, hand_states: &[HandState]) -> bool {
    !hand_states.iter().any(|hs| hs.0 == hand_id)
}

//@note:
//  This function is actually not general enough.  Hands needs to be a list of hands
//  which can be from multiple different games and it should take a number of cards to
//  allocate to each hand.  It might also need an allocation strategy like sequential or
//  iterative
//@note:
//  It would also be nice if we had a way to express that a given parameter was the
//  full list of a given thing, ie in this instance allocations is a list of ALL of
//  the current allocations in this universe.
//@note:
//  Ive decided these are the wrong way around, at the time of writing this function
//  accepted a list of hands and allocated new cards to each hand, this is backwards
//  as to do so it would require passing the DataSource here and calling allocate
//  on each hand but I  believe these functions should be DataSource ignorant, so
//  the need to reverse the implementations
//@note:
//  This currently doesnt reference the deck to which the cards are being drawn from
//  so we dont actually know if we've 'decked' or not.
pub fn draw_cards(
    hand: &Hand,
    allocations: &[CardAllocation],
    count: usize,
) -> Vec<CardAllocation> {
    // Find the current card index into the deck, calculated by grabbing all of
    // the allocated cards and counting them
    let card_idx = allocations
        .iter()
        .filter(|a| a.dealer == hand.dealer)
        .count();

    // for each card that is to be allocated, created a CardAllocation and return
    (0..count)
        .map(|i| CardAllocation {
            card_idx: card_idx + i,
            dealer: hand.dealer,
            hand: hand.id,
        })
        .collect()
}

pub fn process_hand_states(
    hands: &[Hand],
    card_allocations: &[CardAllocation],
    decks: &HashMap<Uuid, Deck>,
) -> Vec<HandState> {
    let mut hand_states = Vec::new();
    for h in hands {
        let deck = decks.get(&h.dealer).expect("Unable to find deck for table");
        //@note: its probably faster to just build the hand values by iterating this once and building
        //  it as we go foldish style and then map that into a hand_state rather than iterate all the
        //  allocations for each hand like this.
        let cards = card_allocations
            .iter()
            .filter(|a| a.hand == h.id)
            .map(|a| deck[a.card_idx].clone())
            .collect::<Vec<_>>();

        //@note: its probably better to just not add the actives here rather than strip them out later.
        let hand_value = hand_value(&cards);
        let state = match hand_value {
            0..=20 => State::Active,
            21 => State::BlackJack,
            _ => State::Bust(hand_value),
        };
        hand_states.push((h.id, h.dealer, state));
    }
    // and strip out all the Active's because we dont want to report those.
    hand_states
        .into_iter()
        .filter(|(_, _, hs)| !matches!(hs, State::Active))
        .collect()
}

// Iterate all of the HandStates in hand_state, for any HandState for which there is a corosponding
// dealer HandState determine the HandOutcome and return it.
pub fn resolve_outcomes(hand_values: &[HandState], outcomes: &[HandOutcome]) -> Vec<HandOutcome> {
    hand_values
        .iter()
        // Check if this particular hand already exists within the outcomes list
        .filter(|h| !outcomes.iter().any(|o| o.0 == h.0))
        // Grab the dealer value and if it exist return (hand, dealer) pair,
        // filter out this hand if the dealer state does not exist.
        .filter_map(|h| hand_values.iter().find(|hv| hv.0 == h.1).map(|d| (h, d)))
        // And finally lets determine the outcome.
        .map(|(h, d)| {
            let state = match d.2 {
                State::BlackJack => Outcome::Lost(0),
                State::Bust(_) => Outcome::Won(22),
                State::Holding(dealer_value) => {
                    match h.2 {
                        State::BlackJack => Outcome::Won(21),
                        State::Bust(v) => Outcome::Lost(v),
                        State::Holding(v) => {
                            if v > dealer_value {
                                Outcome::Won(v)
                            } else if v == dealer_value {
                                Outcome::Push
                            } else {
                                Outcome::Lost(v)
                            }
                        }
                        _ => {
                            unreachable!("Have reached State::Active for a hand while resolving hand outcomes")
                        }
                    }
                },
               _ => unreachable!("The dealer's hand is still active while attempting to resolve the hand outcomes")
            };
            (h.0, state)
        })
        .collect::<_>()
}

fn get_game(current_hand_id: uuid::Uuid, hands: &[Hand]) -> uuid::Uuid {
    hands
        .iter()
        .find(|h| h.id == current_hand_id)
        .expect("passed an invalid current_hand_id")
        .dealer
}

// @todo: this should really be receiving the game_id rather than the turn_order and
// current_hand_idx as both of those should be stored in the Source.
pub fn determine_next_hand(
    current_hand_id: Uuid,
    turn_order: &[Sequence],
    hands: &[Hand],
    hand_states: &[HandState],
) -> Option<Uuid> {
    let game_id = get_game(current_hand_id, hands);
    //is_hand_active(current_hand_id, hands);
    turn_order
        .iter()
        .cycle()
        // We care only for our own game, so filter out the rest
        .filter(|&s| s.game_id == game_id)
        // Fast forward to the current hand
        .skip_while(|&s| s.hand_id != current_hand_id)
        // Skip this hand, since we're trying to find the next good active hand
        .skip(1)
        // And now iterate from here until we find a hand that is active
        .find(|&s| is_hand_active(s.hand_id, hand_states) || s.hand_id == current_hand_id)
        .map(|s| s.hand_id)
        .and_then(|uid| is_hand_active(uid, hand_states).then_some(uid))
}

// turn sequence; the order in which players take turns (with the dealer going last)
// action validation; an action is only balid if its that players turn to go.
// Rules engine?

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_outcomes_push_when_equal_values() {
        let dealer_id = Uuid::new_v4();
        let player_id = Uuid::new_v4();
        // Both dealer and player holding 17
        let hand_states: Vec<HandState> = vec![
            (dealer_id, dealer_id, State::Holding(17)), // dealer's self-hand
            (player_id, dealer_id, State::Holding(17)), // player's hand
        ];
        let outcomes = resolve_outcomes(&hand_states, &[]);
        // The dealer's self-hand also resolves (push against itself), so we get 2 outcomes.
        assert_eq!(outcomes.len(), 2);
        let player_outcome = outcomes
            .iter()
            .find(|(id, _)| *id == player_id)
            .expect("player outcome missing");
        assert!(matches!(player_outcome.1, Outcome::Push));
    }

    #[test]
    fn resolve_outcomes_no_push_when_player_wins() {
        let dealer_id = Uuid::new_v4();
        let player_id = Uuid::new_v4();
        let hand_states: Vec<HandState> = vec![
            (dealer_id, dealer_id, State::Holding(16)),
            (player_id, dealer_id, State::Holding(19)),
        ];
        let outcomes = resolve_outcomes(&hand_states, &[]);
        // 2 outcomes: dealer self-hand (push) + player hand (won)
        assert_eq!(outcomes.len(), 2);
        let player_outcome = outcomes
            .iter()
            .find(|(id, _)| *id == player_id)
            .expect("player outcome missing");
        assert!(matches!(player_outcome.1, Outcome::Won(_)));
    }

    #[test]
    fn resolve_outcomes_lost_when_dealer_wins() {
        let dealer_id = Uuid::new_v4();
        let player_id = Uuid::new_v4();
        let hand_states: Vec<HandState> = vec![
            (dealer_id, dealer_id, State::Holding(20)),
            (player_id, dealer_id, State::Holding(17)),
        ];
        let outcomes = resolve_outcomes(&hand_states, &[]);
        // 2 outcomes: dealer self-hand (push) + player hand (lost)
        assert_eq!(outcomes.len(), 2);
        let player_outcome = outcomes
            .iter()
            .find(|(id, _)| *id == player_id)
            .expect("player outcome missing");
        assert!(matches!(player_outcome.1, Outcome::Lost(_)));
    }

    #[test]
    fn shuffle_deck_changes_order() {
        let original = new_deck();
        let mut shuffled = new_deck();
        shuffle_deck(&mut shuffled);
        // A 52-card Fisher-Yates shuffle virtually never produces the identity permutation.
        // Compare by Debug representation since Card doesn't implement PartialEq.
        let original_repr: Vec<String> = original.iter().map(|c| format!("{:?}", c)).collect();
        let shuffled_repr: Vec<String> = shuffled.iter().map(|c| format!("{:?}", c)).collect();
        assert_ne!(
            original_repr, shuffled_repr,
            "shuffled deck should differ from the original ordering"
        );
    }
}
