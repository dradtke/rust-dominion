
#[deriving(Show)]
pub enum Error {
    NoActions,
    NoBuys,
    InvalidPlay,
    NotInSupply,
	NotInHand,
    EmptyPile,
    NotEnoughMoney(uint), // how much more is needed to buy the card
}
