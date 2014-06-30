#![crate_id = "dominion#0.1.0"]
#![crate_type = "lib"]

//! This module provides an API for writing Dominion AI's in Rust. AI's are created by
//! defining a new empty struct and implementing the `Player` trait, like so:
//!
//! ~~~
//! #[phase(plugin, link)] extern crate dominion;
//!
//! struct Me;
//! impl dominion::Player for Me {
//!     fn name() -> &'static str { "Me" }
//!     fn take_turn() {
//!         dominion::strat::big_money();
//!     }
//! }
//!
//! struct Them;
//! impl dominion::Player for Them {
//!     fn name() -> &'static str { "Them" }
//!     fn take_turn() {
//!         dominion::strat::big_money();
//!     }
//! }
//!
//! fn main() {
//!     dominion!(Me, Them);
//! }
//! ~~~
//!
//! This example is pretty simple, but demonstrates how new AI's are created. Both
//! of these players use the built-in Big Money strategy, which is a good first test
//! for successful strategies; if you can't beat Big Money reliably, then you really
//! need to think about going back to the drawing board.
//!
//! Strategies can be made more complex by overriding the other methods provided by
//! the `Player` trait. Otherwise, everything takes place in `take_turn()`, which will
//! be executed every time it's your AI's turn to play.
//!
//! Actions like playing and buying cards are achieved by using the public methods
//! exposed by this module. The game keeps track of who the active player is, so
//! there's no need to provide any player information in these methods.
//!
//! This library by default plays 1,000 games, unless a different number is
//! specified as an argument. For example, compiling the above example into an
//! executable called `main` and running `./main` will play 1,000 games, but running
//! `./main 100` will only play 100.

#![feature(globs)]
#![feature(struct_variant)]
#![feature(macro_rules)]
#![allow(unused_must_use)]

extern crate getopts;
extern crate sync;
extern crate term;

use std::fmt;
use std::cell::RefCell;
use std::collections::{Deque,DList,HashMap};
use std::comm;
use std::io::{File};
use std::mem;
use std::os;
use std::owned::Box;
use std::rc::Rc;
use std::task;
use std::string::String;
use std::vec::Vec;
use sync::Arc;
use std::rand::{task_rng,Rng};
use term::{Terminal,WriterWrapper,stdout};
use term::color;

pub mod card;
pub mod strat;

/// Play Dominion.
///
/// This macro takes the identifiers of the player types you intend to use,
/// which should be empty structs implementing `Player`.
#[macro_export]
macro_rules! dominion(
    ($($player:ident),+) => ({
        dominion::play(vec!($(
            box $player as Box<dominion::Player + Send + Share>,
        )+));
    })
)

#[macro_export]
macro_rules! kingdom(
    ($($card:ident),+) => ({
        dominion::set_kingdom(vec!($(
            dominion::card::$card,
        )+));
    })
)

// Game setup keys.
local_data_key!(KINGDOM: Vec<Card>)

// Game-specific keys.
local_data_key!(STATE_MAP: RefCell<HashMap<&'static str, PlayerState>>)
local_data_key!(ACTIVE_PLAYER: &'static str)
local_data_key!(ACTIVE_CARD: Card)


/* ------------------------ Player Trait ------------------------ */


/// A player definition.
///
/// The only required methods are `name()` and `take_turn()`,
/// but other methods may be overridden in order to gain more control over
/// your player.
pub trait Player {
    fn name(&self) -> &'static str;
    fn take_turn(&self);

    // init() is called before the first turn is played, and it passes in
    // a list of the cards that will be used this game. It can be used to
    // let a player decide what strategy they wish to use.
    fn init(&mut self, _: &Vec<Card>) {
    }

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
        // TODO: verify the ordering, highest should be first
        money.sort_by(|m1, m2| m2.treasure_value().cmp(&m1.treasure_value()));
        let highest = *money.get(0);
        (highest, highest != card::COPPER)
    }
}


/* ------------------------ Public Methods ------------------------ */


/// Buy a card from the supply, returning one of three possible
/// errors:
///
///   1. NotInSupply, if the card is not available in this game
///   2. EmptyPile, if there are no more available to buy
///   3. NotEnoughMoney(need, have), if the player doesn't have the money
///
/// On success, the appropriate supply count is decremented and a copy
/// of the card is added to the player's discard pile.
pub fn buy(c: Card) -> Result {
    let pile = match count(c) {
        None => return Err(NotInSupply(c)),
        Some(0) => return Err(EmptyPile(c)),
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
            Err(NotEnoughMoney{need: c.cost, have: player.buying_power})
        }
    })
}

