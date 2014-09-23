//! This module provides an API for writing Dominion AI's in Rust. AI's are created by
//! defining a new empty struct and implementing the `Player` trait, like so:
//!
//! ~~~
//! #![feature(phase)]
//!
//! #[phase(plugin, link)]
//! extern crate dominion;
//!
//! // Define our awesome custom strategy.
//! struct Me;
//! impl dominion::Player for Me {
//!     fn name(&self) -> &'static str { "Me" }
//!     fn init(&self, _: &[dominion::Card]) -> fn() { my_turn }
//! }
//!
//! fn my_turn() {
//!     dominion::strat::big_money();
//! }
//!
//! // Define our opponent's lousy strategy using the `player!` macro,
//! // which is shorthand for the above code.
//! player!(Them using dominion::strat::big_money)
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
//! the `Player` trait. Otherwise, everything takes place in the function returned
//! by `init()`, which will be executed every time it's your AI's turn to play.
//!
//! Actions like playing and buying cards are achieved by using the public methods
//! exposed by this module. The game keeps track of who the active player is, so
//! there's no need to provide any player information in these methods.
//!
//! This library by default plays 1,000 games, unless a different number is
//! specified as an argument. For example, compiling the above example into an
//! executable called `main` and running `./main` will play 1,000 games, but running
//! `./main 100` will only play 100.

#![crate_name = "dominion"]
#![crate_type = "lib"]

#![feature(globs)]
#![feature(struct_variant)]
#![feature(macro_rules)]
#![allow(unused_must_use)]

extern crate debug;
extern crate getopts;
extern crate sync;
extern crate term;

use std::fmt;
use std::cell::RefCell;
use std::collections::{Deque, DList, HashMap};
use std::default::Default;
use std::owned::Box;
use std::rc::Rc;
use std::rand::{task_rng, Rng};
use std::string::String;
use std::vec::Vec;
use sync::Arc;
use term::{Terminal, WriterWrapper, stdout};

pub mod cards;
pub mod strat;

/// Play Dominion.
///
/// This macro takes the identifiers of the player types you intend to use,
/// which should be empty structs implementing `Player`.
#[macro_export]
macro_rules! dominion(
    ($($player:ident),+) => (
        dominion::play(vec![$(
            box $player as Box<dominion::Player + Send + Sync>,
        )+]);
    )
)

/// Set a Kingdom to play with.
///
/// By default, games will choose a random set of cards to use as the
/// Kingdom. Use this macro before calling `dominion!()` to include specific
/// cards in the game. If fewer than 10 cards are supplied, then the rest
/// will be randomly chosen from what's left. Anything over 10 will be
/// ignored.
#[macro_export]
macro_rules! kingdom(
    ($($card:expr),+) => (
        dominion::set_kingdom(vec![$( $card, )+]);
    )
)

/// Shortcut for defining a new Player.
///
/// The Player trait's init() function is useful if you want to
/// define a flexible AI that can adapt to different Kingdoms.
/// However, if you only ever want to execute one strategy, then
/// this macro provides a convenient way to define a new player.
/// For example, this...
///
/// ~~~ignore
/// struct Me;
/// impl dominion::Player for Me {
///     fn name(&self) -> &'static str { "Me" }
///     fn init(&self, _: &[dominion::Card]) -> fn() { my_strategy }
/// }
///
/// fn my_strategy() {
///     // Do our awesome custom strategy.
/// }
/// ~~~
///
/// ...can be shortened to this:
///
/// ~~~ignore
/// player!(Me using my_strategy)
///
/// fn my_stategy() {
///     // Do our awesome custom strategy.
/// }
/// ~~~
#[macro_export]
macro_rules! player(
    ($player:ident using $f:expr) => {
        struct $player;
        impl dominion::Player for $player {
            fn name(&self) -> &'static str { stringify!($player) }
            fn init(&self, _: &[dominion::Card]) -> fn() { $f }
        }
    }
)

// Game setup keys.
local_data_key!(local_kingdom: Vec<Card>)

