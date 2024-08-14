use std::collections::HashMap;
use uuid::Uuid;

use crate::types::*;

pub fn get_dealer(hand_id: Uuid, hands: &[Hand]) -> Uuid {
    hands
        .iter()
        .find(|h| hand_id == h.id)
        .expect("Unable to find Hand")
        .dealer
}

pub fn get_game(hand_id: Uuid, hands: &[Hand]) -> Uuid {
    get_dealer(hand_id, hands)
}

pub fn get_hand_count(game_id: Uuid, hands: &[Hand]) -> usize {
    hands.iter().filter(|&h| h.dealer == game_id).count()
}

pub fn get_active_hand(game_id: Uuid, active_hands: &[Uuid], hands: &[Hand]) -> Option<Uuid> {
    active_hands
        .iter()
        .filter_map(|id| hands.iter().find(|h| h.id == *id))
        .find(|h| h.dealer == game_id)
        .map(|h| h.id)
}

pub fn get_hand_value(
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
        .map(|a| &deck[a.card_idx])
        .collect::<Vec<_>>();
    hand_value(&cards)
}

fn hand_value(cards: &[&Card]) -> u8 {
    let mut ace_count = 0;
    let mut value = cards
        .iter()
        .map(|c| match c.1 {
            Value::Value(v) => v,
            Value::Ace => {
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

pub type Deck = Vec<Card>;

pub fn new_deck() -> Deck {
    vec![
        (Suit::Hearts, Value::Ace),
        (Suit::Hearts, Value::King),
        (Suit::Hearts, Value::Queen),
        (Suit::Hearts, Value::Jack),
        (Suit::Hearts, Value::Value(10)),
        (Suit::Hearts, Value::Value(9)),
        (Suit::Hearts, Value::Value(8)),
        (Suit::Hearts, Value::Value(7)),
        (Suit::Hearts, Value::Value(6)),
        (Suit::Hearts, Value::Value(5)),
        (Suit::Hearts, Value::Value(4)),
        (Suit::Hearts, Value::Value(3)),
        (Suit::Hearts, Value::Value(2)),
        (Suit::Hearts, Value::Value(1)),
        (Suit::Diamonds, Value::Ace),
        (Suit::Diamonds, Value::King),
        (Suit::Diamonds, Value::Queen),
        (Suit::Diamonds, Value::Jack),
        (Suit::Diamonds, Value::Value(10)),
        (Suit::Diamonds, Value::Value(9)),
        (Suit::Diamonds, Value::Value(8)),
        (Suit::Diamonds, Value::Value(7)),
        (Suit::Diamonds, Value::Value(6)),
        (Suit::Diamonds, Value::Value(5)),
        (Suit::Diamonds, Value::Value(4)),
        (Suit::Diamonds, Value::Value(3)),
        (Suit::Diamonds, Value::Value(2)),
        (Suit::Diamonds, Value::Value(1)),
        (Suit::Clubs, Value::Ace),
        (Suit::Clubs, Value::King),
        (Suit::Clubs, Value::Queen),
        (Suit::Clubs, Value::Jack),
        (Suit::Clubs, Value::Value(10)),
        (Suit::Clubs, Value::Value(9)),
        (Suit::Clubs, Value::Value(8)),
        (Suit::Clubs, Value::Value(7)),
        (Suit::Clubs, Value::Value(6)),
        (Suit::Clubs, Value::Value(5)),
        (Suit::Clubs, Value::Value(4)),
        (Suit::Clubs, Value::Value(3)),
        (Suit::Clubs, Value::Value(2)),
        (Suit::Clubs, Value::Value(1)),
        (Suit::Spades, Value::Ace),
        (Suit::Spades, Value::King),
        (Suit::Spades, Value::Queen),
        (Suit::Spades, Value::Jack),
        (Suit::Spades, Value::Value(10)),
        (Suit::Spades, Value::Value(9)),
        (Suit::Spades, Value::Value(8)),
        (Suit::Spades, Value::Value(7)),
        (Suit::Spades, Value::Value(6)),
        (Suit::Spades, Value::Value(5)),
        (Suit::Spades, Value::Value(4)),
        (Suit::Spades, Value::Value(3)),
        (Suit::Spades, Value::Value(2)),
        (Suit::Spades, Value::Value(1)),
    ]
}

pub fn is_hand_active(hand_id: Uuid, hand_states: &[HandState]) -> bool {
    hand_states.iter().find(|&hs| hs.0 == hand_id).is_none()
}

pub fn allocate_cards(
    hands: &[Hand],
    allocations: &[CardAllocation],
    game_id: Uuid,
) -> Vec<CardAllocation> {
    // Find the current card index into the deck
    let card_idx = allocations.iter().filter(|a| a.dealer == game_id).count();

    // Every hard in the game gets allocated a card
    hands
        .iter()
        .filter(|hand| hand.dealer == game_id)
        .enumerate()
        .map(|(idx, hand)| {
            println!("Adding card allocation: {},{}", hand.id, card_idx + idx);
            CardAllocation {
                card_idx: card_idx + idx,
                dealer: game_id,
                hand: hand.id,
            }
        })
        .collect::<Vec<_>>()
}

pub enum ActionResolutionError {
    MissingHand,
    MissingDeck,
}

//@note: Making the assumpting here that there is not going to be any actions
//       in this list that are going to end up hitting the same deck and therefore
//       invalidating the number deck index calculated from the allocations list.
pub fn process_hit_actions(
    actions: &[HandAction],
    hands: &[Hand],
    allocations: &[CardAllocation],
) -> Vec<CardAllocation> {
    //println!("Processing Hit Actions");
    actions
        .iter()
        .filter(|(_, action)| matches!(action, Action::Hit))
        .filter_map(|(hand_id, _)| hands.iter().find(|hand| hand.id == *hand_id))
        .map(|hand| {
            let card_idx = allocations
                .iter()
                .filter(|a| a.dealer == hand.dealer)
                .count();
            println!("Adding card allocation: {},{}", hand.id, card_idx);
            CardAllocation {
                card_idx,
                dealer: hand.dealer,
                hand: hand.id,
            }
        })
        .collect::<Vec<_>>()
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
            .map(|a| &deck[a.card_idx])
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

pub fn process_hold_actions(
    hands: &[Hand],
    actions: &[HandAction],
    allocations: &[CardAllocation],
    decks: &HashMap<Uuid, Deck>,
) -> Vec<HandState> {
    actions
        .iter()
        .filter(|(_, action)| matches!(action, Action::Hold))
        .map(|(hand, _)| {
            // Grab the deck for the hand.
            let dealer_id = get_dealer(*hand, hands);
            let deck = decks
                .get(&dealer_id)
                .expect("Unable to find deck for table");

            // Calculate the hand value
            let cards = allocations
                .iter()
                .filter(|a| a.hand == *hand)
                .map(|a| &deck[a.card_idx])
                .collect::<Vec<_>>();
            let value = hand_value(&cards);

            //@todo: I dont know if I need to check if the hand is blackJack or Bust or anything
            //here.

            (*hand, dealer_id, State::Holding(value))
        })
        .collect::<Vec<_>>()
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

pub fn is_game_complete(dealer: Uuid, hands: &[Hand], hand_states: &[HandState]) -> bool {
    // A game is complete if all of the hands associated with it have HandState's.
    let hand_count = hands.iter().filter(|h| dealer == h.dealer).count();
    let state_count = hand_states.iter().filter(|hs| dealer == hs.1).count();
    hand_count == state_count
}

// @todo:  I guess we need to keep this "clone" but I dont like it.
pub fn get_hand_outcome(hand_id: Uuid, outcomes: &[HandOutcome]) -> Option<Outcome> {
    outcomes
        .iter()
        .find(|o| o.0 == hand_id)
        .map(|o| o.1.clone())
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