/// Returns either the number available for a given card, or None
/// if the card wasn't available in this game.
pub fn count(c: Card) -> Option<uint> {
    with_active_player(|player| player.count(c))
}

/// Get the number of actions left for the current player.
pub fn get_action_count() -> uint {
    with_active_player(|player| player.actions)
}

/// Get a count of the total available money currently in the player's hand.
pub fn get_available_money() -> uint {
    with_active_player(|player| {
        player.hand.iter()
        .filter(|&c| c.is_money())
        .fold(0, |a, &b| a + b.treasure_value())
    })
}

/// Get the current available buying power.
///
/// You must have played at least one Money card or Action card providing
/// buying power for this value to be higher than 0.
pub fn get_buying_power() -> uint {
    with_active_player(|player| player.buying_power)
}

/// Get a copy of the player's hand.
///
/// The Card type is defined as a static pointer to a CardDef, so it's not as
/// expensive as if it cloned the card definitions themselves, but
/// is still more expensive than an implementation using an Arc
/// or similar utility.
pub fn get_hand() -> Vec<Card> {
    with_active_player(|player| player.hand.clone())
}

/// Get the number of cards in the player's hand.
pub fn get_hand_size() -> uint {
    with_active_player(|player| player.hand.len())
}

/// Get the total point value from all victory
/// and curse cards in the player's deck, hand, and discard.
pub fn get_total_points() -> int {
    with_active_player(|player| {
        player.deck.iter()
        .chain(player.discard.iter())
        .chain(player.hand.iter())
        .filter(|&c| c.is_victory() || c.is_curse())
        .fold(0, |a, &b| a + b.victory_points())
    })
}

/// Get a clone of the game's trash pile.
pub fn get_trash() -> Vec<Card> {
    with_active_player(|player| (*player.game_ref).borrow().trash.clone())
}

/// Returns true if and only if the player's hand contains
/// the specified card.
pub fn hand_contains(c: Card) -> bool {
    with_active_player(|player| player.hand_contains(c))
}

/// Returns true if and only if the player has the provided card in their
/// hand, deck, discard, or in play.
pub fn has(c: Card) -> bool {
    with_active_player(|player| {
        player.hand.iter().any(|&x| x == c)
        || player.deck.iter().any(|&x| x == c)
        || player.discard.iter().any(|&x| x == c)
        || player.in_play.iter().any(|&x| x == c)
    })
}

/// Returns the number of instances of the provided card
/// that the player has in their hand, deck, discard, or in play.
pub fn number_of(c: Card) -> uint {
    with_active_player(|player| {
        player.hand.iter().filter(|&x| x == &c).count()
        + player.deck.iter().filter(|&x| x == &c).count()
        + player.discard.iter().filter(|&x| x == &c).count()
        + player.in_play.iter().filter(|&x| x == &c).count()
    })
}