// Game-specific keys.
local_data_key!(local_state_map: RefCell<HashMap<&'static str, PlayerState>>)
local_data_key!(local_fn_map: HashMap<&'static str, fn()>)
local_data_key!(local_active_player: &'static str)
local_data_key!(local_active_card: Card)


/// Use 10 different cards per Kingdom.
static KINGDOM_SIZE: uint = 10;

/// Each pile of cards should have 10 copies available.
static PILE_SIZE: uint = 10;


/* ------------------------ Player Trait ------------------------ */


#[allow(unused_variable)]
/// A player definition.
///
/// The only required methods are `name()` and `init()`,
/// but other methods may be overridden in order to gain more control over
/// your player.
pub trait Player {
    /// Gets the name of this player, which must be unique.
    fn name(&self) -> &'static str;

    /// Called before the first turn is played. `kingdom` is a slice of
    /// the 10 cards that will be used this game, and it should return
    /// a pointer to the function that will be called once per turn.
    fn init(&self, kingdom: &[Card]) -> fn();

    /// Called when an Action card is encountered as part of a Library draw.
    /// It should return true if that card should be discarded, false if it
    /// should be kept.
    ///
    /// By default, always discards Action cards.
    fn library_should_discard(&self, drawn: Card) -> bool {
        true
    }

    /// Called when another player plays Militia, and is called repeatedly until
    /// you have three or fewer cards in hand. The value returned should be
    /// one of the cards in your hand. Returning a card not in your hand results
    /// in a task failure.
    ///
    /// By default, discards the first card.
    fn militia_discard(&self, hand: &[Card]) -> Card {
        hand[0]
    }

    /// Called when another player plays an attack card while you have a Moat in
    /// hand. It should return true if you wish to block the attack, otherwise false.
    ///
    /// By default always blocks attacks.
    fn moat_should_block(&self, attacker: Card) -> bool {
        true
    }

    /// Called when a Spy is played, including by you.
    /// Given the value of the top card of a player's deck, this method should
    /// return true if that card should be discarded, and false if it should
    /// be returned to the top of the player's deck. The value of `is_self` is
    /// true if and only if you are the player being acted on.
    ///
    /// By default, keeps victory and curse cards on top for other players and
    /// discards anything else, and does the exact opposite for yourself.
    fn spy_should_discard(&self, top_card: Card, is_self: bool) -> bool {
        let is_worthless = top_card.is_victory() || top_card.is_curse();
        if is_self { is_worthless } else { !is_worthless }
    }

    /// Called when another player plays Bureaucrat, and you have one or more
    /// Victory cards in your hand. `victory_cards` is a slice of all the
    /// Victory cards in your hand (and will always contain at least one),
    /// and the value returned will be placed back on top of your deck.
    /// Returning a value not in the slice results in a task failure.
    ///
    /// By default, returns the first card.
    fn bureaucrat_use_victory(&self, victory_cards: &[Card]) -> Card {
        victory_cards[0]
    }

    /// Called when you play Thief and someone reveals one or more treasure cards.
    /// `options` contains at least one card (but no more than 2), and it should
    /// return a tuple describing how to treat the reveal. The first value is the
    /// card that should be trashed, and the second value is a boolean indicating
    /// whether or not it should be kept.
    ///
    /// By default, always trashes the highest value treasure card and keeps it.
    fn thief_trash_and_keep(&self, options: &[Card]) -> (Card, bool) {
        let mut money = Vec::from_slice(options);
        money.sort_by(|m1, m2| m2.treasure_value().cmp(&m1.treasure_value()));
        (money[0], true)
    }
}


/* ------------------------ Public Methods ------------------------ */


/// Buy a card from the supply, returning one of three possible
/// errors:
///
///   1. NotInSupply, if the card is not available in this game, or
///   2. EmptyPile, if there are no more available to buy, or
///   3. NotEnoughMoney(need, have), if the player doesn't have enough money.
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
            player.with_mut_supply(|supply| supply.insert(c.name, pile - 1));
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
    with_active_player(|x| x.count(c))
}

/// Get the number of actions left for the current player.
pub fn get_action_count() -> uint {
    with_active_player(|x| x.actions)
}

/// Get a count of the total available money currently in the player's hand.
pub fn get_available_money() -> uint {
    with_active_player(|x| x.hand.iter()
        .filter(|&c| c.is_money())
        .fold(0, |a, &b| a + b.treasure_value())
    )
}

/// Get the current available buying power.
///
/// You must have played at least one Money card or Action card providing
/// buying power for this value to be higher than 0.
pub fn get_buying_power() -> uint {
    with_active_player(|x| x.buying_power)
}

/// Get a copy of the player's discard pile.
pub fn get_discard() -> Vec<Card> {
    with_active_player(|x| x.discard.clone())
}

/// Get a copy of the player's hand.
pub fn get_hand() -> Vec<Card> {
    with_active_player(|x| x.hand.clone())
}

/// Get the number of cards in the player's hand.
pub fn get_hand_size() -> uint {
    with_active_player(|x| x.hand.len())
}

/// Get the total point value from all victory
/// and curse cards in the player's deck, hand, and discard.
pub fn get_total_points() -> int {
    with_active_player(|x|
        x.deck.iter().chain(x.discard.iter()).chain(x.hand.iter())
            .filter(|&c| c.is_victory() || c.is_curse())
            .fold(0, |a, &b| a + b.victory_points())
    )
}

/// Get a clone of the game's trash pile.
pub fn get_trash() -> Vec<Card> {
    with_active_player(|x| (*x.game_ref).borrow().trash.clone())
}

/// Returns true if and only if the player's hand contains
/// the specified card.
pub fn hand_contains(c: Card) -> bool {
    with_active_player(|x| x.hand_contains(c))
}

/// Returns true if and only if the player has the provided card in their
/// hand, deck, discard, or in play.
pub fn has(c: Card) -> bool {
    with_active_player(|x|
        x.hand.iter().chain(x.deck.iter()).chain(x.discard.iter()).chain(x.in_play.iter())
            .any(|&y| y == c)
    )
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
pub fn play(player_list: Vec<Box<Player + Send + Sync>>) {
    use std::comm;

    let mut term = stdout().unwrap();

    let args = std::os::args().iter().map(|x| x.to_string()).collect::<Vec<String>>();
    let opts = [
        getopts::optopt("o", "output", "set debug output file name", "NAME"),
    ];
    let matches = match getopts::getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(f) => fail!(format!("{}", f)),
    };
    let output_name = matches.opt_str("o");

    let n: uint = if !matches.free.is_empty() {
        from_str(matches.free[0].as_slice()).unwrap()
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
        supply.insert(card.name, PILE_SIZE);
    }
    writeln!(term, ".");

    let mut player_fn_map = HashMap::<&'static str, fn()>::new();

    for player in player_list.into_iter() {
        player_fn_map.insert(player.name(), player.init(kingdom.as_slice()));
        scores.insert(player.name().to_string(), 0);
        player_arcs.push(Arc::new(player));
    }

    spawn(proc() {
        for _ in range(0u, n) {
            let reporter = reporter.clone();
            let trash = trash.clone();
            let supply = supply.clone();
            let player_arcs = player_arcs.clone();
            let player_fn_map = player_fn_map.clone();

            let future_result = std::task::try_future(proc() {
                let mut rng = task_rng();

                let mut player_arcs = player_arcs;
                rng.shuffle(player_arcs.as_mut_slice());

                let mut deck = Vec::new();
                deck.push_all_move(cards::COPPER.create_copies(7));
                deck.push_all_move(cards::ESTATE.create_copies(3));
                rng.shuffle(deck.as_mut_slice());

                let players = Rc::new(RefCell::new(DList::<Arc<Box<Player + Send + Sync>>>::new()));
                let game = Rc::new(RefCell::new(GameState{ supply: supply, trash: trash }));
                let mut player_state_map = HashMap::<&'static str, PlayerState>::new();
                let other_players = player_arcs.clone().into_iter().collect::<PlayerList>();

                {
                    let mut players = players.borrow_mut();

                    for p in player_arcs.into_iter() {
                        let mut other_players = other_players.clone();
                        while other_players.front().unwrap().name() != p.name() {
                            other_players.rotate_backward();
                        }
                        other_players.pop_front();
                        player_state_map.insert(p.name(), PlayerState{
                            game_ref: game.clone(),
                            myself: p.clone(),
                            other_players: other_players,
                            deck: deck.clone(),
                            ..Default::default()
                        });
                        players.push(p);
                    }
                }

                local_state_map.replace(Some(RefCell::new(player_state_map)));
                local_fn_map.replace(Some(player_fn_map));

                play_game(players)
            });

            reporter.send(future_result);
        }
    });

    let mut ties = 0;
    let mut failures = 0;
    report(&mut term, 0, n, &scores, ties, failures);
    let mut output_file = output_name.clone().map(|x| std::io::File::create(&Path::new(x)).unwrap());

    for i in range(0, n) {
        match receiver.recv().unwrap() {
            Err(e) => {
                failures += 1;
                log(output_file.as_mut(), format!("[failure] {:?}", e));
            },
            Ok(results) => {
                if results.tie {
                    ties += 1;
                    log(output_file.as_mut(), format!("[tie]"));
                } else {
                    scores.insert_or_update_with(String::from_str(results.winner), 1, |_, v| *v += 1);
                    log(output_file.as_mut(), format!("[winner] {}", results.winner));
                }
            },
        }
        report(&mut term, i+1, n, &scores, ties, failures);
    }

    output_file.map(|mut f| f.fsync());
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
        play_card(*card, []).unwrap();
    }
}

fn using_active_card(c: Card, f: ||) {
    local_active_card.replace(Some(c));
    f();
    local_active_card.replace(None);
}

/// Play a card.
///
/// This method returns an InvalidPlay error if either
///
///   1. The requested card is not in the player's hand, or
///   2. The card cannot be played, e.g. Province.
///
/// Other errors may occur if there are not enough actions or buys, and once a non-Action
/// Money card is played, then the player's action count is set to 0.
pub fn play_card(c: Card, params: &[ActionParameter]) -> Result {
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
                    if !c.is_action() {
                        player.actions = 0;
                    }
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
    if result.is_ok() {
        match action {
            Some(f) => using_active_card(c, || match f(params) {
                Ok(_) => (),
                Err(e) => fail!("{}", e),
            }),
            None => (),
        }
    }
    result
}

/// Sets the kingdom to be used.
pub fn set_kingdom(cards: Vec<Card>) {
    local_kingdom.replace(Some(cards));
}


/* ------------------------ Private Methods ------------------------ */

fn build_kingdom() -> Vec<Card> {
    let mut kingdom = Vec::with_capacity(KINGDOM_SIZE);
    match local_kingdom.get() {
        None => (),
        Some(cards) => for card in cards.iter().take(KINGDOM_SIZE) {
            kingdom.push(*card);
        },
    }

    if kingdom.len() < KINGDOM_SIZE {
        let mut rng = task_rng();
        let mut all = cards::dominion::set();
        for c in kingdom.iter() {
            all.remove(&c.name);
        }
        while kingdom.len() < KINGDOM_SIZE {
            let card = *rng.choose(all.iter().map(|x| *x).collect::<Vec<&'static str>>().as_slice()).unwrap();
            kingdom.push(cards::for_name(card));
            all.remove(&card);
        }
    }

    kingdom
}

fn build_supply() -> Supply {
    let mut supply: Supply = HashMap::new();
    supply.insert(cards::COPPER.name,   30);
    supply.insert(cards::SILVER.name,   30);
    supply.insert(cards::GOLD.name,     30);
    supply.insert(cards::ESTATE.name,   12);
    supply.insert(cards::DUCHY.name,    12);
    supply.insert(cards::PROVINCE.name, 12);
    supply.insert(cards::CURSE.name,    30);
    supply
}

#[inline]
fn log(file: Option<&mut std::io::File>, msg: String) {
    match file.map(|f| f.write_line(msg.as_slice())) {
        Some(Err(e)) => fail!(e),
        _ => (),
    }
}

fn play_game(players: Rc<RefCell<PlayerList>>) -> GameResult {
    let empty_limit = get_empty_limit((*players).borrow().len());
    loop {
        let player = (*players).borrow_mut().pop_front().unwrap();
        local_active_player.replace(Some(player.name()));

        take_turn(&(*player));

        let done = with_active_player(|p| is_game_finished(&(*p.game_ref.borrow()), empty_limit));
        (*players).borrow_mut().push(player);

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

    let highest_score = player_results[0].vp;
    let tie = player_results.iter().skip(1).any(|result| result.vp == highest_score);

    GameResult{
        tie: tie,
        winner: player_results[0].name,
        player_results: player_results,
    }
}

fn report(term: &mut Box<Terminal<WriterWrapper> + Send>, games: uint, total_games: uint, scores: &HashMap<String, uint>, ties: uint, failures: uint) {
    use term::color;

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
    write!(term, "\tTies: {} \tFailures: {}\tTotal Played: {}/{}", ties, failures, games, total_games);
    term.flush();
}

fn take_turn(p: &Box<Player + Send + Sync>) {
    with_active_player(|player| {
        player.new_hand();
        player.actions = 1;
        player.buys = 1;
        player.buying_power = 0;
    });
    let map = local_fn_map.get().unwrap();
    (*map)[p.name()]();
    with_active_player(|x| x.discard_hand());
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
    if *game.supply.find(&cards::PROVINCE.name).unwrap() == 0 {
        true
    } else {
        let num_empty = game.supply.iter().filter(|&(_, &x)| x == 0).fold(0, |a, (_, &b)| a + b);
        num_empty >= empty_limit
    }
}

fn with_player<T>(player: &'static str, f: |&mut PlayerState| -> T) -> T {
    f((*local_state_map.get().unwrap().borrow_mut()).get_mut(&player))
}

fn with_active_player<T>(f: |&mut PlayerState| -> T) -> T {
    match local_active_player.get() {
        None => fail!("No active player!"),
        Some(player) => with_player(*player, f),
    }
}

/// Calls f() on each player in turn. If any of the calls returns an `Err`,
/// then the calls are stopped immediately and that value is returned.
/// If all calls to f() succeed, then this method returns `Ok(())`.
fn with_other_players(f: |&mut PlayerState| -> Result) -> Result {
    let others = with_active_player(|x| x.other_players.clone());
    let states_ref = local_state_map.get().unwrap();
    let mut states = states_ref.borrow_mut();
    for other in others.iter() {
        let result = f(states.get_mut(&other.name()));
        if result.is_err() {
            return result;
        }
    }
    Ok(())
}

/// Calls f() on each player in turn. If any of the calls returns an `Err`,
/// then the calls are stopped immediately and that value is returned.
/// If all calls to f() succeed, then this method returns `Ok(())`.
fn attack(f: |&mut PlayerState| -> Result) -> Result {
    let others = with_active_player(|x| x.other_players.clone());
    let states_ref = local_state_map.get().unwrap();
    let mut states = states_ref.borrow_mut();
    for other in others.iter() {
        let state = states.get_mut(&other.name());
        let attacker = *local_active_card.get().unwrap();
        if !state.hand_contains(cards::dominion::MOAT) || !(**other).moat_should_block(attacker) {
            let result = f(state);
            if result.is_err() {
                return result;
            }
        }
    }
    Ok(())
}


/* ------------------------ PlayerState ------------------------ */

// TODO: derive Default?
struct PlayerState {
    game_ref: Rc<RefCell<GameState>>,
    myself: Arc<Box<Player + Send + Sync>>,
    other_players: PlayerList,

    deck: Vec<Card>,
    discard: Vec<Card>,
    in_play: Vec<Card>,
    hand: Vec<Card>,

    actions: uint,
    buys: uint,
    buying_power: uint,
}

impl Default for PlayerState {
    fn default() -> PlayerState {
        PlayerState{
            game_ref: Rc::new(RefCell::new(Default::default())),
            myself: Arc::new(box dummy as Box<Player + Send + Sync>),
            other_players: DList::new(),
            deck: Vec::new(),
            discard: Vec::new(),
            in_play: Vec::new(),
            hand: Vec::new(),
            actions: 0,
            buys: 0,
            buying_power: 0,
        }
    }
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
        self.with_mut_supply(|supply| supply.insert(c.name, pile - 1));
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
        self.with_mut_supply(|supply| supply.insert(c.name, pile - 1));
        self.deck.insert(0, c);
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
        self.with_mut_supply(|supply| supply.insert(c.name, pile - 1));
        self.hand.insert(0, c);
        Ok(())
    }

    // curse() gives the player a curse card and depletes one from the supply.
    fn curse(&mut self) -> Result {
        let pile = self.count(cards::CURSE).unwrap();
        if pile == 0 {
            Err(EmptyPile(cards::CURSE))
        } else {
            self.with_mut_supply(|supply| supply.insert(cards::CURSE.name, pile - 1));
            self.discard.push(cards::CURSE);
            Ok(())
        }
    }

    // count() returns the number of copies of a card available in the supply,
    // or None if it wasn't included in this game.
    fn count(&mut self, c: Card) -> Option<uint> {
        self.with_supply(|supply| {
            match supply.find(&c.name) {
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
        self.discard.push_all(self.in_play.as_slice());
        self.discard.push_all(self.hand.as_slice());
        self.in_play.clear();
        self.hand.clear();
    }

    // discard_deck() puts all of the cards from the deck into the discard pile.
    fn discard_deck(&mut self) {
        self.discard.push_all(self.deck.as_slice());
        self.deck.clear();
    }

    // next_card() removes and returns the top card from the deck, shuffling
    // the discard pile to make a new deck if necessary.
    fn next_card(&mut self) -> Option<Card> {
        use std::mem;

        if self.deck.is_empty() {
            mem::swap(&mut self.deck, &mut self.discard);
            task_rng().shuffle(self.deck.as_mut_slice());
        }
        self.deck.remove(0)
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
            },
            None => None,
        }
    }

    // remove_from_hand() removes the given card from this player's hand,
    // returning true if it was found, or false if it wasn't.
    fn remove_from_hand(&mut self, c: Card) -> bool {
        match self.hand.iter().enumerate().find(|&(_,&x)| x == c) {
            Some((i,_)) => {
                self.hand.remove(i);
                true
            },
            None => false,
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
            Some((i,_)) => {
                let card = self.in_play.remove(i).unwrap();
                (*self.game_ref).borrow_mut().trash.push(card);
                Ok(())
            },
            None => Err(NotInHand(c)),
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

impl Default for GameState {
    fn default() -> GameState {
        GameState{supply: HashMap::new(), trash: Vec::new()}
    }
}


/* ------------------------ ActionParameter ------------------------ */

/// Input parameters for card plays.
pub enum ActionParameter {
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
    Repeat(Card, RepeatFunc),
}

impl ActionParameter {
    fn get_confirmed(inputs: &[ActionParameter]) -> bool {
        inputs.iter().any(|x| match *x {
            Confirm => true,
            _ => false,
        })
    }

    fn get_discarded<'a>(inputs: &'a [ActionParameter]) -> InputIterator<'a>{
        inputs.iter().filter_map(|x| match *x {
            Discard(card) => Some(card),
            _ => None,
        })
    }

    fn get_gained<'a>(inputs: &'a [ActionParameter]) -> InputIterator<'a> {
        inputs.iter().filter_map(|x| match *x {
            Gain(card) => Some(card),
            _ => None,
        })
    }

    fn get_repeated<'a>(inputs: &'a [ActionParameter]) -> std::iter::FilterMap<'a, &'a ActionParameter, (Card, RepeatFunc), std::slice::Items<'a, ActionParameter>> {
        inputs.iter().filter_map(|x| match *x {
            Repeat(card, f) => Some((card, f)),
            _ => None,
        })
    }

    fn get_trashed<'a>(inputs: &'a [ActionParameter]) -> InputIterator<'a> {
        inputs.iter().filter_map(|x| match *x {
            Trash(card) => Some(card),
            _ => None,
        })
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
        self.to_string().eq(&other.to_string())
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
    EmptyPile(Card),
    InvalidChoice(Card), // For cards that choose from a set of available options.
    InvalidPlay(Card),
    NoActions,
    NoBuys,
    NotEnoughMoney { pub need: uint, pub have: uint },
    NothingToGain,
    NothingToRepeat,
    NothingToTrash,
    NotInHand(Card),
    NotInSupply(Card),
}

