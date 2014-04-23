#![allow(dead_code)]
#![feature(macro_rules)]
#![feature(phase)]

#[phase(syntax, link)] extern crate dominion;

use dominion::{PlayerLike,PlayerState,play_many};
use dominion::strat;
use std::os;

/* Damien */

struct Damien;

impl PlayerLike for Damien {
    fn name(&self) -> ~str {
        ~"Damien"
    }

    fn play(&self, p: &mut PlayerState) {
        strat::big_money_witch(p);
    }
}

/* Georgia */

struct Georgia;

impl PlayerLike for Georgia {
    fn name(&self) -> ~str {
        ~"Georgia"
    }

    fn play(&self, p: &mut PlayerState) {
        strat::big_money_smithy(p);
    }
}

/* Main */

fn main() {
    let args = os::args();
    let n: uint = if args.len() > 1 { from_str(args[1]).unwrap() } else { 1000 };

    // This runs, but loops indefinitely...
    play_many(n, players!(Damien, Georgia));
}
