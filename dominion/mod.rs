#![crate_id = "dominion#0.1"]
#![crate_type = "lib"]
#![feature(macro_rules)]

extern crate collections;
extern crate rand;
extern crate sync;

use collections::{Deque,DList,HashMap};
use std::cell::RefCell;
use std::comm;
use std::hash;
use std::mem;
use std::rc::Rc;
use std::vec::Vec;
use rand::{Rng,task_rng};
use sync::Arc;

#[macro_export]
macro_rules! play(
    ($($player:ident),+) => ({
        let args = std::os::args();
        let n: uint = if args.len() > 1 { from_str(args[1]).unwrap() } else { 1000 };
        dominion::play(n, ~[$(~$player as ~dominion::Player:Send+Share,)+]);
    })
)

#[macro_export]
macro_rules! new_player(
    ($player:ident, $func:expr) => {
        struct $player;
        impl dominion::Player for $player {
            fn name(&self) -> &'static str {
                stringify!($player)
            }

            fn play(&self, p: &mut dominion::PlayerState) {
                $func(p);
            }
        }
    }
)

#[macro_export]
macro_rules! players(
    ($($player:ident),+) => (
        ~[$(~$player as ~dominion::Player:Send+Share,)+]
    )
)

macro_rules! unwrap(
	($val:expr else $err:expr) => ({
		match $val {
			None => return Some($err),
			_ => (),
		}
		$val.unwrap()
	});
)

macro_rules! card_count(
    ($p:expr, $c:expr) => ({
		let pile = unwrap!($p.count($c) else error::NotInSupply);
		if pile == 0 {
			return Some(error::EmptyPile);
		} else {
            pile
        }
    });
)

pub mod card;
pub mod error;
pub mod strat;


/* ------------------------ Public Methods ------------------------ */

pub trait Player {
    fn name(&self) -> &'static str;
    fn play(&self, p: &mut PlayerState);
}

// play() is the entry point for Dominion games, spawning a new task
// for each one and printing the results to standard output.
pub fn play(n: uint, player_defs: ~[~Player:Send+Share]) {
    println!("Playing {} games...", n);
    let (reporter, receiver) = comm::channel();
    let defs: ~[Arc<~Player:Send+Share>] = FromIterator::from_iter(player_defs.move_iter().map(|p| Arc::new(p)));
    for _ in range(0, n) {
        let reporter = reporter.clone();
        let defs = defs.clone();
        spawn(proc() {
            let results = play_game(defs);
            // TODO: send more information?
            let &(ref name1, ref score1) = results.get(0);
            let &(_, ref score2) = results.get(1);
            if score1 > score2 {
                reporter.send(Some(name1.into_owned()));
            } else {
                reporter.send(None);
            }
        });
    }

    let mut scores = HashMap::<~str,uint>::new();
    let mut ties = 0;
    for _ in range(0, n) {
        let winner = receiver.recv();
        if winner.is_some() {
            let name = winner.unwrap();
            if !scores.contains_key(&name) {
                scores.insert(name, 1);
            } else {
                let new_score = scores.get(&name) + 1;
                scores.insert(name, new_score);
            }
        } else {
            ties += 1;
        }
    }

    for key in scores.keys() {
        println!("{} won {} times", *key, *scores.get(key));
    }
    println!("There were {} ties.", ties);
}


/* ------------------------ Private Methods ------------------------ */


