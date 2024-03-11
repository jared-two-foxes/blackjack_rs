use std::collections::HashMap;
use uuid::Uuid;

enum Suit {
    Hearts,
    Diamonds,
    Clubs,
    Spades,
}

enum Value {
    Ace,
    King,
    Queen,
    Jack,
    Value(u8),
}

type Card = (Suit, Value);

struct Hand {
    id: Uuid,
    player: Uuid,
    game: Uuid,
    cards: Vec<Card>,
    dealer: bool,
}

fn hand_value(cards: &Vec<Card>) -> u32 {
    0
}

fn hand_bust(cards: &Vec<Card>) -> bool {
    hand_value(cards) > 21
}

type Deck = Vec<Card>;

enum DeckError {
    Empty,
}

fn draw(deck: &mut Deck) -> Result<Card, DeckError> {
    deck.pop().ok_or(DeckError::Empty)
}

enum Action {
    Hit,
    Hold,
    Split,
}

enum HandState {
    Active,
    Win,
    Lost,
}

//pair mapping hand to an action
type HandAction = (Uuid, Action);

struct DataSource {
    decks: HashMap<Uuid, Deck>, // map of game_id to Deck for a given game
    hands: Vec<Hand>,
    actions: Vec<HandAction>,
}

//
// Systems/Controllers
//

enum ActionResolutionError {
    MissingResource,
    EmptyDeck,
}

fn process_actions(
    actions: &Vec<HandAction>,
    hands: &mut Vec<Hand>,
    decks: &mut HashMap<Uuid, Deck>,
) -> Result<(), ActionResolutionError> {
    for ha in actions {
        let (uuid, action) = ha;
        let hand = hands
            .iter_mut()
            .find(|h| h.id == *uuid)
            .ok_or(ActionResolutionError::MissingResource)?;
        let deck = decks
            .get_mut(&hand.game)
            .ok_or(ActionResolutionError::MissingResource)?;

        /*hand.state =*/
        resolve_player_action(action, &mut hand.cards, deck)
            .map_err(|_| ActionResolutionError::EmptyDeck);
    }

    Ok(())
}

fn resolve_player_action(
    action: &Action,
    cards: &mut Vec<Card>,
    deck: &mut Vec<Card>,
) -> Result<(), DeckError> {
    match action {
        Action::Hit => {
            let c = draw(deck)?;
            cards.push(c);
        }
        Action::Hold => {
            // do nothing?
        }
        Action::Split => {
            unimplemented!();
        }
    }

    Ok(())
}

// turn sequence; the order in which players take turns (with the dealer going last)

// action validation; an action is only balid if its that players turn to go.

// Rules engine?
