use super::card::Card;

#[deriving(Show)]
pub enum Reaction {
    NotImplemented,
    MilitiaDiscard(Card),
    RevealMoat,
    OtherReaction,
}