// play_game() playes a single game of Dominion. It plays a game and then
// returns a vector of tuples with the player's name along with their final
// score, ordered from highest to lowest.
fn play_game(player_defs: ~[Arc<~Player:Send+Share>]) -> Vec<(&str, int)> {
    let trash = Vec::new();

    let mut supply: Supply = HashMap::new();
    supply.insert(card::COPPER,   30);
    supply.insert(card::SILVER,   30);
    supply.insert(card::GOLD,     30);
    supply.insert(card::ESTATE,   12);
    supply.insert(card::DUCHY,    12);
    supply.insert(card::PROVINCE, 12);
    supply.insert(card::CURSE,    30);
    // now for the variations!
    supply.insert(card::SMITHY, 10);
    supply.insert(card::WITCH,  10);

    let game = Rc::new(RefCell::new(GameState{ supply: supply, trash: trash }));
    let empty_limit = get_empty_limit(player_defs.len());
    let players = init_players(player_defs, &game);

    loop {
        let (def, mut state) = (*players).borrow_mut().pop_front().unwrap();
        take_turn(&mut state, *def);
        (*players).borrow_mut().push_back((def, state));
        if is_game_finished(&game, empty_limit) {
            break;
        }
    }

    // Calculate the results
    for &(_, ref mut state) in (*players).borrow_mut().mut_iter() {
        state.calculate_score();
        /*
        println!("{}:", player.name);
        println!("\t{} Estates", player.number_of(card::ESTATE));
        println!("\t{} Duchies", player.number_of(card::DUCHY));
        println!("\t{} Provinces", player.number_of(card::PROVINCE));
        println!("\t{} Curses", player.number_of(card::CURSE));
        println!("\tFinal Score: {}", player.score);
        */
    }

    let mut results = Vec::with_capacity((*players).borrow().len());
    for res in (*players).borrow().iter().map(|&(ref def, ref state)| (def.name().clone(), state.score)) {
        results.push(res);
    }
    results.sort_by(|&(_, score1), &(_, score2)| score2.cmp(&score1));
    results
}


// init_players() takes a list of definitions and a shared pointer to the shared GameState reference
// and constructs a reference to a DList containing all of the players.
fn init_players(player_defs: ~[Arc<~Player:Send+Share>], game_ref: &Rc<RefCell<GameState>>) -> Rc<RefCell<DList<PlayerTup>>> {
    let mut deck = Vec::new();
    deck.push_all_move(card::COPPER.create_copies(7));
    deck.push_all_move(card::ESTATE.create_copies(3));
    shuffle(deck.as_mut_slice());

    let players = Rc::new(RefCell::new(DList::<PlayerTup>::new()));

    for def in player_defs.move_iter() {
        let mut ps = (*players).borrow_mut();
        ps.push_back((def, PlayerState{
            game_ref:      game_ref.clone(),
            other_players: players.clone(),
            deck:          deck.clone(),
            discard:       Vec::new(),
            in_play:       Vec::new(),
            hand:          Vec::new(),
            actions:       0,
            buys:          0,
            buying_power:  0,
            score:         0,
        }));
    }
    players
}

fn take_turn(player: &mut PlayerState, def: &Player) {
    player.new_hand();
    player.actions = 1;
    player.buys = 1;
    player.buying_power = 0;
    def.play(player);
    player.discard_hand();
}

fn get_empty_limit(n: uint) -> uint {
    if n < 2 {
        fail!("Not enough players!");
    } else if n > 6 {
        fail!("Too many players!");
    }
    match n {
        2..4 => 3,
        _ => 4,
    }
}

fn is_game_finished(game: &Rc<RefCell<GameState>>, empty_limit: uint) -> bool {
    if *(*game).borrow().supply.find(&card::PROVINCE).unwrap() == 0 {
        true
    } else {
        let num_empty = (*game).borrow().supply.values().filter(|&x| *x == 0).fold(0, |a, &b| a + b);
        num_empty >= empty_limit
    }
}

fn shuffle(cards: &mut [Card]) {
    task_rng().shuffle(cards);
}


/* ------------------------ Game and Player Types ------------------------ */


struct GameState {
    supply: Supply,
    trash: Vec<Card>,
}

// TODO: find a way to derive Default
pub struct PlayerState {
    game_ref: Rc<RefCell<GameState>>,
    other_players: Rc<RefCell<DList<PlayerTup>>>,

	deck: Vec<Card>,
	discard: Vec<Card>,
	in_play: Vec<Card>,
	hand: Vec<Card>,

	actions: uint,
	buys: uint,
	buying_power: uint,
	score: int, // for calculating the final score
}


impl PlayerState {
	// get_available_money() returns a count of the total available money
	// currently in the player's hand.
	pub fn get_available_money(&self) -> uint {
		self.hand.iter()
			.filter(|&c| c.is_money())
			.fold(0, |a, &b| a + b.treasure_value())
	}

	// get_buying_power() returns the current available buying power from
	// everything that's been played so far.
	pub fn get_buying_power(&self) -> uint {
		self.buying_power
	}

	// get_total_points() counts up the total point value from all victory
	// and curse cards in the player's deck, hand, and discard.
	pub fn get_total_points(&self) -> int {
		self.deck.iter()
			.chain(self.discard.iter())
			.chain(self.hand.iter())
			.filter(|&c| c.is_victory() || c.is_curse())
			.fold(0, |a, &b| a + b.victory_points(self))
	}

