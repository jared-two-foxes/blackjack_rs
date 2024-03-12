//
// Integration tests for our blackjack server
//
use blackjack::{Action, DataSource};

// import our lib and setup a game
#[test]
fn can_setup_games() {
    let mut ds = DataSource::default();
    let game_id = blackjack::add_game(&mut ds);
    let hand_id = blackjack::add_player(&mut ds, game_id);

    blackjack::add_action(&mut ds, hand_id, Action::Hit);
    let _ = blackjack::process_actions(&ds.actions, &mut ds.hands, &mut ds.decks); //< this could
                                                                                   //error so we should do something about that.

    assert!(false); // Need to figure out exactly what we are asserting on.
}
