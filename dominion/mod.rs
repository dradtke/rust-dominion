
use collections::{Deque,DList,HashMap};
use std::cell::RefCell;
use std::rc::Rc;
use std::mem;

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

pub type Supply = HashMap<card::Card, uint>;
pub type PlayerFunc = fn(&mut Player);

pub fn play_game(p: ~[(~str, PlayerFunc)]) -> Option<~str> {
    let trash = ~[];

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

    let game = Game{ supply: supply, trash: trash };
    let game_rc = Rc::new(RefCell::new(game));

    let empty_limit = get_empty_limit(p.len());
    let players_rc = init(p, &game_rc);

    'game: loop {
        let players_cell = players_rc.borrow();
        let mut player = players_cell.with_mut(|ps| ps.pop_front().unwrap());
        take_turn(&mut player);
        let done = game_rc.borrow().with_mut(|game| {
            if *game.supply.find(&card::PROVINCE).unwrap() == 0 {
                true
            } else {
                let num_empty = game.supply.values().filter(|x| **x == 0).fold(0, |a, &b| a + b);
                num_empty >= empty_limit
            }
        });
        let mut players_ref = players_cell.borrow_mut();
        players_ref.get().push_back(player);
        if done {
            break 'game;
        }
    }

    let mut players_ref = players_rc.borrow().borrow_mut();
    let players = players_ref.get();

    // Calculate the results
    let mut highest_score = 0;
    for player in players.mut_iter() {
        player.calculate_score();
        if player.score > highest_score {
            highest_score = player.score;
        }
        /*
        println!("{}:", player.name);
        println!("\t{} Estates", player.number_of(card::ESTATE));
        println!("\t{} Duchies", player.number_of(card::DUCHY));
        println!("\t{} Provinces", player.number_of(card::PROVINCE));
        println!("\t{} Curses", player.number_of(card::CURSE));
        println!("\tFinal Score: {}", player.score);
        */
    }

    let winners = players.iter()
        .filter(|player| player.score == highest_score)
        .to_owned_vec();

    if winners.len() == 1 {
        Some(winners[0].name.clone())
    } else {
        // tie
        None
    }
}

fn init(p: ~[(~str, PlayerFunc)], game_rc: &Rc<RefCell<Game>>) -> Rc<RefCell<DList<Player>>> {
    let mut deck = ~[];
    deck.push_all_move(card::COPPER.create_copies(7));
    deck.push_all_move(card::ESTATE.create_copies(3));
    card::shuffle(deck);

    let players_rc = Rc::new(RefCell::new(DList::new()));

    for (name, func) in p.move_iter() {
        let mut ps = players_rc.borrow().borrow_mut();
        ps.get().push_back(Player{
            name:          name,
            game_rc:       game_rc.clone(),
            other_players: players_rc.clone(),
            play:          func,
            deck:          deck.clone(),
            discard:       ~[],
            in_play:       ~[],
            hand:          ~[],
            actions:       0,
            buys:          0,
            buying_power:  0,
            score:         0,
        });
    }
    players_rc
}

