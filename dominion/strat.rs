
use super::PlayerState;
use super::card;

// Big Money is the most basic Dominion strategy. It focuses only on buying
// money and victory points, using the following rules:
//   1. With 8 or more money, buy a province (duh)
//   2. With 6-7 money, buy a gold.
//   3. With 3-4 money, buy a silver
//   4. With 5 money, if there are five or fewer provinces left, buy a duchy.
//        Otherwise, buy a silver.
pub fn big_money(p: &mut PlayerState) {
    p.play_all_money();
    match p.get_buying_power() {
        0..2 => None,
        3..4 => p.buy(card::SILVER),
        5    => {
            if p.count(card::PROVINCE).unwrap() <= 5 {
                p.buy(card::DUCHY)
            } else {
                p.buy(card::SILVER)
            }
        }
        6..7 => p.buy(card::GOLD),
        _    => p.buy(card::PROVINCE),
    };
}

pub fn big_money_smithy(p: &mut PlayerState) {
    if p.hand_contains(card::SMITHY) {
        p.play(card::SMITHY);
    }
    p.play_all_money();
    match p.get_buying_power() {
        0..2 => None,
        3 => p.buy(card::SILVER),
        4 => {
            if !p.has(card::SMITHY) {
                p.buy(card::SMITHY)
            } else {
                p.buy(card::SILVER)
            }
        },
        5 => {
            if p.count(card::PROVINCE).unwrap() <= 5 {
                p.buy(card::DUCHY)
            } else {
                p.buy(card::SILVER)
            }
        }
        6..7 => p.buy(card::GOLD),
        _    => p.buy(card::PROVINCE),
    };
}

pub fn big_money_witch(p: &mut PlayerState) {
    if p.hand_contains(card::WITCH) {
        p.play(card::WITCH);
    }
    p.play_all_money();
    match p.get_buying_power() {
        0..2 => None,
        3..4 => p.buy(card::SILVER),
        5 => {
            if !p.has(card::WITCH) {
                p.buy(card::WITCH)
            }
            else if p.count(card::PROVINCE).unwrap() <= 5 {
                p.buy(card::DUCHY)
            } else {
                p.buy(card::SILVER)
            }
        }
        6..7 => p.buy(card::GOLD),
        _    => p.buy(card::PROVINCE),
    };
}

pub fn cellaring(p: &mut PlayerState) {
	p.play(card::CELLAR);
}
