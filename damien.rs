use dominion::Player;
use dominion::strat;

#[inline(always)]
pub fn name() -> ~str { ~"Damien" }

#[inline]
pub fn play(p: &mut Player) {
    strat::big_money_witch(p)
}
