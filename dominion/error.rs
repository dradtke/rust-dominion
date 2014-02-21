
pub enum Error {
    NoActions,
    NoBuys,
    InvalidPlay,
    NotInSupply,
    EmptyPile,
    NotEnoughMoney(uint), // how much more is needed to buy the card
}
