use std::collections::HashMap;
use uuid::Uuid;

use crate::types::*;
use crate::utils::*;

#[derive(Default)]
pub struct DataSource {
    pub hands: Vec<Hand>,
    pub decks: HashMap<Uuid, Deck>, // map of game_id to Deck for a given game
    pub allocations: Vec<CardAllocation>,
    pub hand_states: Vec<HandState>,
    pub actions: Vec<HandAction>,
    pub outcomes: Vec<HandOutcome>,
    sequence: Vec<Sequence>,
    active_hands: Vec<Uuid>,
}

impl DataSource {
    pub fn add_game(&mut self) -> Uuid {
        let dealer_id = Uuid::new_v4();
        self.decks.insert(dealer_id, new_deck());
        self.hands.push(Hand {
            id: dealer_id,
            player: dealer_id,
            dealer: dealer_id,
        });
        dealer_id
    }

    //@todo: This needs to return something I guess to indicate success or failure.
    pub fn set_deck(&mut self, game_id: Uuid, deck: Deck) {
        self.decks
            .entry(game_id)
            .and_modify(|current| *current = deck);
    }

    //@todo: this is a little awkward.  We should potentially have a second function to
    // create a hand which returns the hand_id else how does the client know how to add
    // an action?
    pub fn add_player(&mut self, dealer_id: Uuid) -> Uuid {
        let player_id = Uuid::new_v4();
        self.hands.push(Hand {
            id: player_id,
            player: player_id,
            dealer: dealer_id,
        });

        player_id
    }

    //@todo: I think this should this return a uuid; reasons 2 fold, we probably
    //       should have a means to identify the action, and we dont want methods
    //       with no return type.

    pub fn add_action(&mut self, hand_id: Uuid, action: Action) {
        match action {
            Action::Hit => println!("Adding Hit Action"),
            Action::Hold => println!("Adding Hold Action"),
        };
        self.actions.push((hand_id, action));
    }

    pub fn start_game(&mut self, game_id: Uuid) {
        // Every hand gets 2 card
        let mut allocations = Vec::new();
        allocations.extend(allocate_cards(&self.hands, &self.allocations, game_id));
        allocations.extend(allocate_cards(&self.hands, &self.allocations, game_id));

        // Grab the list of the hands that have been updated (this should be all the hands in
        // this game)
        let updated_hands = allocations
            .iter()
            .filter_map(|ca| self.hands.iter().find(|&h| h.id == ca.hand))
            .cloned()
            .collect::<Vec<_>>();

        // Combine the allocations into the master allocation list
        self.allocations.extend(allocations);

        // We now need to check the hand states incase anything interesting has
        // resolved from that.
        let resulting_states = process_hand_states(&updated_hands, &self.allocations, &self.decks);

        // Merge any hand_states into the master state list
        self.hand_states.extend(resulting_states);
    }

    pub fn process_hit_actions(&mut self) {
        let allocations = process_hit_actions(&self.actions, &self.hands, &self.allocations);

        // Check for updates to the hand states.
        let updated_hands = allocations
            .iter()
            .filter_map(|ca| self.hands.iter().find(|&h| h.id == ca.hand))
            .cloned()
            .collect::<Vec<_>>();

        // Merge allocations into the master list.
        self.allocations.extend(allocations);

        // Check if any of the new hands have busted or hit blackjack.
        let resulting_states = process_hand_states(&updated_hands, &self.allocations, &self.decks);
        //todo!("need to add a step here to iterate hand states to check for children that need to be added");

        // Merge into the master state list
        self.hand_states.extend(resulting_states);
    }

    pub fn process_hold_actions(&mut self) {
        let hold_states =
            process_hold_actions(&self.hands, &self.actions, &self.allocations, &self.decks);

        // Merge these into the master state list
        self.hand_states.extend(hold_states);
    }

    pub fn resolve_turn(&mut self) {
        let new_outcomes = resolve_outcomes(&self.hand_states, &self.outcomes);
        self.outcomes.extend(new_outcomes);

        self.active_hands = self
            .active_hands
            .iter()
            .map(|current_hand_id| {
                get_next_hand(
                    *current_hand_id,
                    &self.sequence,
                    &self.hands,
                    &self.hand_states,
                )
            })
            .collect::<Vec<_>>();

        self.actions.clear();
    }
}
