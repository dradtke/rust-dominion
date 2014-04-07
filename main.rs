#![allow(dead_code)]
#![feature(macro_rules)]
#![feature(phase)]

#[phase(syntax, link)] extern crate dominion;

use std::os;

fn main() {
    let args = os::args();
    let n: uint = if args.len() > 1 {
        from_str(args[1]).unwrap()
    } else {
        1000
    };

    play_many!(n games with damien and georgia);
}
