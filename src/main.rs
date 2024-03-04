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
  player: Uuid,
  cards: Vec<Card>,
} 

struct GameController {
  deck: Vec<Card>,
  hands: Vec<Hand>, // dealers hand is thr first in list
}

fn hand_value(cards: &Vec<Card>) -> u33 {
  0
}

fn hand_bust(cards:&Vec<Card>) -> bool {
  hand_value(cards) > 21
}

fn is_finished(dealer: &Vec<Hand>, hands: &Vec<Hand>) -> bool {
  let players_out = hands
    .iter()
    .all(|h| hand_bust(h.cards))
  
  players_out || hand_bust(h.cards)
}

fn simulate_round(deck: &mut Vec<Card>, dealer: &mut Vec<Card>, hands: &mut Vec<Hand>) -> bool {
  if is_finished(&hands[0], &hands[1..]) {
    return false;
  }
  
  step(deck, hands);
  broadcast_results(hands);
}

enum Action {
  Hit,
  Hold,
}

enum HandState {
  Active,
  Win,
  Lost,
}

fn resolve_player_action(action: Action, cards: &mut Vec<Card>, deck: &mut Vec<Card>) -> Result<HandState> {
  switch action {
    Hit => {
      let c = draw(deck)?;
      h.cards.push(c);
    },
    Hold => { 
      // do nothing? 
    }
  }
  
  let mut r = Outcome::Active;
  if is_bust(cards) {
    r = Outcome::Lost;
  }
  
  Ok(r) 
}

impl GameController {
  fn resolve(self: &Self, player: Uuid, action: Action) -> Result<> {
    let hand = self.get_active_hand(uuid)?;
    hand.state = resolve_player_action(action, hand.cards, &self.deck)?;
    
    self.active_hand = self.active_hand+1;
    if are_players_finished() {
      resolve_dealer();
      if is_finished() {
        // report results
      }   
    }
  } 
}