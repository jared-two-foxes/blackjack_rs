mod data_source;
mod types;
mod utils;

pub use data_source::DataSource;
pub use types::{Action, Card, CardValue, Deck, Hand, HandAction, HandState, Outcome, State, Suit};
pub use utils::hand_value;
use types::CardAllocation;

use log::trace;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use uuid::Uuid;

/* I think this is going to be a whole new thing on a new micro
  thread or something.

// Task_1: Check if we need to expand table count
            // lets check if we have a minimum of 'x' valid tables.
            let table_count = open_tables(&ds.hands);
            if table_count < 8 {
                // Add 'y' new empty tables for clients to sit at.
                for _ in 0..16 {
                    let game_id = ds.add_game();
                    ds.set_deck(game_id, create_deck());
                }
            }
*/

pub fn start_backend(
    actions: Arc<Mutex<Vec<HandAction>>>,
    ds: DataSource,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        loop {
            // @todo: introduce a time to batch & throttle the calls? Rather than do them one at a time?
            // @todo: Lets take this lock and copy out the actions that we care about for this round.
            // @todo: if there isnt an action for a particular user that should be acting then we should
            //        make them hold instead.

            // Grab the mutex lock for the data source.
            let mut to_process = Vec::new();
            if let Ok(mut actions) = actions.try_lock() {
                let pivot = actions.partition_point(|a| ds.active_hands.contains(&a.0));
                to_process = actions[pivot..].to_vec();
                actions.truncate(pivot);
            }

            let (allocations, hand_states) =
                process_user_actions(&to_process, &ds.hands, &ds.allocations, &ds.decks);

            // maps hand_states to hand_outcome
            //let hand_outcomes = update_hand_outcomes(&hand_states, &ds.allocations, &ds.decks);
        }
    })
}

fn process_user_actions(
    actions: &[HandAction],
    hands: &[Hand],
    allocations: &[CardAllocation],
    decks: &HashMap<Uuid, Deck>,
) -> (Vec<CardAllocation>, Vec<HandState>) {
    let new_allocations = actions
        .iter()
        .filter(|(_, action)| matches!(action, Action::Hit))
        //@todo: What should we do here if we cant find the hand?  We currently dont
        // log this or anything, it just silently dies.
        .filter_map(|(hand_id, _)| hands.iter().find(|hand| hand.id == *hand_id))
        .map(|hand| {
            let card_idx = allocations
                .iter()
                .filter(|a| a.dealer == hand.dealer)
                .count();
            trace!("Adding card allocation: {},{}", hand.id, card_idx);
            CardAllocation {
                card_idx,
                dealer: hand.dealer,
                hand: hand.id,
            }
        })
        .collect::<Vec<_>>();

    // Check for updates to the hand states.
    let updated_hands = allocations
        .iter()
        .filter_map(|ca| hands.iter().find(|&h| h.id == ca.hand))
        .cloned()
        .collect::<Vec<_>>();

    //@todo: Hmmm, this doesnt work because I havent added the new allocations into this list.
    //This needs to be incremental and pass new_allocations instead
    // Check if any of the new hands have busted or hit blackjack.
    let resulting_states = utils::process_hand_states(&updated_hands, allocations, decks);
    //@todo!("need to add a step here to iterate hand states to check for children that need to be added");

    (new_allocations, resulting_states)
}

// Merge allocations into the master list.
//allocations.extend(new_allocations);
// Merge into the master state list
//hand_states.extend(resulting_states);