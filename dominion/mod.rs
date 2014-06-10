#![crate_id = "dominion#0.1"]
#![crate_type = "lib"]

#![feature(globs)]
#![feature(macro_rules)]
#![allow(unused_must_use)]

extern crate sync;
extern crate term;

use std::fmt;
use std::cell::RefCell;
use std::collections::{Deque,DList,HashMap};
use std::comm;
use std::mem;
use std::owned::Box;
use std::rc::Rc;
use std::task;
use std::vec::Vec;
use sync::Arc;
use std::rand::{task_rng,Rng};

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

    // library_should_discard() is called when an Action card is encountered as part of
    // a Library draw. It should return true if that card should be discarded,
    // and false if it should be kept.
    //
    // DEFAULT: Always discard Action cards.
    fn library_should_discard(&self, _: Card) -> bool {
        true
    }

    // militia_discard() is called when another player plays Militia, and is called
    // repeatedly until you have three or fewer cards in hand. Given a list of
    // cards in your hand, it should return the one that you wish to discard.
    //
    // DEFAULT: Discard the first card (TODO: make this default a little better)
    fn militia_discard(&self, options: &[Card]) -> Card {
        options[0]
    }

    // moat_should_block() is called when another player plays an attack card
    // while you have a Moat in hand. It should return true if you wish to block
    // the attack, otherwise false.
    //
    // DEFAULT: Always block attacks. Why wouldn't you?
    fn moat_should_block(&self, _: Card) -> bool {
        true
    }

    // spy_should_discard() is called when a Spy is played, including by you.
    // Given the value of the top card of a player's deck, this method should
    // return true if that card should be discarded, and false if it should
    // be returned to the top of the player's deck. The value of `is_self` is
    // true if and only if you are the player being acted on.
    //
    // DEFAULT: Keep victory and curse cards on top for other players, discard them
    // for yourself.
    fn spy_should_discard(&self, c: Card, is_self: bool) -> bool {
        let is_worthless = c.is_victory() || c.is_curse();
        if is_self { is_worthless } else { !is_worthless }
    }

    fn bureaucrat_use_victory(&self, options: &[Card]) -> Card {
        options[0]
    }

    // thief_tash_and_keep() is called when you play Thief and someone reveals
    // one or more treasure cards. `options` contains at least one card
    // (but no more than 2), and it should return a tuple describing how to
    // treat the reveal. The first value is the card that should be trashed,
    // and the second value is a boolean indicating whether or not it should
    // be kept.
    //
    // DEFAULT: Always trash the highest value treasure card, and only keep it
    // if it isn't a Copper.
    fn thief_trash_and_keep(&self, options: &[Card]) -> (Card, bool) {
        let mut money = Vec::from_slice(options);
        money.sort_by(|m1, m2| m2.treasure_value().cmp(&m1.treasure_value())); // TODO: verify the ordering, highest should be first
        let highest = *money.get(0);
        (highest, highest != card::COPPER)
    }
}


/* ------------------------ Public Methods ------------------------ */

fn report(term: &mut Box<term::Terminal<Box<Writer:Send>>:Send>, games: uint, total_games: uint, scores: &HashMap<String, uint>, ties: uint) {
    let winning = match scores.iter().max_by(|&(_, v)| v) {
        Some((_, v)) => *v,
        _ => 0,
    };
    term.write_str("\r");
    for (i, (key, value)) in scores.iter().enumerate() {
        if i > 0 {
            term.write_str(" \t");
        }
        write!(term, "{}: ", *key);
        term.fg(if *value == winning { term::color::BRIGHT_GREEN } else { term::color::BRIGHT_RED });
        write!(term, "{}", *value);
        term.reset();
    }
    write!(term, "\tTies: {} \tTotal Played: {}/{}", ties, games, total_games);
    term.flush();
}

