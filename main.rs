#![feature(phase)]

#[phase(plugin, link)] extern crate dominion;
use dominion::strat;

struct Georgia;
impl dominion::Player for Georgia {
    fn name(&self) -> &'static str { "Georgia" }
    fn take_turn(&self) { strat::big_money_smithy(); }
}

struct Damien;
impl dominion::Player for Damien {
    fn name(&self) -> &'static str { "Damien" }
    fn take_turn(&self) { strat::big_money_witch(); }
}

fn main() {
    kingdom!(SMITHY, WITCH);
    dominion!(Georgia, Damien);
}
