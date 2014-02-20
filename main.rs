#[feature(struct_variant)];
#[feature(macro_rules)];
#[allow(dead_code)];

extern mod extra;

use dominion::{Player,play};
mod dominion;

fn main() {
    play([
        Player::new(~"Player 1", dominion::strat::big_money),
        Player::new(~"Player 2", dominion::strat::big_money),
    ]);
}