// The entry point for playing a game, usually used via the shorthand play!() macro.
pub fn play(player_list: ~[Box<Player:Send+Share>]) {
    let mut term = term::stdout().unwrap();
    let args = std::os::args();
    let n: uint = if args.len() > 1 { from_str(args.get(1).as_slice()).unwrap() } else { 1000 };
    writeln!(term, "\nPlaying {} games...", n);

    let trash = Vec::new();

    let mut supply: Supply = HashMap::new();
    supply.insert(card::COPPER.to_str(),   30);
    supply.insert(card::SILVER.to_str(),   30);
    supply.insert(card::GOLD.to_str(),     30);
    supply.insert(card::ESTATE.to_str(),   12);
    supply.insert(card::DUCHY.to_str(),    12);
    supply.insert(card::PROVINCE.to_str(), 12);
    supply.insert(card::CURSE.to_str(),    30);
    // now for the variations!
    supply.insert(card::SMITHY.to_str(), 10);
    supply.insert(card::WITCH.to_str(),  10);

    let (reporter, receiver) = comm::channel();

    let player_arcs: Vec<Arc<Box<Player:Send+Share>>> = player_list.move_iter().map(|player| Arc::new(player)).collect();

    let mut scores = HashMap::<String,uint>::new();
    for player in player_arcs.iter() {
        scores.insert(player.name().to_str(), 0);
    }

    spawn(proc() {
        for _ in range(0, n) {
            let reporter = reporter.clone();
            let trash = trash.clone();
            let supply = supply.clone();
            let player_arcs = player_arcs.clone();

            spawn(proc() {
                match task::try(proc() {
                    let mut rng = task_rng();

                    let mut player_arcs = player_arcs;
                    rng.shuffle(player_arcs.as_mut_slice());

                    let mut deck = Vec::new();
                    deck.push_all_move(card::COPPER.create_copies(7));
                    deck.push_all_move(card::ESTATE.create_copies(3));
                    rng.shuffle(deck.as_mut_slice());

                    let players = Rc::new(RefCell::new(DList::<Arc<Box<Player:Send+Share>>>::new()));
                    let game = Rc::new(RefCell::new(GameState{ supply: supply, trash: trash }));
                    let mut player_state_map = HashMap::<&'static str, PlayerState>::new();
                    let other_players: PlayerList = player_arcs.clone().move_iter().collect();

                    for p in player_arcs.move_iter() {
                        let mut other_players = other_players.clone();
                        while other_players.front().unwrap().name() != p.name() {
                            other_players.rotate_backward();
                        }
                        other_players.pop_front();
                        player_state_map.insert(p.name(), PlayerState{
                            game_ref:      game.clone(),
                            myself:        p.clone(),
                            other_players: other_players,
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

                    play_game(players)
                }) {
                    Err(e) => {
                        reporter.send(Err(e));
                    },
                    Ok(results) => {
                        // TODO: send more information?
                        if results.tie {
                            reporter.send(Ok(None));
                        } else {
                            reporter.send(Ok(Some(results.winner.into_string())));
                        }
                    },
                }
            });
        }
    });

    let mut ties = 0;
    report(&mut term, 0, n, &scores, ties);

    for i in range(0, n) {
        match receiver.recv() {
            Err(e) => fail!("Dominion task failed: {}", e),
            Ok(None) => ties += 1,
            Ok(Some(ref winner)) => {
                scores.insert_or_update_with(winner.clone(), 1, |_, v| *v += 1);
            },
        }
        report(&mut term, i+1, n, &scores, ties);
    }

    term.write_line("");
}


// play_game() playes a single game of Dominion. It plays a game and then
// returns a vector of tuples with the player's name along with their final
// score, ordered from highest to lowest.
pub fn play_game(players: Rc<RefCell<PlayerList>>) -> GameResult {
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

pub fn get_action_count() -> uint {
    with_active_player(|player| player.actions)
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
        player.hand.iter().filter(|&x| x == &c).count()
        + player.deck.iter().filter(|&x| x == &c).count()
        + player.discard.iter().filter(|&x| x == &c).count()
        + player.in_play.iter().filter(|&x| x == &c).count()
    })
}

// hand_contains() returns true if and only if the player's hand contains
// the specified card.
pub fn hand_contains(c: Card) -> bool {
    with_active_player(|player| player.hand_contains(c))
}

// get_trash() returns a clone of the game's trash pile.
pub fn get_trash() -> Vec<Card> {
    with_active_player(|player| (*player.game_ref).borrow().trash.clone())
}

// play_card() plays a card with no input parameters.
pub fn play_card(c: Card) -> DominionResult {
    play_card_and(c, [])
}

// play_card_and() plays a card. It returns an InvalidPlay error if either (a) the requested
// card is not in the player's hand, or (b) the card cannot be played, e.g. Province.
// Other errors may occur if there are not enough actions or buys, and once a Money
// card is played, then the player's action count is set to 0.
pub fn play_card_and(c: Card, input: &[ActionInput]) -> DominionResult {
    if !c.is_money() && !c.is_action() {
        return Err(error::InvalidPlay);
    }
    let (action, result) = with_active_player(|player| -> (Option<ActionFunc>, DominionResult) {
        match player.hand.iter().position(|&x| x == c) {
            None => (None, Err(error::InvalidPlay)),
            Some(index) => {
                player.in_play.push(player.hand.remove(index).unwrap());
                if c.is_money() {
                    player.buying_power += c.treasure_value();
                    player.actions = 0;
                }
                if c.is_action() {
                    if player.actions == 0 {
                        (None, Err(error::NoActions))
                    } else {
                        player.actions -= 1;
                        (Some(c.get_action()), Ok(()))
                    }
                } else {
                    (None, Ok(()))
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
    result
}

// play_all_money() is a utility method that iterates through the player's
// hand and calls play() on each money card.
pub fn play_all_money() {
    let hand = get_hand();
    for card in hand.iter().filter(|&c| c.is_money()) {
        play_card(*card).unwrap();
    }
}

// buy() buys a card from the supply, returning one of three possible
// errors:
//   1. NotInSupply, if the card is not available in this game
//   2. EmptyPile, if there are no more available to buy
//   3. NotEnoughMoney(difference), if the player doesn't have the money
// On success, the appropriate supply count is decremented and a copy
// of the card is added to the player's discard pile.
pub fn buy(c: Card) -> DominionResult {
    let pile = match count(c) {
        None => return Err(error::NotInSupply),
        Some(0) => return Err(error::EmptyPile),
        Some(pile) => pile,
    };
    with_active_player(|player| {
        if player.buying_power >= c.cost {
            player.with_mut_supply(|supply| supply.insert(c.to_str(), pile - 1));
            player.discard.push(c);
            player.actions = 0;
            player.buying_power -= c.cost;
            Ok(())
        } else {
            Err(error::NotEnoughMoney(c.cost - player.buying_power))
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
    if *game.supply.find(&card::PROVINCE.to_str()).unwrap() == 0 {
        true
    } else {
        let num_empty = game.supply.iter().filter(|&(_, &x)| x == 0).fold(0, |a, (_, &b)| a + b);
        num_empty >= empty_limit
    }
}

fn with_player<T>(player: &'static str, f: |&mut PlayerState| -> T) -> T {
    let result: T = f((*state_map.get().unwrap().borrow_mut()).get_mut(&player));
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
    let states_ref = state_map.get().unwrap();
    let mut states = states_ref.borrow_mut();
    for other in others.iter() {
        f(states.get_mut(&other.name()));
    }
}

// attack() calls f on each other player, but only if they don't
// have a Moat in hand and want to block it.
fn attack(f: |&mut PlayerState|) {
    let others = with_active_player(|player| player.other_players.clone());
    let states_ref = state_map.get().unwrap();
    let mut states = states_ref.borrow_mut();
    for other in others.iter() {
        let state = states.get_mut(&other.name());
        let attacker = *active_card.get().unwrap();
        if !state.hand_contains(card::MOAT) || !(**other).moat_should_block(attacker) {
            f(state);
        }
    }
}


/* ------------------------ PlayerState ------------------------ */

// TODO: find a way to derive Default
pub struct PlayerState {
    game_ref: Rc<RefCell<GameState>>,
    myself: Arc<Box<Player:Send+Share>>,
    other_players: PlayerList,

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
    fn hand_contains(&mut self, c: Card) -> bool {
        self.hand.iter().any(|&x| x == c)
    }

    // gain() takes a card from the supply, putting it in the discard pile.
    fn gain(&mut self, c: Card) -> DominionResult {
        let pile = match count(c) {
            None => return Err(error::NotInSupply),
            Some(0) => return Err(error::EmptyPile),
            Some(pile) => pile,
        };
        self.with_mut_supply(|supply| supply.insert(c.to_str(), pile - 1));
        self.discard.push(c);
        Ok(())
    }

    // gain_to_deck() takes a card from the supply, putting it on top of
    // the deck.
    fn gain_to_deck(&mut self, c: Card) -> DominionResult {
        let pile = match count(c) {
            None => return Err(error::NotInSupply),
            Some(0) => return Err(error::EmptyPile),
            Some(pile) => pile,
        };
        self.with_mut_supply(|supply| supply.insert(c.to_str(), pile - 1));
        self.deck.unshift(c);
        Ok(())
    }

    // gain_to_hand() takes a card from the supply, putting it into
    // the hand.
    fn gain_to_hand(&mut self, c: Card) -> DominionResult {
        let pile = match count(c) {
            None => return Err(error::NotInSupply),
            Some(0) => return Err(error::EmptyPile),
            Some(pile) => pile,
        };
        self.with_mut_supply(|supply| supply.insert(c.to_str(), pile - 1));
        self.hand.unshift(c);
        Ok(())
    }

	// curse() gives the player a curse card and depletes one from the supply.
	fn curse(&mut self) -> DominionResult {
		let pile = self.count(card::CURSE).unwrap();
		if pile == 0 {
			Err(error::EmptyPile)
		} else {
			self.with_mut_supply(|supply| supply.insert(card::CURSE.to_str(), pile - 1));
			self.discard.push(card::CURSE);
			Ok(())
		}
	}

    fn count(&mut self, c: Card) -> Option<uint> {
        self.with_supply(|supply| {
            match supply.find(&c.to_str()) {
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

    #[allow(dead_code)]
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
	fn discard(&mut self, c: Card) -> DominionResult {
        if !self.remove_from_hand(c) {
            Err(error::NotInHand)
        } else {
            self.discard.push(c);
            Ok(())
        }
	}

	// trash() trashes a card from the player's hand, adding it to the
	// shared trash pile. If it's not in the player's hand than a NotInHand
	// error is returned.
	fn trash(&mut self, c: Card) -> DominionResult {
        if !self.remove_from_hand(c) {
            Err(error::NotInHand)
        } else {
            (*self.game_ref).borrow_mut().trash.push(c);
            Ok(())
        }
	}

    fn trash_from_play(&mut self, c: Card) -> DominionResult {
		match self.in_play.iter().enumerate().find(|&(_,&x)| x == c) {
			None => Err(error::NotInHand),
			Some((i,_)) => {
				let card = self.in_play.remove(i).unwrap();
                (*self.game_ref).borrow_mut().trash.push(card);
				Ok(())
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


/* ------------------------ GameState ------------------------ */

#[deriving(Clone)]
pub struct GameState {
    pub supply: Supply,
    pub trash: Vec<Card>,
}


/* ------------------------ ActionInput ------------------------ */

pub enum ActionInput {
	Discard(Card),
	Trash(Card),
	Gain(Card),
    Confirm, // for "you may" effects, e.g. Chancellor
    Repeat(Card, fn(uint) -> Vec<ActionInput>), // for "play several times" effects, e.g. Throne Room
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

	pub fn get_card(&self) -> Card {
		match *self {
			Discard(c) => c,
			Trash(c) => c,
            Gain(c) => c,
            _ => fail!("Can't get card of unsupported input type!"),
		}
	}
}


/* ------------------------ CardType ------------------------ */

enum CardType {
    Money(uint),
    Victory(VictoryFunc),
    Action(ActionFunc),
    Curse(int),
}

impl PartialEq for CardType {
    fn eq(&self, other: &CardType) -> bool {
        self.to_str().eq(&other.to_str())
    }
}

impl fmt::Show for CardType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match *self {
            Money(_)   => "Money",
            Victory(_) => "Victory",
            Action(_)  => "Action",
            Curse(_)   => "Curse",
        })
    }
}


/* ------------------------ CardDef ------------------------ */

struct CardDef {
    name: &'static str,
    cost: uint,
    types: &'static [CardType],
}

impl PartialEq for CardDef {
	fn eq(&self, other: &CardDef) -> bool {
		self.name.eq(&other.name)
	}
}

impl fmt::Show for CardDef {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

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


/* ------------------------ GameResult ------------------------ */

pub struct GameResult {
    tie: bool,
    winner: &'static str,
    player_results: Vec<PlayerResult>,
}


/* ------------------------ PlayerResult ------------------------ */

pub struct PlayerResult {
    name: &'static str,
    vp: int,
    victory_cards: Vec<Card>,
}


/* ------------------------ Aliases ------------------------ */

pub type ActionFunc = fn(&[ActionInput]);

pub type Card = &'static CardDef;

pub type DominionResult = Result<(), error::Error>;

pub type PlayerFunc = fn(&mut PlayerState);

pub type PlayerList = DList<Arc<Box<Player:Send+Share>>>;

pub type Supply = HashMap<String, uint>;

pub type VictoryFunc = fn() -> int;
