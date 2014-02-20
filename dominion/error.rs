
pub enum Error {
    NoActions,
    NoBuys,
    InvalidPlay,
    NotInSupply,
    EmptyPile,
    NotEnoughMoney(int), // how much more is needed to buy the card
}
