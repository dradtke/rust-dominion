
use super::Player;
use std::hash::Hash;
use std::vec::Vec;
use rand::{Rng,task_rng};

macro_rules! money(
    ($name:expr costs $cost:expr and gives $value:expr buying power) => {
        &'static CardDef { name: $name, cost: $cost, value: $value, vp: 0, action: 0 as *ActionFunc, typ: Money }
    }
)

macro_rules! victory(
    ($name:expr costs $cost:expr and gives $value:expr victory points) => {
        &'static CardDef { name: $name, cost: $cost, value: 0, vp: $value, action: 0 as *ActionFunc, typ: Victory }
    }
)

macro_rules! action(
    ($name:expr costs $cost:expr and calls $action:ident) => {
        &'static CardDef { name: $name, cost: $cost, value: 0, vp: 0, action: &$action, typ: Action }
    }
)

pub fn shuffle(cards: &mut [Card]) {
    task_rng().shuffle_mut(cards);
}

pub type ActionFunc = fn(&mut Player, &[ActionInput]);

/*
#[deriving(Hash,Eq)]
pub enum CardDef {
    Money   { name: &'static str, cost: uint, value: uint },
    Victory { name: &'static str, cost: uint, points: int },
    Action  { name: &'static str, cost: uint, action: *ActionFunc },
    Curse   { name: &'static str, cost: uint, points: int },
}
*/

#[deriving(Hash,Eq,Ord)]
pub enum CardType {
    NoType =  0u,
    Money = 1u,
    Victory = 2u,
    Action = 4u,
    Curse = 8u,
}

#[deriving(Hash,Ord)]
pub struct CardDef {
    name: &'static str,
    cost: uint,
    value: uint,
    vp: int,
    action: *ActionFunc,
    typ: CardType,
}

impl TotalOrd for CardDef {
	fn cmp(&self, other: &CardDef) -> Ordering {
		self.name.cmp(&other.name)
	}
}

impl TotalEq for CardDef {
	fn equals(&self, other: &CardDef) -> bool {
		self.name.equals(&other.name)
	}
}

impl Eq for CardDef {
	fn eq(&self, other: &CardDef) -> bool {
		self.name.equals(&other.name)
	}
}

impl CardDef {
    #[inline]
    pub fn is(&self, t: CardType) -> bool {
        (self.typ as uint) & (t as uint) != 0
    }

    pub fn create_copies(&'static self, n: int) -> Vec<Card> {
        let mut cards = Vec::new();
        for _ in range(0, n) {
            cards.push(self);
        }
        cards
    }
}

/* Card Definitions */
pub type Card = &'static CardDef;

pub static COPPER: Card = money! {"Copper" costs 0 and gives 1 buying power };
pub static SILVER: Card = money! {"Silver" costs 3 and gives 2 buying power };
pub static GOLD:   Card = money! {"Gold" costs 6 and gives 3 buying power };

pub static ESTATE:   Card = victory! {"Estate" costs 2 and gives 1 victory points };
pub static DUCHY:    Card = victory! {"Duchy" costs 5 and gives 3 victory points };
pub static PROVINCE: Card = victory! {"Province" costs 8 and gives 6 victory points };

pub static CURSE: Card = &'static CardDef {
    name: "Curse", cost: 0, vp: -1, value: 0, action: 0 as *ActionFunc, typ: Curse,
};

pub static CELLAR: Card = action! {"Cellar" costs 2 and calls do_cellar };
fn do_cellar(p: &mut Player, inputs: &[ActionInput]) {
	p.actions += 1;
	for to_discard in inputs.iter().filter(|i| i.is_discard()) {
		let card = to_discard.unwrap();
		if p.discard(card).is_none() {
			p.draw();
		}
	}
}

pub static CHAPEL: Card = action! {"Chapel" costs 2 and calls do_chapel };
fn do_chapel(p: &mut Player, inputs: &[ActionInput]) {
	let mut trashed = 0;
	for to_trash in inputs.iter().filter(|i| i.is_trash()) {
		let card = to_trash.unwrap();
		if p.trash(card).is_none() {
			trashed += 1;
			if trashed >= 4 {
				break;
			}
		}
	}
}

pub static SMITHY: Card = action! {"Smithy" costs 3 and calls do_smithy};
fn do_smithy(p: &mut Player, _: &[ActionInput]) {
    for _ in range(0, 3) {
        p.draw();
    }
}

pub static WITCH: Card = action! {"Witch" costs 5 and calls do_witch };
fn do_witch(p: &mut Player, _: &[ActionInput]) {
    for _ in range(0, 2) {
        p.draw();
    }
    p.with_other_players(|other: &mut Player| { other.curse(); });
}

pub enum ActionInput {
	Discard(Card),
	Trash(Card),
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

	pub fn unwrap(&self) -> Card {
		match *self {
			Discard(c) => c,
			Trash(c) => c,
		}
	}
}
