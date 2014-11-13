//! Strategy examples.

use super::cards;
use super::{buy, count, hand_contains, has, get_buying_power, play_all_money, play_card};

/// Big Money.
///
/// This is the most basic Dominion strategy. It focuses only on buying
/// money and victory points, using the following rules:
///
///   1. With 8 or more money, buy a province (duh).
///   2. With 6-7 money, buy a gold.
///   3. With 3-4 money, buy a silver
///   4. With 5 money...
///         a) If there are five or fewer provinces left, buy a duchy.
///         v) Otherwise, buy a silver.
///
pub fn big_money() {
    play_all_money();
    match get_buying_power() {
        0..2 => (),
        3..4 => { buy(cards::SILVER); },
        5    => {
            if count(cards::PROVINCE).unwrap() <= 5 {
                buy(cards::DUCHY);
            } else {
                buy(cards::SILVER);
            }
        }
        6..7 => { buy(cards::GOLD); },
        _    => { buy(cards::PROVINCE); },
    }
}

/// Big Money Smithy.
///
/// Same basic premise as Big Money, except one Smithy will be purchased
/// with exactly 4 money.
pub fn big_money_smithy() {
    if hand_contains(cards::dominion::SMITHY) {
        ::play_card(cards::dominion::SMITHY, []);
    }
    play_all_money();
    match get_buying_power() {
        0..2 => (),
        3 => { buy(cards::SILVER); },
        4 => {
            if !has(cards::dominion::SMITHY) {
                buy(cards::dominion::SMITHY);
            } else {
                buy(cards::SILVER);
            }
        },
        5 => {
            if count(cards::PROVINCE).unwrap() <= 5 {
                buy(cards::DUCHY);
            } else {
                buy(cards::SILVER);
            }
        }
        6..7 => { buy(cards::GOLD); },
        _    => { buy(cards::PROVINCE); },
    }
}

/// Big Money Witch.
///
/// Same basic premise as Big Money, except one Witch will be purchased
/// with exactly 5 money.
pub fn big_money_witch() {
    if hand_contains(cards::dominion::WITCH) {
        play_card(cards::dominion::WITCH, []);
    }
    play_all_money();
    match get_buying_power() {
        0..2 => (),
        3..4 => { buy(cards::SILVER); },
        5 => {
            if !has(cards::dominion::WITCH) {
                buy(cards::dominion::WITCH);
            }
            else if count(cards::PROVINCE).unwrap() <= 5 {
                buy(cards::DUCHY);
            } else {
                buy(cards::SILVER);
            }
        }
        6..7 => { buy(cards::GOLD); },
        _    => { buy(cards::PROVINCE); },
    }
}
