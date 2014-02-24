#[feature(struct_variant)];
#[feature(macro_rules)];
#[allow(dead_code)];

extern crate extra;

use dominion::{Player,play};
use std::comm::Chan;
use std::hashmap::HashMap;
use std::os;

mod dominion;

fn main() {
    let (port, chan) = Chan::new();
    let args = os::args();
    if args.len() > 1 {
        let n: int = from_str(args[1]).unwrap();
        for _ in range(0, n) {
            let done = chan.clone();
            spawn(proc() {
                let winner = play(~[
                     Player::new(~"Georgia", dominion::strat::big_money_smithy),
                     Player::new(~"Damien", dominion::strat::big_money_witch),
                ]);
                done.send(winner);
            });
        }
        let mut scores = HashMap::<~str,uint>::new();
        for _ in range(0, n) {
            let winner = port.recv();
            if winner.is_some() {
                let name = winner.unwrap();
                if !scores.contains_key(&name) {
                    scores.insert(name, 1);
                } else {
                    let new_score = scores.get(&name) + 1;
                    scores.insert(name, new_score);
                }
            }
        }

        for key in scores.keys() {
            println!("{} won {} times", *key, *scores.get(key));
        }
    }
}
