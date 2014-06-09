
use super::card;

// Big Money is the most basic Dominion strategy. It focuses only on buying
// money and victory points, using the following rules:
//   1. With 8 or more money, buy a province (duh)
//   2. With 6-7 money, buy a gold.
//   3. With 3-4 money, buy a silver
//   4. With 5 money, if there are five or fewer provinces left, buy a duchy.
//        Otherwise, buy a silver.
pub fn big_money() {
    ::play_all_money();
    match ::get_buying_power() {
        0..2 => Ok(()),
        3..4 => ::buy(card::SILVER),
        5    => {
            if ::count(card::PROVINCE).unwrap() <= 5 {
                ::buy(card::DUCHY)
            } else {
                ::buy(card::SILVER)
            }
        }
        6..7 => ::buy(card::GOLD),
        _    => ::buy(card::PROVINCE),
    }.unwrap();
}

pub fn big_money_smithy() {
    if ::hand_contains(card::SMITHY) {
        ::play_card(card::SMITHY).unwrap();
    }
    ::play_all_money();
    match ::get_buying_power() {
        0..2 => Ok(()),
        3 => ::buy(card::SILVER),
        4 => {
            if !::has(card::SMITHY) {
                ::buy(card::SMITHY)
            } else {
                ::buy(card::SILVER)
            }
        },
        5 => {
            if ::count(card::PROVINCE).unwrap() <= 5 {
                ::buy(card::DUCHY)
            } else {
                ::buy(card::SILVER)
            }
        }
        6..7 => ::buy(card::GOLD),
        _    => ::buy(card::PROVINCE),
    }.unwrap();
}

pub fn big_money_witch() {
    if ::hand_contains(card::WITCH) {
        ::play_card(card::WITCH).unwrap();
    }
    ::play_all_money();
    match ::get_buying_power() {
        0..2 => Ok(()),
        3..4 => ::buy(card::SILVER),
        5 => {
            if !::has(card::WITCH) {
                ::buy(card::WITCH)
            }
            else if ::count(card::PROVINCE).unwrap() <= 5 {
                ::buy(card::DUCHY)
            } else {
                ::buy(card::SILVER)
            }
        }
        6..7 => ::buy(card::GOLD),
        _    => ::buy(card::PROVINCE),
    }.unwrap();
}

pub fn cellaring() {
	::play_card(card::CELLAR).unwrap();
}
