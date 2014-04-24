#![feature(phase)]

#[phase(syntax, link)]
extern crate dominion;

use dominion::strat::{big_money_smithy, big_money_witch};

new_player!(Georgia, big_money_smithy)
new_player!(Damien, big_money_witch)

fn main() {
    play!(Damien, Georgia);
}
