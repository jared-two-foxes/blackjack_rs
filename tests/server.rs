//
// Integration tests for our blackjack server
//
use blackjack::{Action, DataSource};
use std::collections::HashMap;
use uuid::Uuid;

//@todo:
//  Figure out logging, when where why how
//  Game should end if all the players go bust or blackjack before the dealer draws
//  Game should end and all active players should win if the dealer busts

fn create_loaded_deck() -> blackjack::Deck {
    //@note: For now just going to create a deck with all of the face cards
    //  removed
    vec![
        (blackjack::Suit::Hearts, blackjack::Value::Value(9)),
        (blackjack::Suit::Hearts, blackjack::Value::Value(8)),
        (blackjack::Suit::Hearts, blackjack::Value::Value(7)),
        (blackjack::Suit::Hearts, blackjack::Value::Value(6)),
        (blackjack::Suit::Hearts, blackjack::Value::Value(5)),
        (blackjack::Suit::Hearts, blackjack::Value::Value(4)),
        (blackjack::Suit::Hearts, blackjack::Value::Value(3)),
        (blackjack::Suit::Hearts, blackjack::Value::Value(2)),
        (blackjack::Suit::Hearts, blackjack::Value::Value(1)),
        (blackjack::Suit::Diamonds, blackjack::Value::Value(9)),
        (blackjack::Suit::Diamonds, blackjack::Value::Value(8)),
        (blackjack::Suit::Diamonds, blackjack::Value::Value(7)),
        (blackjack::Suit::Diamonds, blackjack::Value::Value(6)),
        (blackjack::Suit::Diamonds, blackjack::Value::Value(5)),
        (blackjack::Suit::Diamonds, blackjack::Value::Value(4)),
        (blackjack::Suit::Diamonds, blackjack::Value::Value(3)),
        (blackjack::Suit::Diamonds, blackjack::Value::Value(2)),
        (blackjack::Suit::Diamonds, blackjack::Value::Value(1)),
        (blackjack::Suit::Spades, blackjack::Value::Value(9)),
        (blackjack::Suit::Spades, blackjack::Value::Value(8)),
        (blackjack::Suit::Spades, blackjack::Value::Value(7)),
        (blackjack::Suit::Spades, blackjack::Value::Value(6)),
        (blackjack::Suit::Spades, blackjack::Value::Value(5)),
        (blackjack::Suit::Spades, blackjack::Value::Value(4)),
        (blackjack::Suit::Spades, blackjack::Value::Value(3)),
        (blackjack::Suit::Spades, blackjack::Value::Value(2)),
        (blackjack::Suit::Spades, blackjack::Value::Value(1)),
        (blackjack::Suit::Clubs, blackjack::Value::Value(9)),
        (blackjack::Suit::Clubs, blackjack::Value::Value(8)),
        (blackjack::Suit::Clubs, blackjack::Value::Value(7)),
        (blackjack::Suit::Clubs, blackjack::Value::Value(6)),
        (blackjack::Suit::Clubs, blackjack::Value::Value(5)),
        (blackjack::Suit::Clubs, blackjack::Value::Value(4)),
        (blackjack::Suit::Clubs, blackjack::Value::Value(3)),
        (blackjack::Suit::Clubs, blackjack::Value::Value(2)),
        (blackjack::Suit::Clubs, blackjack::Value::Value(1)),
    ]
}

// import our lib and setup a game
#[test]
fn can_play_a_simple_game() {
    let mut ds = DataSource::default();
    let game_id = ds.add_game();
    ds.set_deck(game_id, create_loaded_deck());
    let hand_id = ds.add_player(game_id);

    // Need to determine the turn sequence somehow.
    // @todo: Turn order needs to get migrated into the DataSource as we need to have one of these
    // per game.
    let turn_order = vec![hand_id, game_id];
    let mut current_hand_idx: usize = 0;

    ds.start_game(game_id);

    // Building out a single "round", where a round involves reacting to a
    // collection of hand actions and then detrrmining the results for each
    // hand that was acted upon
    while !blackjack::is_game_complete(game_id, &ds.hands, &ds.hand_states) {
        let current_hand = turn_order[current_hand_idx];
        let hand_value =
            blackjack::get_hand_value(current_hand, &ds.hands, &ds.allocations, &ds.decks);

        //println!("processing for hand: {} ({})", current_hand_idx, hand_value);

        if current_hand == hand_id {
            ds.add_action(hand_id, Action::Hit);
        } else {
            // This is the dealers turn.
            if hand_value > 17 {
                ds.add_action(current_hand, Action::Hold);
            } else {
                ds.add_action(current_hand, Action::Hit);
            }
        }

        // Process the hand actions
        ds.process_hit_actions();
        ds.process_hold_actions();

        // Get the next hands to act
        current_hand_idx = ds.get_next_hand(current_hand_idx, &turn_order);

        //println!(); //< Add blankline to seperate iterations
        ds.actions.clear();
    }

    // determine the outcome of the game.
    let outcomes = blackjack::resolve_outcomes(&ds.hand_states, &ds.outcomes);
    ds.outcomes.extend(outcomes);

    assert_eq!(
        Some(blackjack::Outcome::Won(21)),
        blackjack::get_hand_outcome(hand_id, &ds.outcomes)
    );
}
