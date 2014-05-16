#![feature(phase)]

#[phase(syntax, link)] extern crate dominion;
use dominion::strat::{big_money_smithy, big_money_witch};

struct Georgia;
impl dominion::Player for Georgia {
    fn name(&self) -> &'static str { "Georgia" }
    fn take_turn(&self) { big_money_smithy(); }
}

struct Damien;
impl dominion::Player for Damien {
    fn name(&self) -> &'static str { "Damien" }
    fn take_turn(&self) { big_money_witch(); }
}

fn main() {
    play!(Damien, Georgia);
}
