
use super::{Player};
use super::card;

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
