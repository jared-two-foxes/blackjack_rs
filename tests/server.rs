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

    blackjack::add_action(&mut ds, hand_id, Action::Hit);

    // Building out a single "round", where a round involves reacting to a
    // collection of hand actions and then detrrmining the results for each
    // hand that was acted upon
    {
        // allocate new cards from hit actions
        let allocations = blackjack::process_hit_actions(&ds.actions, &ds.allocations);

        // Determine which hands have had cards allocated to then.
        /*let hands = allocations
        .iter()
        .filter_map(|a| ds.hands.iter().find(|h| h.id == a.hand))
        .collect::<Vec<_>>();*/

        // Merge allocations into the master list.
        ds.allocations.extend(allocations);

        // Check for updates to the hand states.
        let resulting_states =
            blackjack::process_hand_states(/*&hands*/ &ds.hands, &ds.allocations, &ds.decks);

        //todo!("need to add a step here to iterate hand states to check for children that need to be added");

        // Merge into the master state list
        ds.hand_states.extend(resulting_states);

        // process all the hold actions.
        let hold_states =
            blackjack::process_hold_actions(&ds.hands, &ds.actions, &ds.allocations, &ds.decks);

        // Merge these into the master state list
        ds.hand_states.extend(hold_states);
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