	// get_hand() returns a copy of the player's hand. The Card type
	// is defined as a static pointer to a CardDef, so it's not as
	// expensive as if it cloned the card definitions themselves, but
	// is still more expensive than an implementation using an Arc
	// or similar utility.
	pub fn get_hand(&self) -> Vec<Card> {
		self.hand.clone()
	}

	// get_hand_size() returns the number of cards in the player's hand.
	pub fn get_hand_size(&self) -> uint {
		self.hand.len()
	}

	// has() returns true if the player has the provided card, anywhere.
    pub fn has(&self, c: Card) -> bool {
        self.hand.iter().any(|&x| x == c)
            || self.deck.iter().any(|&x| x == c)
            || self.discard.iter().any(|&x| x == c)
            || self.in_play.iter().any(|&x| x == c)
    }

    // number_of() returns the number of instances of the provided card
    // that the player has, anywhere.
    pub fn number_of(&self, c: Card) -> uint {
        self.hand.iter().count(|&x| x == c)
            + self.deck.iter().count(|&x| x == c)
            + self.discard.iter().count(|&x| x == c)
            + self.in_play.iter().count(|&x| x == c)
    }

	// hand_contains() returns true if and only if the player's hand contains
	// the specified card.
	pub fn hand_contains(&self, c: Card) -> bool {
		self.hand.iter().any(|&x| x == c)
	}

	pub fn play(&mut self, c: Card) -> Option<error::Error> {
		self.play_and(c, [])
	}

	// play() plays a card. It returns an InvalidPlay error if either (a) the requested
	// card is not in the player's hand, or (b) the card cannot be played, e.g. Province.
	// Other errors may occur if there are not enough actions or buys, and once a Money
	// card is played, then the player's action count is set to 0.
	pub fn play_and(&mut self, c: Card, input: &[ActionInput]) -> Option<error::Error> {
		let index = unwrap!(self.hand.iter().position(|&x| x == c) else error::InvalidPlay);
        if !c.is_money() && !c.is_action() {
            return Some(error::InvalidPlay);
        }
		self.in_play.push(self.hand.remove(index).unwrap());
        if c.is_money() {
            self.buying_power += c.treasure_value();
            self.actions = 0;
        }
        if c.is_action() {
            if self.actions == 0 {
                return Some(error::NoActions)
            } else {
                self.actions -= 1;
                (c.get_action())(self, input);
            }
        }
        None
	}

	// play_all_money() is a utility method that iterates through the player's
	// hand and calls play() on each money card.
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
	pub fn buy(&mut self, c: Card) -> Option<error::Error> {
        let pile = card_count!(self, c);
		if self.buying_power >= c.cost {
			self.with_mut_supply(|supply| supply.insert(c, pile - 1));
			self.discard.push(c);
			self.actions = 0;
			self.buying_power -= c.cost;
			None
		} else {
			Some(error::NotEnoughMoney(c.cost - self.buying_power))
		}
	}

    // gain() takes a card from the supply, putting it in the discard pile.
    fn gain(&mut self, c: Card) -> Option<error::Error> {
        let pile = card_count!(self, c);
        self.with_mut_supply(|supply| supply.insert(c, pile - 1));
        self.discard.push(c);
        None
    }

    // gain_to_deck() takes a card from the supply, putting it on top of
    // the deck.
    fn gain_to_deck(&mut self, c: Card) -> Option<error::Error> {
        let pile = card_count!(self, c);
        self.with_mut_supply(|supply| supply.insert(c, pile - 1));
        self.deck.unshift(c);
        None
    }

    // gain_to_hand() takes a card from the supply, putting it into
    // the hand.
    fn gain_to_hand(&mut self, c: Card) -> Option<error::Error> {
        let pile = card_count!(self, c);
        self.with_mut_supply(|supply| supply.insert(c, pile - 1));
        self.hand.unshift(c);
        None
    }

	// curse() gives the player a curse card and depletes one from the supply.
	fn curse(&mut self) -> Option<error::Error> {
		let pile = self.count(card::CURSE).unwrap();
		if pile == 0 {
			Some(error::EmptyPile)
		} else {
			self.with_mut_supply(|supply| supply.insert(card::CURSE, pile - 1));
			self.discard.push(card::CURSE);
			None
		}
	}

