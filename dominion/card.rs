
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

#[deriving(IterBytes, Eq)]
pub enum CardDef {
    Money   { name: &'static str, cost: int, value: int },
    Victory { name: &'static str, cost: int, points: int },
    Action  { name: &'static str, cost: int }, // TODO: implement
    Curse   { name: &'static str, cost: int, points: int },
}

impl CardDef {
    #[inline] pub fn get_name(&self) -> &'static str {
        get_common_value!(*self, name)
    }

    #[inline]
    pub fn get_cost(&self) -> int {
        get_common_value!(*self, cost)
    }

    #[inline]
    pub fn get_value(&self) -> int {
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

pub type Card = &'static CardDef;

pub fn shuffle(cards: &mut [Card]) {
    task_rng().shuffle_mut(cards);
}

/* Card Definitions */

pub static copper: CardDef = Money { name: "Copper", cost: 0, value: 1 };
pub static silver: CardDef = Money { name: "Silver", cost: 3, value: 2 };
pub static gold:   CardDef = Money { name: "Gold",   cost: 6, value: 3 };

pub static estate:   CardDef = Victory { name: "Estate",   cost: 2, points: 1  };
pub static duchy:    CardDef = Victory { name: "Duchy",    cost: 5, points: 3  };
pub static province: CardDef = Victory { name: "Province", cost: 8, points: 6  };
pub static curse:    CardDef = Curse   { name: "Curse",    cost: 0, points: -1 };