fn take_turn(player: &mut Player) {
    player.new_hand();
    player.actions = 1;
    player.buys = 1;
    player.buying_power = 0;
    (player.play)(player);
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

struct Game {
    supply: Supply,
    trash: ~[card::Card],
}

// TODO: find a way to derive Default
pub struct Player {
    priv game_rc: Rc<RefCell<Game>>,
    priv other_players: Rc<RefCell<DList<Player>>>,

	priv name: ~str,
	priv play: PlayerFunc,
	//priv player_refs: ~[PlayerRef<'p>],

	priv deck: ~[card::Card],
	priv discard: ~[card::Card],
	priv in_play: ~[card::Card],
	priv hand: ~[card::Card],

	priv actions: uint,
	priv buys: uint,
	priv buying_power: uint,
	priv score: int, // for calculating the final score
}


impl Eq for Player {
	fn eq(&self, other: &Player) -> bool {
		self.name == other.name
	}
}


impl Player {
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

    // number_of() returns the number of instances of the provided card
    // that the player has, anywhere.
    pub fn number_of(&self, c: card::Card) -> uint {
        self.hand.iter().count(|&x| x == c)
            + self.deck.iter().count(|&x| x == c)
            + self.discard.iter().count(|&x| x == c)
            + self.in_play.iter().count(|&x| x == c)
    }

	// hand_contains() returns true if and only if the player's hand contains
	// the specified card.
	pub fn hand_contains(&self, c: card::Card) -> bool {
		self.hand.iter().any(|&x| x == c)
	}

	pub fn play(&mut self, c: card::Card) -> Option<error::Error> {
		self.play_and(c, [])
	}

	// play() plays a card. It returns an InvalidPlay error if either (a) the requested
	// card is not in the player's hand, or (b) the card cannot be played, e.g. Province.
	// Other errors may occur if there are not enough actions or buys, and once a Money
	// card is played, then the player's action count is set to 0.
	pub fn play_and(&mut self, c: card::Card, input: &[card::ActionInput]) -> Option<error::Error> {
		let index = unwrap_or_err!(self.hand.iter().position(|&x| x == c), error::InvalidPlay);
		self.in_play.push(self.hand.remove(index).unwrap());
		match *c {
			card::Money { value: v, .. } => {
				self.buying_power += v;
				self.actions = 0;
				None
			},
			card:: Action { action: a, .. } => {
				if self.actions == 0 {
					Some(error::NoActions)
				} else {
					unsafe {
						self.actions -= 1;
						(*a)(self, input);
					}
					None
				}
			},
			_ => Some(error::InvalidPlay),
		}
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

	// discard() discards a card from the player's hand, adding it to the
	// discard pile. If it's not in the player's hand than a NotInHand
	// error is returned.
	fn discard(&mut self, c: card::Card) -> Option<error::Error> {
		match self.hand.iter().enumerate().find(|&(_,x)| *x == c) {
			None => Some(error::NotInHand),
			Some((i,_)) => {
				self.discard.push(self.hand.remove(i).unwrap());
				None
			},
		}
	}

	// trash() trashes a card from the player's hand, adding it to the
	// shared trash pile. If it's not in the player's hand than a NotInHand
	// error is returned.
	fn trash(&mut self, c: card::Card) -> Option<error::Error> {
		match self.hand.iter().enumerate().find(|&(_,x)| *x == c) {
			None => Some(error::NotInHand),
			Some((i,_)) => {
				let card = self.hand.remove(i).unwrap();
				self.game_rc.borrow().with_mut(|game| {
					game.trash.push(card);
				});
				None
			},
		}
	}

	// calculate_score() counts up the total number of points and saves it
	// in the local score variable.
	fn calculate_score(&mut self) {
		self.score = self.get_total_points();
	}

	fn with_other_players(&mut self, f: |&mut Player|) {
        let mut r = self.other_players.borrow().borrow_mut();
        for other_player in r.get().mut_iter() {
            f(other_player);
        }
	}

	fn with_left_player<U>(&mut self, f: |&mut Player| -> U) -> U {
        let mut r = self.other_players.borrow().borrow_mut();
        f(r.get().mut_iter().next().unwrap())
	}

	fn with_right_player<U>(&mut self, f: |&mut Player| -> U) -> U {
        let mut r = self.other_players.borrow().borrow_mut();
        f(r.get().mut_rev_iter().next().unwrap())
	}

	fn with_mut_supply<U>(&mut self, f: |&mut Supply| -> U) -> U {
        self.game_rc.borrow().with_mut(|game| f(&mut game.supply))
	}

	fn with_supply<U>(&mut self, f: |&Supply| -> U) -> U {
		self.game_rc.borrow().with(|game| f(&game.supply))
	}
}

