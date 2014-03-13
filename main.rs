#[feature(struct_variant)];
#[feature(macro_rules)];
#[feature(default_type_params)];
#[allow(dead_code)];

extern crate collections;
extern crate extra;

use collections::HashMap;
use dominion::play_game;
use std::comm::Chan;
use std::os;

mod dominion;

fn main() {
    let (port, chan) = Chan::new();
    let args = os::args();
    let n: int = if args.len() > 1 {
        from_str(args[1]).unwrap()
    } else {
        1000
    };

    println!("Playing {} games...", n);
    for _ in range(0, n) {
        let done = chan.clone();
        spawn(proc() {
            let winner = dominion::play_game(~[
                                             (~"Damien", dominion::strat::big_money_witch),
                                             (~"Georgia", dominion::strat::big_money_smithy),
                                             ]);
            done.send(winner);
        });
    }

    let mut scores = HashMap::<~str,uint>::new();
    let mut ties = 0;
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
        } else {
            ties += 1;
        }
    }

    for key in scores.keys() {
        println!("{} won {} times", *key, *scores.get(key));
    }
    println!("There were {} ties.", ties);
}