impl fmt::Show for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match *self {
            EmptyPile(c)                     => format!("empty pile: {}", c),
            InvalidChoice(c)                 => format!("invalid choice: {}", c),
            InvalidPlay(c)                   => format!("invalid play: {}", c),
            NoActions                        => format!("no actions"),
            NoBuys                           => format!("no buys"),
            NotEnoughMoney{need: x, have: y} => format!("not enough money: need {}, but only have {}", x, y),
            NothingToGain                    => format!("nothing to gain"),
            NothingToRepeat                  => format!("nothing to repeat"),
            NothingToTrash                   => format!("nothing to trash"),
            NotInHand(c)                     => format!("not in hand: {}", c),
            NotInSupply(c)                   => format!("not in supply: {}", c),
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


#[allow(non_camel_case_types)]
struct dummy;
impl Player for dummy {
    fn name(&self) -> &'static str { "Dummy" }
    fn init(&self, _: &[Card]) -> fn() { dummy_turn }
}

fn dummy_turn() {}

/* ------------------------ Aliases ------------------------ */

/// A static pointer to a card definition.
pub type Card = &'static CardDef;

/// An alias for `std::result::Result<(), dominion::Error>`.
pub type Result = std::result::Result<(), Error>;

pub type RepeatFunc = fn(uint) -> Vec<ActionParameter>;

type ActionFunc = fn(&[ActionParameter]) -> Result;

type InputIterator<'a> = std::iter::FilterMap<'a, &'a ActionParameter, Card, std::slice::Items<'a, ActionParameter>>;

type PlayerList = DList<Arc<Box<Player + Send + Sync>>>;

type Supply = HashMap<&'static str, uint>;

type VictoryFunc = fn() -> int;
