#![feature(macro_rules, globs, struct_variant, unboxed_closure_sugar, if_let)]
#![allow(dead_code)]

use std::any::Any;
use std::boxed::BoxAny;
use std::collections::{HashMap, RingBuf};
use std::default::Default;
use std::rand::{task_rng, Rng};

use card::Card;
use command::Command;
use notify::Notification;
use query::Query;
use reaction::Reaction;
use response::Response;

mod card;
mod command;
mod notify;
mod query;
mod reaction;
mod response;

#[doc(hidden)]
mod sets;
mod strats;

/// The `Connection` contains the channels that need
/// to be passed to the player for actions to be taken.
pub struct Connection {
    cmd_chan: SyncSender<Command>,
    done_chan: SyncSender<()>,
    notify_port: Receiver<Notification>,
    query_a_port: Receiver<Answer>,
    query_q_chan: SyncSender<Query>,
    react_chan: SyncSender<Reaction>,
    resp_port: Receiver<Response>,
}

impl Connection {
    /// Play a card and return its response. If the card requires doing other
    /// things, e.g. Cellar asks you to discard cards, those should be done via
    /// fluently chaining the call like:
    ///
    /// ~~~ignore
    /// use card::*;
    /// let resp = conn.play(Cellar).discarding(vec![Estate, Duchy]);
    /// ~~~
    pub fn play(&self, card: Card) -> Response {
        self.do_action(command::Play(card))
    }

    pub fn play_all_money(&self) -> Response {
        self.do_action(command::PlayAllMoney)
    }

    pub fn buy(&self, card: Card) -> Response {
        self.do_action(command::Buy(card))
    }

    pub fn recv_notification(&self) -> Notification {
        self.notify_port.recv_opt().unwrap_or(notify::GameOver)
    }

    pub fn not_implemented(&self) {
        self.react_chan.send(reaction::NotImplemented);
    }

    pub fn react(&self, action: Reaction) {
        self.react_chan.send(action);
    }

    pub fn done(&self) {
        self.done_chan.send(());
    }

    fn query<T: 'static>(&self, q: Query) -> Option<T> {
        self.query_q_chan.send(q);
        match self.query_a_port.recv().downcast() {
            Ok(val) => Some(*val),
            Err(_) => None,
        }
    }

    fn do_action(&self, cmd: Command) -> Response {
        self.cmd_chan.send(cmd); self.resp_port.recv()
    }
}

impl Player for Connection {
    fn get_buying_power(&self) -> uint {
        self.query(query::BuyingPower).expect("get_buying_power() query returned an invalid response")
    }

    fn get_hand(&self) -> Vec<Card> {
        self.query(query::Hand).expect("get_hand() query returned an invalid response")
    }

    fn get_hand_size(&self) -> uint {
        self.query(query::HandSize).expect("get_hand_size() query returned an invalid response")
    }

    fn has_in_hand(&self, card: Card) -> bool {
        self.query(query::HasInHand(card)).expect("has_in_hand() query returned an invalid response")
    }
}

enum LoopOption {
    LoopCommand(Command),
    LoopQuery(Query),
    LoopPending((Card, PendingPlay), Sender<Response>),
    LoopDone,
}

#[deriving(Default)]
pub struct Game {
    playing: bool, // could potentially use a status enum here instead
    players: Vec<PlayerHandle>,
    state: GameState,
}

impl Game {
    /// Initialize a new game object.
    pub fn new() -> Game { Default::default() }

    /// Initialize a new game object with enough room for `capacity`
    /// players.
    pub fn with_capacity(capacity: uint) -> Game {
        Game{players: Vec::with_capacity(capacity), ..Default::default()}
    }

    /// Add a player.
    pub fn add_player(&mut self) -> Connection {
        use std::comm::sync_channel;

        // So many channels!
        let (cmd_chan, cmd_port)         = sync_channel(0);
        let (done_chan, done_port)       = sync_channel(0);
        let (notify_chan, notify_port)   = sync_channel(0);
        let (query_q_chan, query_q_port) = sync_channel(0);
        let (query_a_chan, query_a_port) = sync_channel(0);
        let (react_chan, react_port)     = sync_channel(0);
        let (resp_chan, resp_port)       = sync_channel(0);

        self.players.push(PlayerHandle{
            cmd_port: cmd_port,
            done_port: done_port,
            notify_chan: notify_chan,
            play_complete: Vec::new(),
            query_a_chan: query_a_chan,
            query_q_port: query_q_port,
            react_port: react_port,
            resp_chan: resp_chan,

            actions: 0,
            buys: 0,
            buying_power: 0,
            hand: vec![],
            deck: Game::new_deck(),
            discard: vec![],
            in_play: vec![],
        });

        Connection {
            cmd_chan: cmd_chan,
            done_chan: done_chan,
            notify_port: notify_port,
            query_a_port: query_a_port,
            query_q_chan: query_q_chan,
            react_chan: react_chan,
            resp_port: resp_port,
        }
    }

