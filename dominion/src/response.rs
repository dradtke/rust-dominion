use super::card::Card;
use super::PendingPlay;

/// Represents a closure that indicates whether the pending play
/// is complete or not.
type IsCompleteFn = |&PendingPlay|:Send -> bool;

type Chans = (Sender<(Card, PendingPlay)>, Receiver<Response>);

/// Game response as an enum.
pub enum Response {
    NoProblem, // rename to `Ok` after enum sub-namespacing occurs
    DontUnderstand,
    NotEnoughActions,
    NotInKingdom(Card),
    PileEmpty(Card),

    Incomplete {
        card: Card,
        pending: PendingPlay,
        chans: Chans,
        is_complete: IsCompleteFn,
    },
}

impl Response {
    pub fn is_err(&self) -> bool {
        match *self {
            DontUnderstand | NotEnoughActions | NotInKingdom(_) | PileEmpty(_) => true,
            NoProblem | Incomplete{..} => false,
        }
    }

    pub fn incomplete(card: Card, pending: PendingPlay, chans: Chans, is_complete: IsCompleteFn) -> Response {
        Incomplete{card: card, pending: pending, chans: chans, is_complete: is_complete}
    }

    pub fn discarding(self, cards: Vec<Card>) -> Response {
        match self {
            Incomplete{card, mut pending, chans, is_complete} => {
                let (play_send, resp_recv) = chans;
                pending.discarding = cards;
                if is_complete(&pending) {
                    play_send.send((card, pending));
                    resp_recv.recv()
                } else {
                    Incomplete{card: card, pending: pending, chans: (play_send, resp_recv), is_complete: is_complete}
                }
            },
            _ => self,
        }
    }
}
