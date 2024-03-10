enum Suit { 
  Hearts,
  Diamonds,
  Clubs,
  Spades
}

enum Value {
  Ace,
  King,
  Queen,
  Jack,
  Value(u8),
}  

type Card = (Suit,Value);

struct Hand  {
  id: Uuid,
  player: Uuid, 
  game: Uuid,
  cards: Vec<Card>,
  dealer: bool,
} 

type Deck = Vec<Card>;



fn hand_value(cards: &Vec<Card>) -> u33 {
  0
}

fn hand_bust(cards:&Vec<Card>) -> bool {
  hand_value(cards) > 21
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
type HandAction = (Uuid,Action);

struct DataSource {
  decks: HashMap<Uuid,Deck>, // map of game_id to Deck for a given game
  hands: Vec<Hand>, 
  actions: Vec<HandAction>,
}




//
// Systems/Controllers
//



pub fn process_actions(actions: &Vec<HandAction>, hands: Vec<Hand>, decks: Vec<Deck>) -> Result<()> {
  for ha in actions {
    let (uuid, action) = ha; 
    let hand = hands.find(uuid)?;
    let deck = decks.find(get_game(uuid))?;
    
    hand.state = resolve_player_action(action, &mut hand.cards, &mut deck)?;
  }
  
  Ok(())
}

fn resolve_player_action(action: Action, cards: &mut Vec<Card>, deck: &mut Vec<Card>) -> Result<()> {
  switch action {
    Hit => {
      let c = draw(deck)?;
      cards.push(c);
    },
    Hold => { 
      // do nothing? 
    },
    //Split => {}
  }
  
  Ok(())
}


// turn sequence; the order in which players take turns (with the dealer going last)

// action validation; an action is only balid if its that players turn to go.

// Rules engine?