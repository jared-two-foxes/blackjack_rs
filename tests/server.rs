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

    // Building out a single "turn", where a turn involves reacting to a hand action
    // and then checking whether the hand is now "bust"
    {
        let allocations = blackjack::process_actions(&ds.actions, &ds.allocations); //< this could error so we should do something about that?

        // Determine which hands have been modified.
        let hands = allocations
            .iter()
            .filter_map(|a| ds.hands.iter().find(|h| h.id == a.hand))
            .collect::<Vec<_>>();

        // Merge allocations into the master list.
        ds.allocations.extend(allocations);

        // Check if any of the hands have busted.
        let hand_states = blackjack::resolve_hand_states(&hands, &ds.allocations, &ds.decks);

        // Check the table state.
        blackjack::resolve_table_state(&ds.hands, &ds.allocations, &ds.decks);
    }

    //assert_eq!(blackjack::Outcome::Won, get_game_outcome(&hands, hand_id));
}
