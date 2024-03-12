use std::collections::HashMap;
use uuid::Uuid;

pub enum Suit {
    Hearts,
    Diamonds,
    Clubs,
    Spades,
}

pub enum Value {
    Ace,
    King,
    Queen,
    Jack,
    Value(u8),
}

pub type Card = (Suit, Value);

pub struct Hand {
    id: Uuid,
    player: Uuid,
    game: Uuid,
    cards: Vec<Card>,
    dealer: bool,
}

fn hand_value(cards: &Vec<Card>) -> u32 {
    let mut ace_count = 0;
    let mut value = cards.iter().map(|c| {
      match c.1 {
        Value::Value(v) => v,
        Value::Ace => { ace_count = ace_count + 1; 11 }
        _ => 10
      }
    }).sum::<u32>();
    for i in ..ace_count {
      if value > 21 {
        value = value - 10;
      } 
    }
    value
}

fn hand_bust(cards: &Vec<Card>) -> bool {
    hand_value(cards) > 21
}

pub type Deck = Vec<Card>;

pub fn new_deck() -> Deck {
    // Todo: Create a deck with all of the cards in it.
    vec![(Suit::Hearts, Value::King)]
}

enum DeckError {
    Empty,
}

fn draw(deck: &mut Deck) -> Result<Card, DeckError> {
    deck.pop().ok_or(DeckError::Empty)
}

pub enum Action {
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
pub type HandAction = (Uuid, Action);

#[derive(Default)]
pub struct DataSource {
    pub decks: HashMap<Uuid, Deck>, // map of game_id to Deck for a given game
    pub hands: Vec<Hand>,
    pub actions: Vec<HandAction>,
}

pub fn add_game(ds: &mut DataSource) -> Uuid {
    let game_id = Uuid::default();
    ds.decks.insert(game_id, new_deck());
    ds.hands.push(Hand {
        id: Uuid::default(),
        player: Uuid::default(),
        game: game_id,
        cards: Vec::new(),
        dealer: true,
    });
    game_id
}

pub fn add_player(ds: &mut DataSource, game_id: Uuid) -> Uuid {
    let player_id = Uuid::default();
    ds.hands.push(Hand {
        id: Uuid::default(),
        player: player_id,
        game: game_id,
        cards: Vec::new(),
        dealer: false,
    });

    player_id
}

// Should this return a uuid or something to identify the Action passed?
pub fn add_action(ds: &mut DataSource, hand_id: Uuid, action: Action) {
    ds.actions.push((hand_id, action));
}

//
// Systems/Controllers
//

pub enum ActionResolutionError {
    MissingResource,
    EmptyDeck,
}

pub fn process_actions(
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
        resolve_player_action(action, &mut hand.cards, deck)?;
    }

    Ok(())
}

pub fn resolve_player_action(
    action: &Action,
    cards: &mut Vec<Card>,
    deck: &mut Vec<Card>,
) -> Result<(), ActionResolutionError> {
    match action {
        Action::Hit => {
            let c = draw(deck).map_err(|_| ActionResolutionError::EmptyDeck)?;
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
