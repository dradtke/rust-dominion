
use super::Player;
use std::rand::{task_rng, Rng};

macro_rules! get_common_value(
    ($val:expr, $field:ident) => (
        match $val {
            Money   { $field: x, .. } => x,
            Victory { $field: x, .. } => x,
            Action  { $field: x, .. } => x,
            Curse   { $field: x, .. } => x,
        }
    )
)

pub fn shuffle(cards: &mut [Card]) {
    task_rng().shuffle_mut(cards);
}

type ActionFunc = fn(&mut Player, &[ActionInput]);

#[deriving(Hash,Eq)]
pub enum CardDef {
    Money   { name: &'static str, cost: uint, value: uint },
    Victory { name: &'static str, cost: uint, points: int },
    Action  { name: &'static str, cost: uint, action: *ActionFunc },
    Curse   { name: &'static str, cost: uint, points: int },
}

impl TotalOrd for CardDef {
	fn cmp(&self, other: &CardDef) -> Ordering {
		self.get_name().cmp(&other.get_name())
	}
}

impl TotalEq for CardDef {
	fn equals(&self, other: &CardDef) -> bool {
		self.get_name().equals(&other.get_name())
	}
}

impl CardDef {
    #[inline]
    pub fn get_name(&self) -> &'static str {
        get_common_value!(*self, name)
    }

    #[inline]
    pub fn get_cost(&self) -> uint {
        get_common_value!(*self, cost)
    }

    #[inline]
    pub fn get_value(&self) -> uint {
        match *self {
            Money { value: v, .. } => v,
            _ => fail!("Can't get value of non-money card!"),
        }
    }

    #[inline]
    pub fn get_points(&self) -> int {
        match *self {
            Victory { points: p, .. } => p,
            Curse   { points: p, .. } => p,
            _ => fail!("Can't get point value of non-victory and non-curse card!"),
        }
    }

    #[inline]
    pub fn is_money(&self) -> bool {
        match *self {
            Money { .. } => true,
            _ => false,
        }
    }

    #[inline]
    pub fn is_victory(&self) -> bool {
        match *self {
            Victory { .. } => true,
            _ => false,
        }
    }

    #[inline]
    pub fn is_curse(&self) -> bool {
        match *self {
            Curse { .. } => true,
            _ => false,
        }
    }

    #[inline]
    pub fn is_action(&self) -> bool {
        match *self {
            Action { .. } => true,
            _ => false,
        }
    }

    pub fn create_copies(&'static self, n: int) -> ~[Card] {
        let mut cards = ~[];
        for _ in range(0, n) {
            cards.push(self);
        }
        cards
    }
}

/* Card Definitions */
pub type Card = &'static CardDef;

pub static COPPER: Card = &'static Money { name: "Copper", cost: 0, value: 1 };
pub static SILVER: Card = &'static Money { name: "Silver", cost: 3, value: 2 };
pub static GOLD:   Card = &'static Money { name: "Gold",   cost: 6, value: 3 };

pub static ESTATE:   Card = &'static Victory { name: "Estate",   cost: 2, points: 1  };
pub static DUCHY:    Card = &'static Victory { name: "Duchy",    cost: 5, points: 3  };
pub static PROVINCE: Card = &'static Victory { name: "Province", cost: 8, points: 6  };
pub static CURSE:    Card = &'static Curse   { name: "Curse",    cost: 0, points: -1 };

pub static CELLAR: Card = &'static Action { name: "Cellar", cost: 2, action: &do_cellar };
fn do_cellar(p: &mut Player, inputs: &[ActionInput]) {
	p.actions += 1;
	for to_discard in inputs.iter().filter(|i| i.is_discard()) {
		let card = to_discard.unwrap();
		if p.discard(card).is_none() {
			p.draw();
		}
	}
}

pub static CHAPEL: Card = &'static Action { name: "Chapel", cost: 2, action: &do_chapel };
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

pub static SMITHY: Card = &'static Action { name: "Smithy", cost: 3, action: &do_smithy };
fn do_smithy(p: &mut Player, _: &[ActionInput]) {
    for _ in range(0, 3) {
        p.draw();
    }
}

pub static WITCH: Card = &'static Action { name: "Witch", cost: 5, action: &do_witch };
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
