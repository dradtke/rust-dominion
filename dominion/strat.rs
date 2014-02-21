
use super::{Player};
use super::card;

// Big Money is the most basic Dominion strategy. It focuses only on buying
// money and victory points, using the following rules:
//   1. With 8 or more money, buy a province (duh)
//   2. With 6-7 money, buy a gold.
//   3. With 3-4 money, buy a silver
//   4. With 5 money, if there are five or fewer provinces left, buy a duchy.
//        Otherwise, buy a silver.
pub fn big_money(p: &mut Player) {
    p.play_all_money();
    match p.get_buying_power() {
        0..2 => None,
        3..4 => p.buy(&card::silver),
        5    => {
            if p.count(&card::province).unwrap() <= 5 {
                p.buy(&card::duchy)
            } else {
                p.buy(&card::silver)
            }
        }
        6..7 => p.buy(&card::gold),
        _    => p.buy(&card::province),
    };
}
