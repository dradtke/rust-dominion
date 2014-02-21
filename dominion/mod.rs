
use extra::arc;

use std::hashmap::HashMap;
use std::ptr;
use std::util;

pub mod card;
pub mod error;
pub mod strat;

macro_rules! unwrap_or_err(
    ($val:expr, $err:expr) => ({
        match $val {
            None => return Some($err),
            _ => (),
        }
        $val.unwrap()
    });
)

// play() is the entry point for a game. It should be passed a
// list of Player references, and does a few things. First it
// creates a new supply pile, sticks it in a RWArc and gives each
// player a reference to it; then it loops forever until the game
// has ended, playing each player in turn; and finally, it
// prints out the results of the game.
pub fn play(players: &mut [Player]) -> Option<~str> {
    let num_players = players.len();
    if num_players <= 1 {
        fail!("Not enough players!");
    }
    let empty_limit = match num_players {
        0..4 => 3,
        _    => 4,
    };

    let mut supply = HashMap::new();
    supply.insert(&card::copper,   30);
    supply.insert(&card::silver,   30);
    supply.insert(&card::gold,     30);
    supply.insert(&card::estate,   12);
    supply.insert(&card::duchy,    12);
    supply.insert(&card::province, 12);
    supply.insert(&card::curse,    30);

    // now for the variations!
    supply.insert(&card::smithy, 10);

    let supply_ref = arc::RWArc::new(supply);
    let mut player_refs = ~[];

    // give each player a reference to the supply
    // and fill the player_refs array
    for player in players.mut_iter() {
        player.supply_ref = supply_ref.clone();
        player_refs.push(player as *mut Player);
    }

    // give each player a clone of player_refs and
    // ensure that each player's copy has them at
    // position 0.
    for player in players.mut_iter() {
        player.player_refs = player_refs.clone();
        let me = (player as *mut Player);
        loop {
            if player.player_refs[0] == me {
                break;
            }
            let p = player.player_refs.shift();
            player.player_refs.push(p);
        }
    }

    'game: loop {
        for player in players.mut_iter() {
            player.new_hand();
            player.actions = 1;
            player.buys = 1;
            player.buying_power = 0;
            (player.play)(player);
            player.discard();

            let done = player.supply_ref.read(|supply| {
                if *supply.get(& &card::province) == 0 {
                    return true;
                }
                let num_empty = supply.values()
                    .filter(|x| **x == 0)
                    .fold(0, |a, &b| a + b);

                num_empty >= empty_limit
            });
            if done {
                break 'game;
            }
        }
    }

    // Calculate the results
    let mut highest_score = 0;
    players.mut_iter().advance(|p| {
        p.calculate_score();
        if p.score > highest_score {
            highest_score = p.score;
        }
        true
    });
    let winners = players.iter().filter(|p| p.score == highest_score).to_owned_vec();

    let num_winners = winners.len();
    if num_winners == 1 {
        Some(winners[0].name.clone())
    } else {
        // tie
        None
    }
}

pub type PlayerFunc = fn(&mut Player);

pub struct Player {
    priv supply_ref: arc::RWArc<HashMap<card::Card, uint>>,
    priv name: ~str,
    priv play: PlayerFunc,
    priv player_refs: ~[*mut Player], // unsafe references to the other players

    priv deck: ~[card::Card],
    priv discard: ~[card::Card],
    priv in_play: ~[card::Card],
    priv hand: ~[card::Card],
    // TODO: just-gained? other card "locations"?

    priv actions: uint,
    priv buys: uint,
    priv buying_power: uint,
    priv score: int, // for calculating the final score
}

impl Player {
    // new() creates a new player. They're given a shuffled deck
    // of 7 coppers and 3 estates.
    pub fn new(name: ~str, play: PlayerFunc) -> Player {
        let mut deck = ~[];
        deck.push_all_move(card::copper.create_copies(7));
        deck.push_all_move(card::estate.create_copies(3));
        card::shuffle(deck);
        Player{
            supply_ref: arc::RWArc::new(HashMap::new()),
            name: name,
            play: play,
            player_refs: ~[],
            deck: deck,
            discard: ~[],
            in_play: ~[],
            hand: ~[],
            actions: 0,
            buys: 0,
            buying_power: 0,
            score: 0,
        }
    }

