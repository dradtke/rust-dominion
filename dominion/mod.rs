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
use std::owned::Box;
use std::rc::Rc;
use std::task;
use std::vec::Vec;
use sync::Arc;
use rand::{Rng,task_rng};

pub mod card;
pub mod error;
pub mod strat;

#[macro_export]
macro_rules! play(
    ($($player:ident),+) => ({
        dominion::play(~[$(
            box $player as Box<dominion::Player:Send+Share>,
        )+]);
    })
)

local_data_key!(state_map: RefCell<HashMap<&'static str, PlayerState>>)
local_data_key!(active_player: &'static str)
local_data_key!(active_card: Card)


/* ------------------------ Player Trait ------------------------ */


pub trait Player {
    fn name(&self) -> &'static str;
    fn take_turn(&self);

    // The default strategy is to always set aside action cards.
    fn library_should_discard(&self, _: Card) -> bool {
        true
    }

    // The default strategy is to always block attacks.
    fn moat_should_block(&self, _: Card) -> bool {
        true
    }
}


/* ------------------------ Public Methods ------------------------ */


// The entry point for playing a game, usually used via the shorthand play!() macro.
pub fn play(player_list: ~[Box<Player:Send+Share>]) {
    println!("The contestants are:");
    for player in player_list.iter() {
        println!("\t{}", player.name());
    }

    let args = std::os::args();
    let n: uint = if args.len() > 1 { from_str(*args.get(1)).unwrap() } else { 1000 };
    println!("\nPlaying {} games...", n);

    let trash = Vec::new();

    let mut supply: Supply = collections::HashMap::new();
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

    let (reporter, receiver) = comm::channel();

    let player_arcs: Vec<Arc<Box<Player:Send+Share>>> = player_list.move_iter().map(|player| Arc::new(player)).collect();

    for _ in range(0, n) {
        let mut deck = Vec::new();
        deck.push_all_move(card::COPPER.create_copies(7));
        deck.push_all_move(card::ESTATE.create_copies(3));
        task_rng().shuffle(deck.as_mut_slice());

        let reporter = reporter.clone();
        let trash = trash.clone();
        let supply = supply.clone();
        let player_arcs = player_arcs.clone();

        spawn(proc() {
            match task::try(proc() {
                let players = Rc::new(RefCell::new(DList::<Arc<Box<Player:Send+Share>>>::new()));
                let game = Rc::new(RefCell::new(GameState{ supply: supply, trash: trash }));

                let mut player_state_map = HashMap::<&'static str, PlayerState>::new();

                for p in player_arcs.move_iter() {
                    player_state_map.insert(p.name(), PlayerState{
                        game_ref:      game.clone(),
                        other_players: players.clone(),
                        deck:          deck.clone(),
                        discard:       Vec::new(),
                        in_play:       Vec::new(),
                        hand:          Vec::new(),
                        actions:       0,
                        buys:          0,
                        buying_power:  0,
                        score:         0,
                    });
                    (*(*players).borrow_mut()).push_back(p);
                }

                state_map.replace(Some(RefCell::new(player_state_map)));

                play_game(players.clone())
            }) {
                Err(e) => {
                    reporter.send(Err(e));
                },
                Ok(results) => {
                    // TODO: send more information?
                    if results.tie {
                        reporter.send(Ok(None));
                    } else {
                        reporter.send(Ok(Some(results.winner.into_owned())));
                    }
                },
            }
        });
    }

    let mut scores = HashMap::<~str,uint>::new();
    let mut ties = 0;
    for _ in range(0, n) {
        match receiver.recv() {
            Err(e) => println!("Dominion task failed: {}", e),
            Ok(None) => ties += 1,
            Ok(Some(winner)) => { scores.insert_or_update_with(winner.clone(), 1, |_, v| *v += 1); },
        }
    }

    for key in scores.keys() {
        println!("{} won {} times", *key, *scores.get(key));
    }
    println!("There were {} ties.", ties);
}


// play_game() playes a single game of Dominion. It plays a game and then
// returns a vector of tuples with the player's name along with their final
// score, ordered from highest to lowest.
pub fn play_game(players: Rc<RefCell<DList<Arc<Box<Player:Send+Share>>>>>) -> GameResult {
    let empty_limit = get_empty_limit((*players).borrow().len());
    loop {
        let player = (*players).borrow_mut().pop_front().unwrap();
        active_player.replace(Some(player.name()));

        take_turn(&(*player));

        let done = with_active_player(|p| is_game_finished(&(*p.game_ref.borrow()), empty_limit));
        (*players).borrow_mut().push_back(player);

        if done {
            break;
        }
    }

    let mut player_results: Vec<PlayerResult> = (*players).borrow_mut().iter()
        .map(|p| {
            let name = p.name();
            with_player(name, |state| {
                PlayerResult{
                    name: name,
                    vp: state.calculate_score(),
                    victory_cards: state.deck.iter().filter_map(|&c| {
                        if c.is_victory() || c.is_curse() {
                            Some(c)
                        } else {
                            None
                        }
                    }).collect(),
                }
            })
        }).collect();
    player_results.sort_by(|a, ref b| b.vp.cmp(&a.vp));

    let highest_score = player_results.get(0).vp;
    let tie = player_results.iter().skip(1).any(|result| result.vp == highest_score);
    GameResult{
        tie: tie,
        winner: player_results.get(0).name,
        player_results: player_results,
    }
}


// get_available_money() returns a count of the total available money
// currently in the player's hand.
pub fn get_available_money() -> uint {
    with_active_player(|player| {
        player.hand.iter()
        .filter(|&c| c.is_money())
        .fold(0, |a, &b| a + b.treasure_value())
    })
}

// get_buying_power() returns the current available buying power from
// everything that's been played so far.
pub fn get_buying_power() -> uint {
    with_active_player(|player| player.buying_power)
}

// get_total_points() counts up the total point value from all victory
// and curse cards in the player's deck, hand, and discard.
pub fn get_total_points() -> int {
    with_active_player(|player| {
        player.deck.iter()
        .chain(player.discard.iter())
        .chain(player.hand.iter())
        .filter(|&c| c.is_victory() || c.is_curse())
        .fold(0, |a, &b| a + b.victory_points())
    })
}

// get_hand() returns a copy of the player's hand. The Card type
// is defined as a static pointer to a CardDef, so it's not as
// expensive as if it cloned the card definitions themselves, but
// is still more expensive than an implementation using an Arc
// or similar utility.
pub fn get_hand() -> Vec<Card> {
    with_active_player(|player| player.hand.clone())
}

// get_hand_size() returns the number of cards in the player's hand.
pub fn get_hand_size() -> uint {
    with_active_player(|player| player.hand.len())
}

// has() returns true if the player has the provided card, anywhere.
pub fn has(c: Card) -> bool {
    with_active_player(|player| {
        player.hand.iter().any(|&x| x == c)
        || player.deck.iter().any(|&x| x == c)
        || player.discard.iter().any(|&x| x == c)
        || player.in_play.iter().any(|&x| x == c)
    })
}

// number_of() returns the number of instances of the provided card
// that the player has, anywhere.
pub fn number_of(c: Card) -> uint {
    with_active_player(|player| {
        player.hand.iter().count(|&x| x == c)
        + player.deck.iter().count(|&x| x == c)
        + player.discard.iter().count(|&x| x == c)
        + player.in_play.iter().count(|&x| x == c)
    })
}

// hand_contains() returns true if and only if the player's hand contains
// the specified card.
pub fn hand_contains(c: Card) -> bool {
    with_active_player(|player| player.hand_contains(c))
}

pub fn play_card(c: Card) -> Option<error::Error> {
    play_card_and(c, [])
}

// play_card_and() plays a card. It returns an InvalidPlay error if either (a) the requested
// card is not in the player's hand, or (b) the card cannot be played, e.g. Province.
// Other errors may occur if there are not enough actions or buys, and once a Money
// card is played, then the player's action count is set to 0.
pub fn play_card_and(c: Card, input: &[ActionInput]) -> Option<error::Error> {
    if !c.is_money() && !c.is_action() {
        return Some(error::InvalidPlay);
    }
    let (action, error) = with_active_player(|player| -> (Option<ActionFunc>, Option<error::Error>) {
        match player.hand.iter().position(|&x| x == c) {
            None => (None, Some(error::InvalidPlay)),
            Some(index) => {
                player.in_play.push(player.hand.remove(index).unwrap());
                if c.is_money() {
                    player.buying_power += c.treasure_value();
                    player.actions = 0;
                }
                if c.is_action() {
                    if player.actions == 0 {
                        (None, Some(error::NoActions))
                    } else {
                        player.actions -= 1;
                        (Some(c.get_action()), None)
                    }
                } else {
                    (None, None)
                }
            }
        }
    });
    if action.is_some() {
        let f = action.unwrap();
        active_card.replace(Some(c));
        f(input);
        active_card.replace(None);
    }
    error
}

// play_all_money() is a utility method that iterates through the player's
// hand and calls play() on each money card.
pub fn play_all_money() {
    let hand = get_hand();
    for card in hand.iter().filter(|&c| c.is_money()) {
        play_card(*card);
    }
}

// buy() buys a card from the supply, returning one of three possible
// errors:
//   1. NotInSupply, if the card is not available in this game
//   2. EmptyPile, if there are no more available to buy
//   3. NotEnoughMoney(difference), if the player doesn't have the money
// On success, the appropriate supply count is decremented and a copy
// of the card is added to the player's discard pile.
pub fn buy(c: Card) -> Option<error::Error> {
    let pile = match count(c) {
        None => return Some(error::NotInSupply),
        Some(0) => return Some(error::EmptyPile),
        Some(pile) => pile,
    };
    with_active_player(|player| {
        if player.buying_power >= c.cost {
            player.with_mut_supply(|supply| supply.insert(c, pile - 1));
            player.discard.push(c);
            player.actions = 0;
            player.buying_power -= c.cost;
            None
        } else {
            Some(error::NotEnoughMoney(c.cost - player.buying_power))
        }
    })
}

// count() returns either the number available for a given card, or None
// if the card wasn't available in this game.
pub fn count(c: Card) -> Option<uint> {
    with_active_player(|player| player.count(c))
}


/* ------------------------ Private Methods ------------------------ */


fn take_turn(p: &Box<Player:Send+Share>) {
    with_active_player(|player| {
        player.new_hand();
        player.actions = 1;
        player.buys = 1;
        player.buying_power = 0;
    });
    p.take_turn();
    with_active_player(|player| {
        player.discard_hand();
    });
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

fn is_game_finished(game: &GameState, empty_limit: uint) -> bool {
    if *game.supply.find(&card::PROVINCE).unwrap() == 0 {
        true
    } else {
        let num_empty = game.supply.values().filter(|&x| *x == 0).fold(0, |a, &b| a + b);
        num_empty >= empty_limit
    }
}

fn with_player<T>(player: &'static str, f: |&mut PlayerState| -> T) -> T {
    let result: T = f(state_map.get().unwrap().borrow_mut().get_mut(&player));
    result
}

fn with_active_player<T>(f: |&mut PlayerState| -> T) -> T {
    match active_player.get() {
        None => fail!("No active player!"),
        Some(player) => with_player(*player, f),
    }
}

fn with_other_players(f: |&mut PlayerState|) {
    let others = with_active_player(|player| player.other_players.clone());
    let state_map_ref = state_map.get().unwrap();
    let mut states = state_map_ref.borrow_mut();
    for other in (*others).borrow_mut().iter() {
        let state = states.get_mut(&other.name());
        f(state);
    }
}

// attack() calls f on each other player, but only if they don't
// have a Moat in hand and want to block it.
fn attack(f: |&mut PlayerState|) {
    let others = with_active_player(|player| player.other_players.clone());
    let state_map_ref = state_map.get().unwrap();
    let mut states = state_map_ref.borrow_mut();
    for other in (*others).borrow_mut().iter() {
        let state = states.get_mut(&other.name());
        let attacker = *active_card.get().unwrap();
        if !state.hand_contains(card::MOAT) || !(**other).moat_should_block(attacker) {
            f(state);
        }
    }
}


/* ------------------------ Types ------------------------ */


// TODO: find a way to derive Default
pub struct PlayerState {
    game_ref: Rc<RefCell<GameState>>,
    other_players: Rc<RefCell<DList<Arc<Box<Player:Send+Share>>>>>,

	deck: Vec<Card>,
	discard: Vec<Card>,
	in_play: Vec<Card>,
	hand: Vec<Card>,

	actions: uint,
	buys: uint,
	buying_power: uint,
	score: int, // for calculating the final score
}

#[deriving(Clone)]
pub struct GameState {
    pub supply: Supply,
    pub trash: Vec<Card>,
}

pub enum ActionInput {
	Discard(Card),
	Trash(Card),
	Gain(Card),
    Confirm, // for "you may" effects, e.g. Chancellor
    Repeat(Card, fn(uint) -> Vec<ActionInput>), // for "play several times" effects, e.g. Throne Room
}

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

pub struct GameResult {
    tie: bool,
    winner: &'static str,
    player_results: Vec<PlayerResult>,
}

pub struct PlayerResult {
    name: &'static str,
    vp: int,
    victory_cards: Vec<Card>,
}


// Aliases

pub type Supply = HashMap<Card, uint>;

pub type ActionFunc = fn(&[ActionInput]);

pub type VictoryFunc = fn() -> int;

pub type Card = &'static CardDef;

pub type PlayerFunc = fn(&mut PlayerState);


/* ------------------------ PlayerState Impl ------------------------ */


impl PlayerState {
    fn hand_contains(&mut self, c: Card) -> bool {
        self.hand.iter().any(|&x| x == c)
    }

    // gain() takes a card from the supply, putting it in the discard pile.
    fn gain(&mut self, c: Card) -> Option<error::Error> {
        let pile = match count(c) {
            None => return Some(error::NotInSupply),
            Some(0) => return Some(error::EmptyPile),
            Some(pile) => pile,
        };
        self.with_mut_supply(|supply| supply.insert(c, pile - 1));
        self.discard.push(c);
        None
    }

    // gain_to_deck() takes a card from the supply, putting it on top of
    // the deck.
    fn gain_to_deck(&mut self, c: Card) -> Option<error::Error> {
        let pile = match count(c) {
            None => return Some(error::NotInSupply),
            Some(0) => return Some(error::EmptyPile),
            Some(pile) => pile,
        };
        self.with_mut_supply(|supply| supply.insert(c, pile - 1));
        self.deck.unshift(c);
        None
    }

    // gain_to_hand() takes a card from the supply, putting it into
    // the hand.
    fn gain_to_hand(&mut self, c: Card) -> Option<error::Error> {
        let pile = match count(c) {
            None => return Some(error::NotInSupply),
            Some(0) => return Some(error::EmptyPile),
            Some(pile) => pile,
        };
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

    fn count(&mut self, c: Card) -> Option<uint> {
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
            task_rng().shuffle(self.deck.as_mut_slice());
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

	fn draw(&mut self) -> Option<Card> {
        match self.next_card() {
            Some(c) => {
                self.hand.push(c);
                Some(c)
            }
            None => None
        }
	}

    fn remove_from_hand(&mut self, c: Card) -> bool {
		match self.hand.iter().enumerate().find(|&(_,&x)| x == c) {
            None => false,
            Some((i,_)) => {
                self.hand.remove(i);
                true
            }
        }
    }

	// discard() discards a card from the player's hand, adding it to the
	// discard pile. If it's not in the player's hand than a NotInHand
	// error is returned.
	fn discard(&mut self, c: Card) -> Option<error::Error> {
        if !self.remove_from_hand(c) {
            Some(error::NotInHand)
        } else {
            self.discard.push(c);
            None
        }
	}

	fn discard_first(&mut self) {
        self.discard.push(self.hand.remove(0).unwrap());
	}

	// trash() trashes a card from the player's hand, adding it to the
	// shared trash pile. If it's not in the player's hand than a NotInHand
	// error is returned.
	fn trash(&mut self, c: Card) -> Option<error::Error> {
        if !self.remove_from_hand(c) {
            Some(error::NotInHand)
        } else {
            (*self.game_ref).borrow_mut().trash.push(c);
            None
        }
	}

    fn trash_from_play(&mut self, c: Card) -> Option<error::Error> {
		match self.in_play.iter().enumerate().find(|&(_,&x)| x == c) {
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
	fn calculate_score(&mut self) -> int {
        self.deck.iter()
            .chain(self.discard.iter())
            .chain(self.hand.iter())
            .filter(|&c| c.is_victory() || c.is_curse())
            .fold(0, |a, &b| a + b.victory_points())
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


/* ------------------------ CardDef Impl ------------------------ */


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
    #[allow(dead_code)]
    pub fn create_copies(&'static self, n: int) -> Vec<Card> {
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

    pub fn victory_points(&self) -> int {
        for t in self.types.iter() {
            match *t {
                Victory(f) => return f(),
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


/* ------------------------ ActionInput Impl ------------------------ */


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