	// count() returns either the number available for a given card, or None
	// if the card wasn't available in this game.
	pub fn count(&mut self, c: Card) -> Option<uint> {
		self.with_supply(|supply| {
			match supply.find(&c) {
				None => None,
				Some(count) => Some(*count),
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

	// discard_hand() puts all of the cards the player's hand and in-play into the
	// discard pile.
	fn discard_hand(&mut self) {
		loop {
			match self.hand.shift() {
				Some(c) => self.discard.push(c),
				None => break,
			}
		}
		loop {
			match self.in_play.shift() {
				Some(c) => self.discard.push(c),
				None => break,
			}
		}
	}

    // discard_deck() puts all of the cards from the deck into the discard pile.
    fn discard_deck(&mut self) {
		loop {
			match self.deck.shift() {
				Some(c) => self.discard.push(c),
				None => break,
			}
		}
    }

    // next_card() removes and returns the top card from the deck, shuffling
    // the discard pile to make a new deck if necessary.
    fn next_card(&mut self) -> Option<Card> {
        if self.deck.is_empty() {
            mem::swap(&mut self.deck, &mut self.discard);
            shuffle(self.deck.as_mut_slice());
        }
        self.deck.shift()
    }

    fn next_n_cards(&mut self, n: uint) -> Vec<Card> {
        let mut cards = Vec::with_capacity(n);
        for _ in range(0, n) {
            match self.next_card() {
                Some(c) => cards.push(c),
                None => break,
            }
        }
        cards
    }

    fn mill(&mut self) {
        match self.next_card() {
            Some(c) => self.discard.push(c),
            None => (),
        }
    }

	fn draw(&mut self) {
        match self.next_card() {
            Some(c) => self.hand.push(c),
            None => (),
        }
	}

	// discard() discards a card from the player's hand, adding it to the
	// discard pile. If it's not in the player's hand than a NotInHand
	// error is returned.
	fn discard(&mut self, c: Card) -> Option<error::Error> {
		match self.hand.iter().enumerate().find(|&(_,x)| *x == c) {
			None => Some(error::NotInHand),
			Some((i,_)) => {
				self.discard.push(self.hand.remove(i).unwrap());
				None
			},
		}
	}

	fn discard_first(&mut self) {
        self.discard.push(self.hand.remove(0).unwrap());
	}

	// trash() trashes a card from the player's hand, adding it to the
	// shared trash pile. If it's not in the player's hand than a NotInHand
	// error is returned.
	fn trash(&mut self, c: Card) -> Option<error::Error> {
		match self.hand.iter().enumerate().find(|&(_,x)| *x == c) {
			None => Some(error::NotInHand),
			Some((i,_)) => {
				let card = self.hand.remove(i).unwrap();
                (*self.game_ref).borrow_mut().trash.push(card);
				None
			},
		}
	}

    fn trash_from_play(&mut self, c: Card) -> Option<error::Error> {
		match self.in_play.iter().enumerate().find(|&(_,x)| *x == c) {
			None => Some(error::NotInHand),
			Some((i,_)) => {
				let card = self.in_play.remove(i).unwrap();
                (*self.game_ref).borrow_mut().trash.push(card);
				None
			},
		}
    }

	// calculate_score() counts up the total number of points and saves it
	// in the local score variable.
	fn calculate_score(&mut self) {
		self.score = self.get_total_points();
	}

	fn with_other_players(&mut self, f: |&mut PlayerState|) {
        for &(_, ref mut other_player) in (*self.other_players).borrow_mut().mut_iter() {
            f(other_player);
        }
	}

    // attack() calls f on each other player, but only if they don't
    // have a Moat in hand.
    fn attack(&mut self, f: |&mut PlayerState|) {
        for &(_, ref mut other_player) in (*self.other_players).borrow_mut().mut_iter() {
            if other_player.hand_contains(card::MOAT) {
                continue;
            }
            f(other_player);
        }
    }

    /*
    #[allow(dead_code)]
	fn with_left_player<U>(&mut self, f: |&mut PlayerState| -> U) -> U {
        f((*self.other_players).borrow_mut().mut_iter().next().unwrap())
	}

    #[allow(dead_code)]
	fn with_right_player<U>(&mut self, f: |&mut PlayerState| -> U) -> U {
        f((*self.other_players).borrow_mut().mut_rev_iter().next().unwrap())
	}
    */

	fn with_mut_supply<U>(&mut self, f: |&mut Supply| -> U) -> U {
        f(&mut (*self.game_ref).borrow_mut().supply)
	}

	fn with_supply<U>(&mut self, f: |&Supply| -> U) -> U {
        f(&(*self.game_ref).borrow_mut().supply)
	}
}


/* ------------------------ Card Types ------------------------ */


enum CardType {
    Money(uint),
    Victory(VictoryFunc),
    Action(ActionFunc),
    Curse(int),
}

struct CardDef {
    name: &'static str,
    cost: uint,
    types: &'static [CardType],
}

// Hash card definitions by their name.
impl hash::Hash for CardDef {
    fn hash(&self, state: &mut hash::sip::SipState) {
        self.name.hash(state);
    }
}

impl Eq for CardDef {
	fn eq(&self, other: &CardDef) -> bool {
		self.name.eq(&other.name)
	}
}

impl TotalEq for CardDef { }

impl CardDef {
    fn create_copies(&'static self, n: int) -> Vec<Card> {
        let mut cards = Vec::new();
        for _ in range(0, n) {
            cards.push(self);
        }
        cards
    }

    #[inline]
    pub fn is_money(&self) -> bool {
        self.types.iter().any(|x| match *x {
            Money(_) => true,
            _ => false,
        })
    }

    #[inline]
    pub fn is_action(&self) -> bool {
        self.types.iter().any(|x| match *x {
            Action(_) => true,
            _ => false,
        })
    }

    #[inline]
    pub fn is_victory(&self) -> bool {
        self.types.iter().any(|x| match *x {
            Victory(_) => true,
            _ => false,
        })
    }

    #[inline]
    pub fn is_curse(&self) -> bool {
        self.types.iter().any(|x| match *x {
            Curse(_) => true,
            _ => false,
        })
    }

    #[inline]
    pub fn treasure_value(&self) -> uint {
        for t in self.types.iter() {
            match *t {
                Money(v) => return v,
                _ => (),
            }
        }
        fail!("Can't get treasure value of non-Money card!");
    }

    pub fn victory_points(&self, p: &PlayerState) -> int {
        for t in self.types.iter() {
            match *t {
                Victory(f) => return f(p),
                Curse(v) => return v,
                _ => (),
            }
        }
        fail!("Can't get victory point value of non-Victory and non-Curse card!");
    }

    #[inline]
    fn get_action(&self) -> ActionFunc {
        for t in self.types.iter() {
            match *t {
                Action(f) => return f,
                _ => (),
            }
        }
        fail!("Can't get action method of non-Action card!");
    }
}

pub enum ActionInput {
	Discard(Card),
	Trash(Card),
	Gain(Card),
    Confirm, // for "you may" effects, e.g. Chancellor
    Repeat(Card, fn(&PlayerState, uint) -> Vec<ActionInput>), // for "play several times" effects, e.g. Throne Room
}

impl ActionInput {
	pub fn is_discard(&self) -> bool {
		match *self {
			Discard(_) => true,
			_ => false,
		}
	}

	pub fn is_trash(&self) -> bool {
		match *self {
			Trash(_) => true,
			_ => false,
		}
	}

	pub fn is_gain(&self) -> bool {
		match *self {
			Gain(_) => true,
			_ => false,
		}
	}

    pub fn is_confirm(&self) -> bool {
        match *self {
            Confirm => true,
            _ => false,
        }
    }

    pub fn is_repeat(&self) -> bool {
        match *self {
            Repeat(_, _) => true,
            _ => false,
        }
    }

	pub fn unwrap(&self) -> Card {
		match *self {
			Discard(c) => c,
			Trash(c) => c,
            _ => fail!("Nothing to unwrap!"),
		}
	}
}


/* ------------------------ Misc Types ------------------------ */

type Supply = HashMap<Card, uint>;
type ActionFunc = fn(&mut PlayerState, &[ActionInput]);
type VictoryFunc = fn(&PlayerState) -> int;
pub type Card = &'static CardDef;
pub type PlayerFunc = fn(&mut PlayerState);
pub type PlayerTup = (Arc<~Player:Send+Share>, PlayerState);
