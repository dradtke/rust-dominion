#[deriving(Show)]
pub enum Query {
    BuyingPower,
    Hand,
    HandSize,
    HasInHand(::card::Card),
}