    // get_name() returns the name of this player. The reassignment to a
    // borrowed pointer is necessary because otherwise an error is thrown
    // when the method finds `~str` instead of `&'a str`. The lifetime
    // parameter tells the compiler that the borrowed name has the same
    // lifetime as the Player it's owned by.
    pub fn get_name<'a>(&'a self) -> &'a str {
        let name: &str = self.name;
        name
    }

    // get_available_money() returns a count of the total available money
    // currently in the player's hand.
    pub fn get_available_money(&self) -> uint {
        self.hand.iter()
            .filter(|&c| c.is_money())
            .fold(0, |a, &b| a + b.get_value())
    }

    // get_buying_power() returns the current available buying power from
    // everything that's been played so far.
    pub fn get_buying_power(&self) -> uint {
        self.buying_power
    }

    // get_total_points() counts up the total point value from all victory
    // and curse cards in the player's deck, hand, and discard.
    pub fn get_total_points(&self) -> int {
        self.deck.iter().chain(self.discard.iter()).chain(self.hand.iter())
            .filter(|&c| c.is_victory() || c.is_curse())
            .fold(0, |a, &b| a + b.get_points())
    }

    // get_hand() returns a copy of the player's hand. The Card type
    // is defined as a static pointer to a CardDef, so it's not as
    // expensive as if it cloned the card definitions themselves, but
    // is still more expensive than an implementation using an Arc
    // or similar utility.
    pub fn get_hand(&self) -> ~[card::Card] {
        self.hand.clone()
    }

    // get_hand_size() returns the number of cards in the player's hand.
    pub fn get_hand_size(&self) -> uint {
        self.hand.len()
    }

    pub fn has(&self, c: card::Card) -> bool {
        self.hand.iter().any(|&x| x == c)
            || self.deck.iter().any(|&x| x == c)
            || self.discard.iter().any(|&x| x == c)
            || self.in_play.iter().any(|&x| x == c)
    }

    // hand_contains() returns true if and only if the player's hand contains
    // the specified card.
    pub fn hand_contains(&self, c: card::Card) -> bool {
        self.hand.iter().any(|&x| x == c)
    }

    // play() plays a card. It returns an InvalidPlay error if either (a) the requested
    // card is not in the player's hand, or (b) the card cannot be played, e.g. Province.
    // Other errors may occur if there are not enough actions or buys, and once a Money
    // card is played, then the player's action count is set to 0.
    pub fn play(&mut self, c: card::Card) -> Option<error::Error> {
        let index = unwrap_or_err!(self.hand.iter().position(|&x| x == c), error::InvalidPlay);
        match *c {
            card::Money { value: v, .. } => {
                self.buying_power += v;
                self.actions = 0;
            },
            card:: Action { action: a, .. } => {
                if self.actions == 0 {
                    return Some(error::NoActions);
                }
                unsafe {
                    (*a)(self);
                    self.actions = self.actions - 1;
                }
            },
            _ => return Some(error::InvalidPlay),
        }
        self.in_play.push(self.hand.remove(index));
        None
    }

    pub fn play_all_money(&mut self) {
        let hand = self.get_hand();
        for money in hand.iter().filter(|&c| c.is_money()) {
            self.play(*money);
        }
    }

    // buy() buys a card from the supply, returning one of three possible
    // errors:
    //   1. NotInSupply, if the card is not available in this game
    //   2. EmptyPile, if there are no more available to buy
    //   3. NotEnoughMoney(difference), if the player doesn't have the money
    // On success, the appropriate supply count is decremented and a copy
    // of the card is added to the player's discard pile.
    pub fn buy(&mut self, c: card::Card) -> Option<error::Error> {
        self.supply_ref.write(|supply| -> Option<error::Error> {
            if !supply.contains_key(&c) {
                return Some(error::NotInSupply);
            }
            let pile = *supply.get(&c);
            if pile == 0 {
                return Some(error::EmptyPile);
            }
            let need = c.get_cost();
            if self.buying_power >= need {
                supply.insert(c, pile - 1);
                self.discard.push(c);
                self.actions = 0;
                self.buying_power -= need;
                None
            } else {
                Some(error::NotEnoughMoney(need - self.buying_power))
            }
        })
    }

    // curse() gives the player a curse card and depletes one from the supply.
    fn curse(&mut self) -> Option<error::Error> {
        self.supply_ref.write(|supply| -> Option<error::Error> {
            let pile = *supply.get(& &card::curse);
            if pile == 0 {
                return Some(error::EmptyPile);
            }
            supply.insert(&card::curse, pile - 1);
            self.discard.push(&card::curse);
            None
        })
    }

    // count() returns either the number available for a given card, or a
    // NotInSupply error if the card isn't available in the game.
    pub fn count(&mut self, c: card::Card) -> Result<uint, error::Error> {
        self.supply_ref.read(|supply| -> Result<uint, error::Error> {
            if !supply.contains_key(&c) {
                Err(error::NotInSupply)
            } else {
                Ok(*supply.get(&c))
            }
        })
    }

    // new_hand() draws up to five cards from the deck and places them in
    // the player's hand.
    fn new_hand(&mut self) {
        for _ in range(0, 5) {
            self.draw();
        }
    }

    // discard() puts all of the cards the player's hand and in-play into the
    // discard pile.
    fn discard(&mut self) {
        loop {
            match self.hand.shift_opt() {
                Some(c) => self.discard.push(c),
                None => break,
            }
        }
        loop {
            match self.in_play.shift_opt() {
                Some(c) => self.discard.push(c),
                None => break,
            }
        }
    }

    // draw() removes a card from the top of the deck and adds it to the hand. If
    // the deck is empty, then the discard pile and deck are swapped (making
    // the deck equal to the old discard and the discard empty), the deck
    // is shuffled, and the draw is tried again.
    fn draw(&mut self) {
        match self.deck.shift_opt() {
            Some(c) => self.hand.push(c),
            None => {
                // deck is empty, swap it with the discard and shuffle it
                if self.discard.len() == 0 {
                    return;
                } else {
                    util::swap(&mut self.deck, &mut self.discard);
                    card::shuffle(self.deck);
                    self.draw();
                }
            }
        };
    }

    fn calculate_score(&mut self) {
        self.score = self.get_total_points();
    }

    unsafe fn other_players(&mut self) -> ~[&Player] {
        let mut them = ~[];
        for player in self.player_refs.iter().skip(1) {
            them.push(&(*ptr::read_ptr(player)));
        }
        them
    }

    unsafe fn left_player(&mut self) -> &Player {
        let player = self.player_refs.iter().skip(1).next().unwrap();
        &(*ptr::read_ptr(player))
    }

    unsafe fn right_player(&mut self) -> &Player {
        let player = self.player_refs.iter().last().unwrap();
        &(*ptr::read_ptr(player))
    }
}