/// The entry point for playing a game, usually used via the shorthand `play!` macro.
pub fn play(player_list: Vec<Box<Player + Send + Share>>) {
    let mut term = stdout().unwrap();

    let args = os::args().iter().map(|x| x.to_string()).collect::<Vec<String>>();
    let opts = [
        getopts::optopt("o", "output", "set debug output file name", "NAME"),
    ];
    let matches = match getopts::getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(f) => fail!(f.to_str()),
    };
    let output_name = matches.opt_str("o");

    let n: uint = if !matches.free.is_empty() {
            from_str(matches.free.get(0).as_slice()).unwrap()
        } else {
            1000
        };

    let trash = Vec::new();
    let mut supply = build_supply();
    let kingdom = build_kingdom();
    let (reporter, receiver) = comm::channel();
    let mut player_arcs = Vec::with_capacity(player_list.len());
    let mut scores = HashMap::<String,uint>::new();

    write!(term, "\nPlaying {} games with ", n);
    for (i, card) in kingdom.iter().enumerate() {
        write!(term, "{}", card.name);
        if i < 9 {
            write!(term, ", ");
        }
        if i == 8 {
            write!(term, "and ");
        }
        supply.insert(card.to_str(), 10);
    }
    writeln!(term, ".");

    for mut player in player_list.move_iter() {
        player.init(&kingdom);
        scores.insert(player.name().to_str(), 0);
        player_arcs.push(Arc::new(player));
    }

    spawn(proc() {
        for _ in range(0u, n) {
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

                    let players = Rc::new(RefCell::new(DList::<Arc<Box<Player + Send + Share>>>::new()));
                    let game = Rc::new(RefCell::new(GameState{ supply: supply, trash: trash }));
                    let mut player_state_map = HashMap::<&'static str, PlayerState>::new();
                    let other_players = player_arcs.clone().move_iter().collect::<PlayerList>();

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
                        });
                        (*(*players).borrow_mut()).push_back(p);
                    }

                    STATE_MAP.replace(Some(RefCell::new(player_state_map)));

                    play_game(players)
                }) {
                    Err(e) => {
                        reporter.send(Err(e));
                    },
                    Ok(results) => reporter.send(Ok(results)),
                }
            });
        }
    });

    let mut ties = 0;
    report(&mut term, 0, n, &scores, ties);
    let mut output_file = output_name.clone().map(|x| File::create(&Path::new(x)).unwrap());

    for i in range(0, n) {
        match receiver.recv() {
            Err(_) => fail!("Dominion task failed. =("), // TODO: get the error message somehow
            Ok(results) => {
                if results.tie {
                    ties += 1;
                    output_file.mutate(|mut f| { f.write_line("[tie]"); f });
                } else {
                    scores.insert_or_update_with(String::from_str(results.winner), 1, |_, v| *v += 1);
                    output_file.mutate(|mut f| { writeln!(f, "[winner: {}]", results.winner); f });
                }
            },
        }
        report(&mut term, i+1, n, &scores, ties);
    }

    output_file.mutate(|mut f| { f.fsync(); f });
    term.write_line("");
    match output_name {
        None    => (),
        Some(x) => { writeln!(term, "Results saved to {}.", x); },
    };
}

/// Plays all Money cards in the player's hand.
pub fn play_all_money() {
    let hand = get_hand();
    for card in hand.iter().filter(|&c| c.is_money()) {
        play_card(*card).unwrap();
    }
}

/// Play a card with no input parameters. See `play_card_and()`.
pub fn play_card(c: Card) -> Result {
    play_card_and(c, [])
}