    fn new_deck() -> Vec<Card> {
        use card::*;
        vec![Estate, Estate, Estate, Copper, Copper, Copper, Copper, Copper, Copper, Copper]
    }

    /// Play the game. It loops forever until the game is over.
    pub fn play(mut self) {
        use card::*;

        self.playing = true;
        let num_players = self.players.len();
        let mut handles = RingBuf::new();

        // Populate the kingdom. Need to find a way to customize this.
        for card in vec![Copper, Silver, Gold, Estate, Duchy, Province].into_iter() {
            self.state.kingdom.insert(card, 10);
        }

        for mut p in self.players.into_iter() {
            task_rng().shuffle(p.deck.as_mut_slice());
            p.draw_n(5); // start with 5 cards
            handles.push(p);
        }

        let mut turn = 0u;
        let mut round = 1u;

        'game: loop {
            let mut player = handles.pop_front().expect("no players found!");
            player.actions = 1;
            player.buys = 1;
            player.buying_power = 0;

            // Signal the player that it's their turn.
            player.notify_chan.send(notify::YourTurn(round));

            'player: loop {
                match player.wait() {
                    LoopCommand(cmd) => {
                        let resp = player.handle_cmd(cmd, &mut self.state, &mut handles, None);
                        player.resp_chan.send(resp);
                    },
                    LoopQuery(query) => {
                        let a = player.answer_query(query, &mut self.state);
                        player.query_a_chan.send(a);
                    },
                    LoopPending((card, pending), resp_chan) => {
                        let resp = player.handle_cmd(command::Play(card), &mut self.state, &mut handles, Some(pending));
                        resp_chan.send(resp);
                    },
                    LoopDone => break 'player,
                }
            }

            // Refresh the hand.
            player.discard_hand();
            player.draw_n(5);

            // Add the player to the end of the list.
            handles.push(player);

            // Keep track of the turn. Once the turn number hits the number of
            // players, we've gone full circle and begun a new round.
            turn += 1;
            if turn == num_players {
                turn = 0u;
                round += 1;
            }

            // Play for ten rounds.
            if round > 10 {
                break 'game;
            }
        }

        // Tell everyone to quit.
        for player in handles.iter() {
            player.notify_chan.send(notify::GameOver);
        }

        // Game is done.
    }
}

/// Player handle. It contains a definition of the player trait,
/// as well as several "pipes" that act as two-way communication
/// channels.
struct PlayerHandle {
    cmd_port: Receiver<Command>,
    done_port: Receiver<()>,
    notify_chan: SyncSender<Notification>,
    play_complete: Vec<(Sender<Response>, Receiver<(Card, PendingPlay)>)>,
    query_a_chan: SyncSender<Answer>,
    query_q_port: Receiver<Query>,
    react_port: Receiver<Reaction>,
    resp_chan: SyncSender<Response>,

    actions: uint,
    buys: uint,
    buying_power: uint,
    hand: Vec<Card>,
    deck: Vec<Card>,
    discard: Vec<Card>,
    in_play: Vec<Card>,
}

trait Player {
    fn get_buying_power(&self) -> uint;
    fn get_hand(&self) -> Vec<Card>;
    fn get_hand_size(&self) -> uint;
    fn has_in_hand(&self, card: Card) -> bool;

    fn has_or_else(&self, card: Card, f: ||) {
        if !self.has_in_hand(card) {
            f();
        }
    }
}

impl PlayerHandle {
    fn wait(&mut self) -> LoopOption {
        let sel = std::comm::Select::new();

        let mut cmd = sel.handle(&self.cmd_port);
        let mut query = sel.handle(&self.query_q_port);
        let mut done = sel.handle(&self.done_port);
        let mut all_pending: Vec<(&Sender<Response>, std::comm::Handle<(Card, PendingPlay)>)> = Vec::new();
        let mut pending_iter = self.play_complete.iter_mut();

        unsafe {
            for &(ref resp_chan, ref pending_port) in pending_iter {
                let mut pending = sel.handle(pending_port);
                pending.add();
                all_pending.push((resp_chan, pending));
            }
            cmd.add(); query.add(); done.add();
        }

        let id = sel.wait();

        if id == cmd.id() {
            LoopCommand(cmd.recv())
        } else if id == query.id() {
            LoopQuery(query.recv())
        } else if id == done.id() {
            LoopDone
        } else {
            for &(ref resp_chan, ref mut pending) in all_pending.iter_mut() {
                if id == pending.id() {
                    return LoopPending(pending.recv(), (*resp_chan).clone());
                }
            }
            unreachable!()
        }
    }

