// 
// Integration tests for our blackjack server
//
use blackjack::{DataSource,Action};

// import our lib and setup a game
#[test]
fn main() {
  let mut ds = DataSource::default();
  let game_id = add_game(&ds);
  let hand_id = add_player(&ds, game_id);
  
  add_action(&ds, hand_id, Action::Hit);
  process_actions(&ds.actions, &ds.hands, &ds.decks);
  
  assert_eq!(false); // Need to figure out exactly what we are asserting on.
}