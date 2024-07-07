//
// Integration tests for our blackjack server
//
use blackjack::{Action, DataSource};

//@todo:
//  Figure out logging, when where why how
//  Double deal on the first round.  Each player should receive 2 cards
//  Game should end if all the players go bust or blackjack before the dealer draws
//  Game should end and all active players should win if the dealer busts

// import our lib and setup a game
#[test]
fn can_play_a_simple_game() {
    let mut ds = DataSource::default();
    let game_id = blackjack::add_game(&mut ds);
    //@todo: swap in a loaded deck
    let hand_id = blackjack::add_player(&mut ds, game_id);

    // Need to determine the turn sequence somehow.
    let turn_order = vec![hand_id, game_id];
    let mut current_hand_idx: usize = 0;
    const PLAYER_COUNT: usize = 2;

    // Building out a single "round", where a round involves reacting to a
    // collection of hand actions and then detrrmining the results for each
    // hand that was acted upon
    while !blackjack::is_game_complete(game_id, &ds.hands, &ds.hand_states) {
        let current_hand = turn_order[current_hand_idx];
        let hand_value =
            blackjack::get_hand_value(current_hand, &ds.hands, &ds.allocations, &ds.decks);

        println!("processing for hand: {} ({})", current_hand_idx, hand_value);

        if current_hand == hand_id {
            blackjack::add_action(&mut ds, hand_id, Action::Hit);
        } else {
            // This is the dealers turn.
            if hand_value > 17 {
                blackjack::add_action(&mut ds, current_hand, Action::Hold);
            } else {
                blackjack::add_action(&mut ds, current_hand, Action::Hit);
            }
        }

        // Step 1: Allocate new cards from hit actions
        {
            let allocations =
                blackjack::process_hit_actions(&ds.actions, &ds.hands, &ds.allocations);

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

        // Step 3: Pick next hand to act.
        {
            let mut next_hand_idx = current_hand_idx;
            current_hand_idx = loop {
                next_hand_idx = (next_hand_idx + 1) % PLAYER_COUNT; // Because there are 2 players
                println!("Checking active state of {}", next_hand_idx);
                let hand_id = turn_order[next_hand_idx];
                let is_hand_active = blackjack::is_hand_active(hand_id, &ds.hand_states);
                if is_hand_active {
                    println!("Returning {}", next_hand_idx);
                    break next_hand_idx;
                }
                if next_hand_idx == current_hand_idx {
                    //@note:
                    //  We've looped all the hands, none were active which indicates that
                    //  the game is over.
                    println!("We've iterated all the hands, returning {}", next_hand_idx);
                    break next_hand_idx;
                }
            };
        }

        println!(); //< Add blankline to seperate iterations
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
