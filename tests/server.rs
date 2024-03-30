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
        let hands = allocations.iter().map(|a| a.hand).collect::<Vec<_>>();
        
        // Merge allocations into the master list.
        ds.allocations.extend(allocations);
        
        
        // Check if any of the hands have busted.
        let busted_states = blackjack::resolve_hand_states(&hands, &ds.allocations, &ds.decks);
        
        // Merge into the master state list
        ds.hand_states.extend(busted_states);

        // process all if the hold actions.
        let hold_states = blackjack::process_hold_action(&ds.actions, &ds.allocations);
        
        // Merge these into the master state list
        ds.hand_states.extend(hold_states);
    }
    
    //assert!(that the game is over)

    // determine the outcome of the game.
    blackjack::resolve_outcomes(&ds.hands, &ds.hand_states);

    assert_eq!(blackjack::Outcome::Won, get_game_outcome(&hands, hand_id));
}
