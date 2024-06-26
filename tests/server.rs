//
// Integration tests for our blackjack server
//
use blackjack::{Action, DataSource};

// import our lib and setup a game
#[test]
fn can_play_a_simple_game() {
    let mut ds = DataSource::default();
    let game_id = blackjack::add_game(&mut ds);
    //@todo: swap in a loaded deck
    let hand_id = blackjack::add_player(&mut ds, game_id);

    // Need to determine the turn sequence somehow.
    let turn_order = vec![hand_id, game_id];
    let mut current_player_idx: usize = 0;
    const PLAYER_COUNT: usize = 2;

    // Building out a single "round", where a round involves reacting to a
    // collection of hand actions and then detrrmining the results for each
    // hand that was acted upon
    while blackjack::is_game_complete(game_id, &ds.hands, &ds.outcomes) {
        let current_hand = turn_order[current_player_idx];

        //@todo: If current_player has a hand outcome then they dont get to act again, just loop to
        //the next player.

        if current_hand == hand_id {
            blackjack::add_action(&mut ds, hand_id, Action::Hit);
        } else {
            // This is the dealers turn.
            let hand_value =
                blackjack::get_hand_value(current_hand, &ds.hands, &ds.allocations, &ds.decks);
            if hand_value > 17 {
                blackjack::add_action(&mut ds, current_hand, Action::Hold);
            } else {
                blackjack::add_action(&mut ds, current_hand, Action::Hit);
            }
        }

        current_player_idx = (current_player_idx + 1) % PLAYER_COUNT; // Because there are 2 players

        // Step 1: Allocate new cards from hit actions
        {
            let allocations = blackjack::process_hit_actions(&ds.actions, &ds.allocations);

            // Check for updates to the hand states.
            //
            // @note:
            //  We really only want to do this to any hands that have changed due to
            //  an addition to the card states.  We dont want to be doing this for all
            //  hands.
            //
            let updated_hands = allocations
                .iter()
                .filter_map(|ca| ds.hands.iter().find(|&h| h.id == ca.hand))
                .cloned()
                .collect::<Vec<_>>();

            // Merge allocations into the master list.
            ds.allocations.extend(allocations);

            // Check if any of the new hands have busted or hit blackjack.
            let resulting_states =
                blackjack::process_hand_states(&updated_hands, &ds.allocations, &ds.decks);
            //todo!("need to add a step here to iterate hand states to check for children that need to be added");

            // Merge into the master state list
            ds.hand_states.extend(resulting_states);
        }

        // Step 2: Process all the hold actions.
        {
            let hold_states =
                blackjack::process_hold_actions(&ds.hands, &ds.actions, &ds.allocations, &ds.decks);

            // Merge these into the master state list
            ds.hand_states.extend(hold_states);
        }
    }

    //assert!(that the game is over)

    // determine the outcome of the game.
    let outcomes = blackjack::resolve_outcomes(&ds.hand_states, &ds.outcomes);
    ds.outcomes.extend(outcomes);

    assert_eq!(
        Some(blackjack::Outcome::Won),
        blackjack::get_hand_outcome(hand_id, &ds.outcomes)
    );
}
