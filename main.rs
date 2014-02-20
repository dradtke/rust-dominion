#[feature(struct_variant)];
#[feature(macro_rules)];
#[allow(dead_code)];

extern mod extra;

use dominion::{Player,play};
mod dominion;

fn player1(p: &mut Player) {
    // TODO: add AI here
}

fn player2(p: &mut Player) {
    // TODO: add AI here
}

fn main() {
    play([
        Player::new(~"Player 1", player1),
        Player::new(~"Player 2", player2),
    ]);
}
