#[allow(dead_code)];

extern crate dominion;

use std::os;

fn main() {
    let args = os::args();
    let n: uint = if args.len() > 1 {
        from_str(args[1]).unwrap()
    } else {
        1000
    };

    dominion::play_many(n, vec!((~"Damien", dominion::strat::big_money_witch), (~"Georgia", dominion::strat::big_money_smithy)));
}
