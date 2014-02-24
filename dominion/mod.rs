
use std::any;
use std::cell::RefCell;
use std::hashmap::HashMap;
use std::iter;
use std::ptr;
use std::rc::{Rc,Weak};
use std::mem;
use std::vec;

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
pub fn play(players: ~[Player]) -> Option<~str> {
    let num_players = players.len();
    if num_players <= 1 {
        fail!("Not enough players!");
    } else if num_players > 6 {
        fail!("Too many players!");
    }

    let empty_limit = match num_players {
        0..4 => 3,
        _    => 4,
    };

    let mut supply = HashMap::new();
    supply.insert(card::COPPER,   30);
    supply.insert(card::SILVER,   30);
    supply.insert(card::GOLD,     30);
    supply.insert(card::ESTATE,   12);
    supply.insert(card::DUCHY,    12);
    supply.insert(card::PROVINCE, 12);
    supply.insert(card::CURSE,    30);

    // now for the variations!
    supply.insert(card::SMITHY, 10);
    supply.insert(card::WITCH, 10);

	let supply_cell = RefCell::new(supply);
	let supply_rc = Rc::new(supply_cell);

	let player_refs = players.move_iter().map(|p| Rc::new(RefCell::new(p))).to_owned_vec();

    for me_ref in player_refs.iter() {
		let refs = player_refs.clone();
		let refs_iter = refs.iter();
        let weak_refs = me_ref.borrow().with(|me| {
			let pre  = refs_iter.take_while(|p_ref| p_ref.borrow().with(|p| p.ne(me)));
			let post = refs_iter.skip_while(|p_ref| p_ref.borrow().with(|p| p.ne(me))).skip(1);
			let others = post.chain(pre);
			others.map(|r| r.downgrade()).to_owned_vec()
		});
        me_ref.borrow().with_mut(|me| {
			me.player_refs = weak_refs.clone(); // why is this necessary to clone?
			me.supply_rc = supply_rc.clone();
		});
    }

    'game: loop {
        for player_ref in player_refs.iter() {
			let done = player_ref.borrow().with_mut(|player| {
				player.new_hand();
				player.actions = 1;
				player.buys = 1;
				player.buying_power = 0;
				(player.play)(player);
				player.discard();

				player.with_supply(|supply| {
					if *supply.get(&card::PROVINCE) == 0 {
						true
					} else {
						let num_empty = supply.values().filter(|x| **x == 0).fold(0, |a, &b| a + b);
						num_empty >= empty_limit
					}
				})
			});
			if done {
				break 'game;
			}
        }
    }

    // Calculate the results
    let mut highest_score = 0;
	for player_ref in player_refs.iter() {
		player_ref.borrow().with_mut(|player| {
			player.calculate_score();
			if player.score > highest_score {
				highest_score = player.score;
			}
		});
	}

    let winners = player_refs.iter()
		.filter(|player_ref| player_ref.borrow().with(|player| player.score == highest_score))
		.to_owned_vec();

    if winners.len() == 1 {
		Some(winners[0].borrow().with(|player| player.name.clone()))
    } else {
        // tie
        None
    }
}


pub type PlayerFunc = fn(&mut Player);

pub type PlayerRef<'p> = Weak<RefCell<Player<'p>>>;

pub type Supply = HashMap<card::Card, uint>;


// TODO: find a way to derive Default
pub struct Player<'p> {
    priv supply_rc: Rc<RefCell<Supply>>,
    priv name: ~str,
    priv play: PlayerFunc,
    priv player_refs: ~[PlayerRef<'p>],

    priv deck: ~[card::Card],
    priv discard: ~[card::Card],
    priv in_play: ~[card::Card],
    priv hand: ~[card::Card],

    priv actions: uint,
    priv buys: uint,
    priv buying_power: uint,
    priv score: int, // for calculating the final score
}


impl<'p> Eq for Player<'p> {
	fn eq(&self, other: &Player) -> bool {
		self.name == other.name
	}
}


impl<'p> Player<'p> {
    // new() creates a new player. They're given a shuffled deck
    // of 7 coppers and 3 estates.
    pub fn new(name: ~str, play: PlayerFunc) -> Player {
        let mut deck = ~[];
        deck.push_all_move(card::COPPER.create_copies(7));
        deck.push_all_move(card::ESTATE.create_copies(3));
        card::shuffle(deck);
        Player{
            supply_rc: Rc::new(RefCell::new(HashMap::new())),
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
        self.deck.iter()
            .chain(self.discard.iter())
            .chain(self.hand.iter())
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

    // has() returns true if the player has the provided card, anywhere.
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
        self.in_play.push(self.hand.remove(index).unwrap());
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
    pub fn buy(&mut self, c: card::Card) -> Option<error::Error> {
		let pile = unwrap_or_err!(self.count(c), error::NotInSupply);
		if pile == 0 {
			return Some(error::EmptyPile);
		}
		let need = c.get_cost();
		if self.buying_power >= need {
			self.with_mut_supply(|supply| supply.insert(c, pile - 1));
			self.discard.push(c);
			self.actions = 0;
			self.buying_power -= need;
			None
		} else {
			Some(error::NotEnoughMoney(need - self.buying_power))
		}
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
    pub fn count(&mut self, c: card::Card) -> Option<uint> {
		self.with_supply(|supply| {
			if !supply.contains_key(&c) {
				None
			} else {
				Some(*supply.get(&c))
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

    // draw() removes a card from the top of the deck and adds it to the hand. If
    // the deck is empty, then the discard pile and deck are swapped (making
    // the deck equal to the old discard and the discard empty), the deck
    // is shuffled, and the draw is tried again.
    fn draw(&mut self) {
        match self.deck.shift() {
            Some(c) => self.hand.push(c),
            None => {
                // deck is empty, swap it with the discard and shuffle it
                if self.discard.len() == 0 {
                    return;
                } else {
                    mem::swap(&mut self.deck, &mut self.discard);
                    card::shuffle(self.deck);
                    self.draw();
                }
            }
        };
    }

    // calculate_score() counts up the total number of points and saves it
    // in the local score variable.
    fn calculate_score(&mut self) {
        self.score = self.get_total_points();
    }

    // other_players() returns a list of references to the other players
    // in the game, starting with the player on the left and ending with
    // the player on the right.
    fn other_players<'a>(&'a mut self) -> iter::Map<'a, PlayerRef<'p>> {
		self.player_refs.iter()
    }

	/*
    // left_player() returns a reference to the player on the left.
    unsafe fn left_player(&mut self) {
        let player = self.player_refs.iter().skip(1).next().unwrap();
        //&(*ptr::read_ptr(player))
    }

    // right_player() returns a reference to the player on the right.
    unsafe fn right_player(&mut self) {
        let player = self.player_refs.iter().last().unwrap();
        //&(*ptr::read_ptr(player))
    }
	*/

	fn with_mut_supply<U>(&mut self, f: |&mut Supply| -> U) -> U {
		self.supply_rc.borrow().with_mut(f)
	}

	fn with_supply<U>(&mut self, f: |&Supply| -> U) -> U {
		self.supply_rc.borrow().with(f)
	}
}