    /// Handle a command from the player.
    fn handle_cmd(&mut self, cmd: Command, state: &mut GameState, opponents: &mut RingBuf<PlayerHandle>, pending: Option<PendingPlay>) -> Response {
        use command::*;
        macro_rules! try(($e:expr) => ({
            let resp = $e;
            if resp.is_err() { return resp } else { resp }
        }))
        match cmd {
            Buy(card) => {
                use std::collections::hash_map::{Vacant, Occupied};
                match state.kingdom.entry(card) {
                    Vacant(_) => return response::NotInKingdom(card),
                    Occupied(ref entry) if *entry.get() == 0 => response::PileEmpty(card),
                    Occupied(entry) => {
                        *entry.into_mut() -= 1;
                        self.discard.push(card);
                        response::NoProblem
                    },
                }
            },
            Play(card) => {
                let resp = try!(card.play(self, state, opponents.iter_mut(), pending));
                self.put_in_play(card);
                resp
            },
            PlayAllMoney => {
                let money: Vec<Card> = self.hand.iter().filter_map(|x| if x.is_money() && !x.is_action() { Some(*x) } else { None }).collect();
                for card in money.iter() {
                    try!(card.play(self, state, opponents.iter_mut(), None));
                    self.put_in_play(*card);
                }
                response::NoProblem
            },
        }
    }

    fn answer_query(&self, q: Query, _: &mut GameState) -> Answer {
        use query::*;
        macro_rules! answer (($e:expr) => (box $e as Answer))
        match q {
            BuyingPower => answer!(self.get_buying_power()),
            Hand => answer!(self.get_hand()),
            HandSize => answer!(self.get_hand_size()),
            HasInHand(card) => answer!(self.has_in_hand(card)),
        }
    }

    /// Draw a card from the top of the player's deck and put it into their hand. If the deck
    /// is empty, then the discard needs to be shuffled and turned into the new deck.
    fn draw(&mut self) -> Option<Card> {
        if self.deck.is_empty() && !self.discard.is_empty() {
            self.deck.push_all(self.discard.as_slice());
            task_rng().shuffle(self.deck.as_mut_slice());
            self.discard.clear();
        }
        let drew = self.deck.remove(0);
        if let Some(card) = drew {
            self.hand.push(card);
        }
        drew
    }

    /// Draw multiple cards.
    fn draw_n(&mut self, n: uint) {
        for _ in range(0, n) {
            self.draw();
        }
    }

    /// Discard your hand.
    fn discard_hand(&mut self) {
        self.discard.push_all(self.hand.as_slice());
        self.hand.clear();
    }

    /// Discard a card from the player's hand. It fails if that card isn't
    /// in the player's hand.
    fn discard(&mut self, card: Card) {
        match self.remove_from_hand(card) {
            true => self.discard.push(card),
            false => panic!("player tried to discard {}, but doesn't have it!", card),
        }
    }

    /// Like discard(), but the card goes to the playing area instead of the
    /// discard pile.
    fn put_in_play(&mut self, card: Card) {
        match self.remove_from_hand(card) {
            true => self.in_play.push(card),
            false => panic!("player tried to put {} in play, but doesn't have it!", card),
        }
    }

    /// Trash a card from the player's hand. It fails if that card isn't
    /// in the player's hand.
    fn trash(&mut self, state: &mut GameState, card: Card) {
        match self.remove_from_hand(card) {
            true => state.trash.push(card),
            false => panic!("player tried to trash {}, but doesn't have it!", card),
        }
    }

    /// Utility method used for actions like discarding and trashing. Returns true
    /// if the card was successfully removed from the hand, otherwise false.
    fn remove_from_hand(&mut self, card: Card) -> bool {
        match self.hand.iter().position(|x| *x == card) {
            Some(i) => self.hand.remove(i).is_some(),
            None => false,
        }
    }
}

impl Player for PlayerHandle {
    fn get_buying_power(&self) -> uint {
        self.buying_power
    }

    /// Returns a clone of the player's hand.
    fn get_hand(&self) -> Vec<Card> {
        self.hand.clone()
    }

    /// Returns the number of cards in the player's hand.
    fn get_hand_size(&self) -> uint {
        self.hand.len()
    }

    /// Returns true only if the provided card is currently held in the
    /// player's hand.
    fn has_in_hand(&self, card: Card) -> bool {
        self.hand.iter().any(|x| *x == card)
    }
}

struct PendingPlay {
    index: uint,
    discarding: Vec<Card>,
    trashing: Vec<Card>,
}

impl PendingPlay {
    fn new(index: uint) -> PendingPlay {
        PendingPlay{
            index: index,
            discarding: Vec::new(),
            trashing: Vec::new(),
        }
    }
}

/// Game state which keeps track of things like how many cards are
/// in each pile, what's in the trash, etc.
#[deriving(Default)]
struct GameState {
    kingdom: HashMap<Card, uint>,
    trash: Vec<Card>,
}

type Answer = Box<Any + Send>;
