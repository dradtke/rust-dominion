use super::response;
use super::{GameState, PendingPlay, PlayerHandle};

macro_rules! defcards {
    ($($card:ident [$($typ:expr),+]),+,) => {
        #[deriving(Clone, Show, PartialEq, Eq, Hash)]
        pub enum Card {
            $($card,)+
        }

        impl Card {
            pub fn name(&self) -> &'static str {
                match *self {
                    $($card => stringify!($card),)+
                }
            }

            pub fn is_action(&self) -> bool {
                match *self {
                    $($card => [$($typ),+].iter().any(|t| *t == Action),)+
                }
            }

            pub fn is_money(&self) -> bool {
                match *self {
                    $($card => [$($typ),+].iter().any(|t| *t == Money),)+
                }
            }

            pub fn is_victory(&self) -> bool {
                match *self {
                    $($card => [$($typ),+].iter().any(|t| *t == Victory),)+
                }
            }

            pub fn is_curse(&self) -> bool {
                match *self {
                    $($card => [$($typ),+].iter().any(|t| *t == Curse),)+
                }
            }
        }
    }
}

#[deriving(Show, PartialEq)]
pub enum CardType {
    Action,
    Money,
    Victory,
    Curse,
}

defcards! {
    // Card [Types]
    Copper [Money],
    Silver [Money],
    Gold [Money],

    Cellar [Action],
    Chapel [Action],
    Moat [Action],
    Militia [Action],

    Estate [Victory],
    Duchy [Victory],
    Province [Victory],
}

impl Card {
    pub fn play<'a, T: Iterator<&'a mut PlayerHandle>>(&self, player: &mut PlayerHandle, state: &mut GameState, opponents: T, pending: Option<PendingPlay>) -> response::Response {
        macro_rules! complete_when(
            ($card:expr, $f:expr) => ({
                let (play_complete_chan, play_complete_recv) = channel();
                let (play_complete_resp_chan, play_complete_resp_recv) = channel();
                let i = player.play_complete.len();
                player.play_complete.push((play_complete_resp_chan, play_complete_recv));
                response::Response::incomplete($card, PendingPlay::new(i), (play_complete_chan, play_complete_resp_recv), $f)
            })
        )

        if self.is_action() {
            if player.actions == 0 {
                return response::NotEnoughActions;
            }
            player.actions -= 1;
        } else if self.is_money() {
            player.actions = 0;
        } else {
            return response::DontUnderstand;
        }

        match *self {
            Copper => { player.buying_power += 1; response::NoProblem },
            Silver => { player.buying_power += 2; response::NoProblem },
            Gold => { player.buying_power += 3; response::NoProblem },
            Cellar => match pending {
                Some(x) => ::sets::dominion::cellar(player, x.discarding.as_slice()),
                None => complete_when!(Cellar, |x| -> bool { x.discarding.len() > 0 }),
            },
            Chapel => match pending {
                Some(x) => ::sets::dominion::chapel(player, state, x.trashing.as_slice()),
                None => complete_when!(Chapel, |x| -> bool { x.trashing.len() > 0 }),
            },
            Militia => ::sets::dominion::militia(player, opponents),
            Moat => ::sets::dominion::moat(player),

            Estate | Duchy | Province => unreachable!(),
        }
    }
}
