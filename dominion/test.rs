extern crate dominion;
use dominion::{Connection, Game};

fn damien_fn(conn: &Connection) {
    loop {
        let msg = conn.recv_notification();
        match msg {
            use dominion::*;

            GameOver => break,
            YourTurn(round) => {
                conn.play(card::Copper);
            },

            _ => conn.not_implemented(),
        }
    }
}

fn main() {
    let mut game = Game::new();
    let damien = game.add_player();
    spawn(proc() {
        damien_fn(damien);
    });
    game.play();
}
