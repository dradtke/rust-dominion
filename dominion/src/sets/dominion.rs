use super::super::{GameState, Player, PlayerHandle};
use super::super::card::*;
use super::super::notify::*;
use super::super::reaction::*;
use super::super::response::*;

pub fn cellar(player: &mut PlayerHandle, to_discard: &[Card]) -> Response {
    player.actions += 1;
    for card in to_discard.iter() {
        player.discard(*card);
        player.draw();
    }
    NoProblem
}

pub fn chapel(player: &mut PlayerHandle, state: &mut GameState, to_trash: &[Card]) -> Response {
    for card in to_trash.iter().take(4) {
        player.trash(state, *card);
    }
    NoProblem
}

pub fn militia<'a, T: Iterator<&'a mut PlayerHandle>>(player: &mut PlayerHandle, mut opponents: T) -> Response {
    player.buying_power += 2;
    for opponent in opponents {
        for _ in range(3, opponent.get_hand_size()) {
            opponent.notify_chan.send(Militia);
            match opponent.react_port.recv() {
                MilitiaDiscard(card) => opponent.discard(card),
                RevealMoat => opponent.has_or_else(Moat, || panic!("player tried to block Militia with Moat, but he didn't have one!")),
                NotImplemented => {
                    let card = opponent.get_hand()[0];
                    opponent.discard(card);
                },
                resp => panic!("player had to react to Militia, but responded with {}!", resp),
            }
        }
    }
    NoProblem
}

pub fn moat(player: &mut PlayerHandle) -> Response {
    player.draw_n(2);
    NoProblem
}