/// Play a card.
///
/// This method returns an InvalidPlay error if either
///
///     (a) the requested card is not in the player's hand, or
///     (b) the card cannot be played, e.g. Province.
///
/// Other errors may occur if there are not enough actions or buys, and once a Money
/// card is played, then the player's action count is set to 0.
pub fn play_card_and(c: Card, input: &[ActionInput]) -> Result {
    if !c.is_money() && !c.is_action() {
        return Err(InvalidPlay(c));
    }
    let (action, result) = with_active_player(|player| -> (Option<ActionFunc>, Result) {
        match player.hand.iter().position(|&x| x == c) {
            None => (None, Err(InvalidPlay(c))),
            Some(index) => {
                player.in_play.push(player.hand.remove(index).unwrap());
                if c.is_money() {
                    player.buying_power += c.treasure_value();
                    player.actions = 0;
                }
                if c.is_action() {
                    if player.actions == 0 {
                        (None, Err(NoActions))
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
        ACTIVE_CARD.replace(Some(c));
        f(input);
        ACTIVE_CARD.replace(None);
    }
    result
}

/// Sets the kingdom to be used.
pub fn set_kingdom(cards: Vec<Card>) {
    KINGDOM.replace(Some(cards));
}


/* ------------------------ Private Methods ------------------------ */

fn build_kingdom() -> Vec<Card> {
    let mut kingdom = match KINGDOM.get() {
        None => Vec::with_capacity(10),
        Some(x) => {
            let mut k = x.clone();
            k.reserve(10);
            k
        }
    };

    if kingdom.len() < 10 {
        let mut rng = task_rng();
        let mut all = card::dominion_set();
        for c in kingdom.iter() {
            all.remove(&c.name);
        }
        while kingdom.len() < 10 {
            let card = *rng.choose(all.iter().map(|x| *x).collect::<Vec<&'static str>>().as_slice()).unwrap();
            kingdom.push(card::for_name(card));
            all.remove(&card);
        }
    }

    kingdom
}

fn build_supply() -> Supply {
    let mut supply: Supply = HashMap::new();
    supply.insert(card::COPPER.to_str(),   30);
    supply.insert(card::SILVER.to_str(),   30);
    supply.insert(card::GOLD.to_str(),     30);
    supply.insert(card::ESTATE.to_str(),   12);
    supply.insert(card::DUCHY.to_str(),    12);
    supply.insert(card::PROVINCE.to_str(), 12);
    supply.insert(card::CURSE.to_str(),    30);
    supply
}

fn play_game(players: Rc<RefCell<PlayerList>>) -> GameResult {
    let empty_limit = get_empty_limit((*players).borrow().len());
    loop {
        let player = (*players).borrow_mut().pop_front().unwrap();
        ACTIVE_PLAYER.replace(Some(player.name()));

        take_turn(&(*player));

        let done = with_active_player(|p| is_game_finished(&(*p.game_ref.borrow()), empty_limit));
        (*players).borrow_mut().push_back(player);

        if done {
            break;
        }
    }

    let mut player_results = (*players).borrow_mut().iter()
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
        }).collect::<Vec<PlayerResult>>();
    player_results.sort_by(|a, ref b| b.vp.cmp(&a.vp));

    let highest_score = player_results.get(0).vp;
    let tie = player_results.iter().skip(1).any(|result| result.vp == highest_score);

    GameResult{
        tie: tie,
        winner: player_results.get(0).name,
        player_results: player_results,
    }
}

fn report(term: &mut Box<Terminal<WriterWrapper> + Send>, games: uint, total_games: uint, scores: &HashMap<String, uint>, ties: uint) {
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
        term.fg(if *value == winning { color::BRIGHT_GREEN } else { color::BRIGHT_RED });
        write!(term, "{}", *value);
        term.reset();
    }
    write!(term, "\tTies: {} \tTotal Played: {}/{}", ties, games, total_games);
    term.flush();
}

fn take_turn(p: &Box<Player + Send + Share>) {
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
    match n {
        0..1 => fail!("Not enough players!"),
        2..4 => 3,
        5..6 => 4,
        _    => fail!("Too many players!"),
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
    f((*STATE_MAP.get().unwrap().borrow_mut()).get_mut(&player))
}

fn with_active_player<T>(f: |&mut PlayerState| -> T) -> T {
    match ACTIVE_PLAYER.get() {
        None => fail!("No active player!"),
        Some(player) => with_player(*player, f),
    }
}

fn with_other_players(f: |&mut PlayerState|) {
    let others = with_active_player(|player| player.other_players.clone());
    let states_ref = STATE_MAP.get().unwrap();
    let mut states = states_ref.borrow_mut();
    for other in others.iter() {
        f(states.get_mut(&other.name()));
    }
}

fn attack(f: |&mut PlayerState|) {
    let others = with_active_player(|player| player.other_players.clone());
    let states_ref = STATE_MAP.get().unwrap();
    let mut states = states_ref.borrow_mut();
    for other in others.iter() {
        let state = states.get_mut(&other.name());
        let attacker = *ACTIVE_CARD.get().unwrap();
        if !state.hand_contains(card::MOAT) || !(**other).moat_should_block(attacker) {
            f(state);
        }
    }
}


/* ------------------------ PlayerState ------------------------ */

// TODO: derive Default?
struct PlayerState {
    game_ref: Rc<RefCell<GameState>>,
    myself: Arc<Box<Player + Send + Share>>,
    other_players: PlayerList,

    deck: Vec<Card>,
    discard: Vec<Card>,
    in_play: Vec<Card>,
    hand: Vec<Card>,

    actions: uint,
    buys: uint,
    buying_power: uint,
}

impl PlayerState {
    // hand_contains() returns true if and only if this player's hand
    // contains a copy of the given card.
    fn hand_contains(&mut self, c: Card) -> bool {
        self.hand.iter().any(|&x| x == c)
    }

    // gain() takes a card from the supply, putting it in the discard pile.
    fn gain(&mut self, c: Card) -> Result {
        let pile = match count(c) {
            None => return Err(NotInSupply(c)),
            Some(0) => return Err(EmptyPile(c)),
            Some(pile) => pile,
        };
        self.with_mut_supply(|supply| supply.insert(c.to_str(), pile - 1));
        self.discard.push(c);
        Ok(())
    }

    // gain_to_deck() takes a card from the supply, putting it on top of
    // the deck.
    fn gain_to_deck(&mut self, c: Card) -> Result {
        let pile = match count(c) {
            None => return Err(NotInSupply(c)),
            Some(0) => return Err(EmptyPile(c)),
            Some(pile) => pile,
        };
        self.with_mut_supply(|supply| supply.insert(c.to_str(), pile - 1));
        self.deck.unshift(c);
        Ok(())
    }

    // gain_to_hand() takes a card from the supply, putting it into
    // the hand.
    fn gain_to_hand(&mut self, c: Card) -> Result {
        let pile = match count(c) {
            None => return Err(NotInSupply(c)),
            Some(0) => return Err(EmptyPile(c)),
            Some(pile) => pile,
        };
        self.with_mut_supply(|supply| supply.insert(c.to_str(), pile - 1));
        self.hand.unshift(c);
        Ok(())
    }

    // curse() gives the player a curse card and depletes one from the supply.
    fn curse(&mut self) -> Result {
        let pile = self.count(card::CURSE).unwrap();
        if pile == 0 {
            Err(EmptyPile(card::CURSE))
        } else {
            self.with_mut_supply(|supply| supply.insert(card::CURSE.to_str(), pile - 1));
            self.discard.push(card::CURSE);
            Ok(())
        }
    }

    // count() returns the number of copies of a card available in the supply,
    // or None if it wasn't included in this game.
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
        for _ in range(0u, 5u) {
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

    // next_n_cards() removes and returns the top n cards from the deck,
    // shuffling the discard pile to make a new deck if necessary.
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

    // mill() takes the top card from the deck and places it in the
    // discard pile.
    #[allow(dead_code)]
    fn mill(&mut self) {
        match self.next_card() {
            Some(c) => self.discard.push(c),
            None => (),
        }
    }

    // draw() takes the top card from the deck and places it in the hand.
    fn draw(&mut self) -> Option<Card> {
        match self.next_card() {
            Some(c) => {
                self.hand.push(c);
                Some(c)
            }
            None => None
        }
    }

    // remove_from_hand() removes the given card from this player's hand,
    // returning true if it was found, or false if it wasn't.
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
    fn discard(&mut self, c: Card) -> Result {
        if !self.remove_from_hand(c) {
            Err(NotInHand(c))
        } else {
            self.discard.push(c);
            Ok(())
        }
    }

    // trash() trashes a card from the player's hand, adding it to the
    // shared trash pile. If it's not in the player's hand than a NotInHand
    // error is returned.
    fn trash(&mut self, c: Card) -> Result {
        if !self.remove_from_hand(c) {
            Err(NotInHand(c))
        } else {
            (*self.game_ref).borrow_mut().trash.push(c);
            Ok(())
        }
    }

    // trash_from_player() is like trash(), but the trashed card must
    // currently be in play.
    fn trash_from_play(&mut self, c: Card) -> Result {
        match self.in_play.iter().enumerate().find(|&(_,&x)| x == c) {
            None => Err(NotInHand(c)),
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

    // with_mut_supply() executes an arbitrary action on the game's supply,
    // mutably.
    fn with_mut_supply<U>(&mut self, f: |&mut Supply| -> U) -> U {
        f(&mut (*self.game_ref).borrow_mut().supply)
    }

    // with_supply() executes an arbitrary action on the game's supply.
    fn with_supply<U>(&mut self, f: |&Supply| -> U) -> U {
        f(&(*self.game_ref).borrow_mut().supply)
    }
}


/* ------------------------ GameState ------------------------ */

#[deriving(Clone)]
struct GameState {
    pub supply: Supply,
    pub trash: Vec<Card>,
}


/* ------------------------ ActionInput ------------------------ */

/// Input parameters for card plays.
pub enum ActionInput {
    /// Discard a card.
    Discard(Card),

    /// Trash a card.
    Trash(Card),

    /// Gain a card.
    Gain(Card),

    /// Confirm an effect, i.e. discarding your deck with Chancellor.
    Confirm,

    /// Repeat an effect, i.e. with Throne Room.
    ///
    /// The first parameter is the card to repeat, and the second is
    /// a function from play iteration (starting with 0 and increasing by one
    /// each time the card is repeated) to the input for that card.
    Repeat(Card, fn(uint) -> Vec<ActionInput>),
}

impl ActionInput {
    #[inline]
    fn is_discard(&self) -> bool {
        match *self {
            Discard(_) => true,
            _ => false,
        }
    }

    #[inline]
    fn is_trash(&self) -> bool {
        match *self {
            Trash(_) => true,
            _ => false,
        }
    }

    #[inline]
    fn is_gain(&self) -> bool {
        match *self {
            Gain(_) => true,
            _ => false,
        }
    }

    #[inline]
    fn is_confirm(&self) -> bool {
        match *self {
            Confirm => true,
            _ => false,
        }
    }

    #[inline]
    fn is_repeat(&self) -> bool {
        match *self {
            Repeat(_, _) => true,
            _ => false,
        }
    }

    #[inline]
    fn get_card(&self) -> Card {
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

    #[inline]
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


/* ------------------------ Error ------------------------ */

/// A custom error type for reporting strategic errors.
pub enum Error {
    NoActions,
    NoBuys,
    InvalidPlay(Card),
    NotInSupply(Card),
    EmptyPile(Card),
    NotInHand(Card),
    NotEnoughMoney { pub need: uint, pub have: uint },
}

impl fmt::Show for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match *self {
            NoActions                        => format!("no actions"),
            NoBuys                           => format!("no buys"),
            InvalidPlay(c)                   => format!("invalid play: {}", c),
            NotInSupply(c)                   => format!("not in supply: {}", c),
            EmptyPile(c)                     => format!("empty pile: {}", c),
            NotInHand(c)                     => format!("not in hand: {}", c),
            NotEnoughMoney{need: x, have: y} => format!("not enough money: need {}, but only have {}", x, y),
        })
    }
}


/* ------------------------ GameResult ------------------------ */

struct GameResult {
    tie: bool,
    winner: &'static str,

    #[allow(dead_code)]
    player_results: Vec<PlayerResult>,
}


/* ------------------------ PlayerResult ------------------------ */

struct PlayerResult {
    name: &'static str,
    vp: int,

    #[allow(dead_code)]
    victory_cards: Vec<Card>,
}


/* ------------------------ Aliases ------------------------ */

/// A static pointer to a card definition.
pub type Card = &'static CardDef;

/// An alias for `std::result::Result<(), Error>`.
pub type Result = std::result::Result<(), Error>;

type ActionFunc = fn(&[ActionInput]);

type PlayerFunc = fn(&mut PlayerState);

type PlayerList = DList<Arc<Box<Player + Send + Share>>>;

type Supply = HashMap<String, uint>;

type VictoryFunc = fn() -> int;
