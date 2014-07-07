#![feature(phase)]

#[phase(plugin, link)]
extern crate dominion;

use dominion::strat;
use dominion::cards::dominion::{SMITHY, WITCH};

player!(Georgia using strat::big_money_smithy)
player!(Damien using strat::big_money_witch)

fn main() {
    kingdom!(SMITHY, WITCH);
    dominion!(Georgia, Damien);
}
