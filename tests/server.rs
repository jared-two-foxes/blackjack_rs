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

    let allocations = blackjack::process_actions(&ds.actions, &ds.allocations); //< this could error so we should do something about that?
    blackjack::resolve_hand_states(&ds.hands, &allocations);

    //@todo: merge the modified hands back in to the list.

    //assert_eq!(blackjack::Outcome::Won, get_game_outcome(&hands, hand_id));
}
