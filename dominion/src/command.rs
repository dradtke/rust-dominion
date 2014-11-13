use super::card::Card;

pub enum Command {
    Buy(Card),
    Play(Card),
    PlayAllMoney,
}
