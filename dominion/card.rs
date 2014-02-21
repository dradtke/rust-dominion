
use super::{Player,PlayerFunc};
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

#[deriving(IterBytes,Eq)]
pub enum CardDef {
    Money   { name: &'static str, cost: uint, value: uint },
    Victory { name: &'static str, cost: uint, points: int },
    Action  { name: &'static str, cost: uint, action: *PlayerFunc },
    Curse   { name: &'static str, cost: uint, points: int },
}

impl CardDef {
    #[inline] pub fn get_name(&self) -> &'static str {
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

    pub fn act(&'static self, player: &mut Player) {
        if !self.is_action() {
            fail!("Not an action card!");
        }
        match self.get_name() {
            "Smithy" => do_smithy(player),
            _ => fail!("Not implemented!"),
        }
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

pub static smithy: CardDef = Action { name: "Smithy", cost: 3, action: &do_smithy };
fn do_smithy(p: &mut Player) {
    for _ in range(0, 3) {
        p.draw();
    }
}

pub static witch: CardDef = Action { name: "Witch", cost: 5, action: &do_witch };
fn do_witch(p: &mut Player) {
    for _ in range(0, 2) {
        p.draw();
    }
    unsafe {
        let mut others = p.other_players();
        for player in others.mut_iter() {
            player.curse();
        }
    }
}
