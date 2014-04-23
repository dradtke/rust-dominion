
use std::vec::Vec;
use super::{Card, CardDef, PlayerState, Money, Victory, Action, Curse, ActionInput};

pub static COPPER: Card = &'static CardDef { name: "Copper", cost: 0, types: &'static[Money(1)] };
pub static SILVER: Card = &'static CardDef { name: "Silver", cost: 3, types: &'static[Money(2)] };
pub static GOLD:   Card = &'static CardDef { name: "Gold", cost: 6, types: &'static[Money(3)] };

pub static ESTATE: Card = &'static CardDef { name: "Estate", cost: 2, types: &'static[Victory(get_estate_value)] };
fn get_estate_value(_: &PlayerState) -> int { 1 }

pub static DUCHY: Card = &'static CardDef { name: "Duchy", cost: 5, types: &'static[Victory(get_duchy_value)] };
fn get_duchy_value(_: &PlayerState) -> int { 3 }

pub static PROVINCE: Card = &'static CardDef { name: "Province", cost: 8, types: &'static[Victory(get_province_value)] };
fn get_province_value(_: &PlayerState) -> int { 6 }

pub static CURSE: Card = &'static CardDef { name: "Curse", cost: 0, types: &'static[Curse(-1)] };

/* ---------------------------- Cellar ---------------------------- */

pub static CELLAR: Card = &'static CardDef { name: "Cellar", cost: 2, types: &'static[Action(do_cellar)] };
fn do_cellar(p: &mut PlayerState, inputs: &[ActionInput]) {
	p.actions += 1;
	for to_discard in inputs.iter().filter(|i| i.is_discard()) {
		let card = to_discard.unwrap();
		if p.discard(card).is_none() {
			p.draw();
		}
	}
}

/* ---------------------------- Chapel ---------------------------- */

pub static CHAPEL: Card = &'static CardDef { name: "Chapel", cost: 2, types: &[Action(do_chapel)] };
fn do_chapel(p: &mut PlayerState, inputs: &[ActionInput]) {
	let mut trashed = 0;
	for to_trash in inputs.iter().filter(|i| i.is_trash()) {
		let card = to_trash.unwrap();
		if p.trash(card).is_none() {
			trashed += 1;
			if trashed >= 4 {
				break;
			}
		}
	}
}

/* ---------------------------- Moat ---------------------------- */

pub static MOAT: Card = &'static CardDef { name: "Moat", cost: 2, types: &[Action(do_moat)] };
fn do_moat(p: &mut PlayerState, _: &[ActionInput]) {
    for _ in range(0, 2) {
        p.draw();
    }
}

/* ---------------------------- Chancellor ---------------------------- */

pub static CHANCELLOR: Card = &'static CardDef { name: "Chancellor", cost: 3, types: &[Action(do_chancellor)] };
fn do_chancellor(p: &mut PlayerState, inputs: &[ActionInput]) {
    p.buying_power += 2;
    if inputs.iter().any(|i| i.is_confirm()) {
        p.discard_deck();
    }
}

/* ---------------------------- Village ---------------------------- */

pub static VILLAGE: Card = &'static CardDef { name: "Village", cost: 3, types: &[Action(do_village)] };
fn do_village(p: &mut PlayerState, _: &[ActionInput]) {
    p.draw();
    p.actions += 2;
}

/* ---------------------------- Woodcutter ---------------------------- */

pub static WOODCUTTER: Card = &'static CardDef { name: "Woodcutter", cost: 3, types: &[Action(do_woodcutter)] };
fn do_woodcutter(p: &mut PlayerState, _: &[ActionInput]) {
    p.buys += 1;
    p.buying_power += 2;
}

/* ---------------------------- Workshop ---------------------------- */

pub static WORKSHOP: Card = &'static CardDef { name: "Workshop", cost: 3, types: &[Action(do_workshop)] };
fn do_workshop(p: &mut PlayerState, inputs: &[ActionInput]) {
    let card = inputs.iter().find(|i| i.is_gain()).unwrap().unwrap();
    if card.cost <= 4 {
        p.gain(card);
    }
}

/* ---------------------------- Bureaucrat ---------------------------- */

pub static BUREAUCRAT: Card = &'static CardDef { name: "Bureaucrat", cost: 4, types: &[Action(do_bureaucrat)] };
fn do_bureaucrat(p: &mut PlayerState, _: &[ActionInput]) {
    p.gain_to_deck(SILVER);
    // allow other players input on what card is used?
    p.attack(|other: &mut PlayerState| {
        match other.hand.iter().find(|c| c.is_victory()) {
            Some(c) => other.deck.unshift(*c),
            None => (),
        }
    });
}

/* ---------------------------- Feast ---------------------------- */

pub static FEAST: Card = &'static CardDef { name: "Feast", cost: 4, types: &[Action(do_feast)] };
fn do_feast(p: &mut PlayerState, inputs: &[ActionInput]) {
    p.trash_from_play(FEAST);
    let card = inputs.iter().find(|i| i.is_gain()).unwrap().unwrap();
    if card.cost <= 5 {
        p.gain(card);
    }
}

/* ---------------------------- Gardens ---------------------------- */

pub static GARDENS: Card = &'static CardDef { name: "Gardens", cost: 4, types: &[Victory(get_gardens_value)] };
fn get_gardens_value(p: &PlayerState) -> int {
    (p.deck.len() as int) / 10
}

/* ---------------------------- Militia ---------------------------- */

pub static MILITIA: Card = &'static CardDef { name: "Militia", cost: 4, types: &[Action(do_militia)] };
fn do_militia(p: &mut PlayerState, _: &[ActionInput]) {
    p.buying_power += 2;
    p.attack(|other: &mut PlayerState| {
        loop {
            if other.hand.len() <= 3 {
                break;
            }
            // TODO: find a way for players to choose the cards that are discarded
            other.discard_first();
        }
    });
}

/* ---------------------------- Moneylender ---------------------------- */

pub static MONEYLENDER: Card = &'static CardDef { name: "Moneylender", cost: 4, types: &[Action(do_moneylender)] };
fn do_moneylender(p: &mut PlayerState, _: &[ActionInput]) {
    if !p.hand_contains(COPPER) {
        return;
    }
    p.trash(COPPER);
    p.buying_power += 3;
}

/* ---------------------------- Remodel ---------------------------- */

pub static REMODEL: Card = &'static CardDef { name: "Remodel", cost: 4, types: &[Action(do_remodel)] };
fn do_remodel(p: &mut PlayerState, inputs: &[ActionInput]) {
    let to_trash = inputs.iter().find(|i| i.is_trash()).unwrap().unwrap();
    if !p.hand_contains(to_trash) {
        return;
    }
    let to_gain = inputs.iter().find(|i| i.is_gain()).unwrap().unwrap();
    if to_gain.cost > to_trash.cost + 2 {
        return;
    }
    p.trash(to_trash);
    p.gain(to_gain);
}

/* ---------------------------- Smithy ---------------------------- */

pub static SMITHY: Card = &'static CardDef { name: "Smithy", cost: 4, types: &[Action(do_smithy)] };
fn do_smithy(p: &mut PlayerState, _: &[ActionInput]) {
    for _ in range(0, 3) {
        p.draw();
    }
}

/* ---------------------------- Spy ---------------------------- */

pub static SPY: Card = &'static CardDef { name: "Spy", cost: 4, types: &[Action(do_spy)] };
fn do_spy(p: &mut PlayerState, _: &[ActionInput]) {
    p.draw();
    p.actions += 1;
    p.attack(|other| {
        // TODO: get input from the player on where to put this
        other.mill();
    });
    // TODO: do the same thing for yourself
}

/* ---------------------------- Thief ---------------------------- */

pub static THIEF: Card = &'static CardDef { name: "Thief", cost: 4, types: &[Action(do_thief)] };
fn do_thief(p: &mut PlayerState, _: &[ActionInput]) {
    let mut gained = Vec::new();
    p.attack(|other| {
        let (mut money, non_money) = other.next_n_cards(2).partition(|c| c.is_money());
        for c in non_money.iter() {
            other.discard(*c);
        }
        if money.is_empty() {
            return;
        }
        money.sort_by(|m1, m2| m2.treasure_value().cmp(&m1.treasure_value())); // TODO: verify the ordering, highest should be first
        let mut iter = money.iter();
        let chosen = *iter.next().unwrap();
        other.trash(chosen);
        gained.push(chosen);
        for rest in iter {
            other.discard(*rest);
        }
    });
    for c in gained.iter() {
        p.gain(*c);
    }
}

/* ---------------------------- Throne Room ---------------------------- */

pub static THRONE_ROOM: Card = &'static CardDef { name: "Throne Room", cost: 4, types: &[Action(do_throne_room)] };
fn do_throne_room(p: &mut PlayerState, inputs: &[ActionInput]) {
    let (c, f) = match *inputs.iter().find(|i| i.is_repeat()).unwrap() {
        super::Repeat(c, f) => (c, f),
        _ => fail!("Invalid Throne Room input!"),
    };
    if !c.is_action() {
        fail!("Can't play Throne Room on non-Action card!");
    }
    let action = c.get_action();
    for i in range(0u, 2u) {
        let input = f(p, i);
        action(p, input.as_slice());
    }
}

/* ---------------------------- Council Room ---------------------------- */

pub static COUNCIL_ROOM: Card = &'static CardDef { name: "Council Room", cost: 5, types: &[Action(do_council_room)] };
fn do_council_room(p: &mut PlayerState, _: &[ActionInput]) {
    for _ in range(0, 4) {
        p.draw();
    }
    p.buys += 1;
    p.with_other_players(|other| {
        other.draw();
    });
}

/* ---------------------------- Festival ---------------------------- */

pub static FESTIVAL: Card = &'static CardDef { name: "Festival", cost: 5, types: &[Action(do_festival)] };
fn do_festival(p: &mut PlayerState, _: &[ActionInput]) {
    p.actions += 2;
    p.buys += 1;
    p.buying_power += 2;
}

/* ---------------------------- Laboratory ---------------------------- */

pub static LABORATORY: Card = &'static CardDef { name: "Laboratory", cost: 5, types: &[Action(do_laboratory)] };
fn do_laboratory(p: &mut PlayerState, _: &[ActionInput]) {
    for _ in range(0, 2) {
        p.draw();
    }
    p.actions += 1;
}

/* ---------------------------- Library ---------------------------- */

pub static LIBRARY: Card = &'static CardDef { name: "Library", cost: 5, types: &[Action(do_library)] };
fn do_library(p: &mut PlayerState, _: &[ActionInput]) {
    // TODO: let the player discard action cards as they draw
    while p.hand.len() < 7 {
        p.draw();
    }
}

/* ---------------------------- Market ---------------------------- */

pub static MARKET: Card = &'static CardDef { name: "Market", cost: 5, types: &[Action(do_market)] };
fn do_market(p: &mut PlayerState, _: &[ActionInput]) {
    p.draw();
    p.actions += 1;
    p.buys += 1;
    p.buying_power += 1;
}

/* ---------------------------- Mine ---------------------------- */

pub static MINE: Card = &'static CardDef { name: "Mine", cost: 5, types: &[Action(do_mine)] };
fn do_mine(p: &mut PlayerState, inputs: &[ActionInput]) {
    let to_trash = inputs.iter().find(|x| x.is_trash()).unwrap().unwrap();
    if !p.hand.contains(&to_trash) || !to_trash.is_money() {
        return;
    }
    let to_gain = inputs.iter().find(|x| x.is_gain()).unwrap().unwrap();
    if to_gain.treasure_value() > (to_trash.treasure_value() + 3) || !to_gain.is_money() {
        return;
    }
    p.trash(to_trash);
    p.gain_to_hand(to_gain);
}

/* ---------------------------- Witch ---------------------------- */

pub static WITCH: Card = &'static CardDef { name: "Witch", cost: 5, types: &[Action(do_witch)] };
fn do_witch(p: &mut PlayerState, _: &[ActionInput]) {
    for _ in range(0, 2) {
        p.draw();
    }
    p.attack(|other| {
        other.curse();
    });
}

/* ---------------------------- Adventurer ---------------------------- */

pub static ADVENTURER: Card = &'static CardDef { name: "Adventurer", cost: 6, types: &[Action(do_adventurer)] };
fn do_adventurer(p: &mut PlayerState, _: &[ActionInput]) {
    let mut count = 0;
    while count < 2 {
        match p.next_card() {
            Some(c) => {
                if c.is_money() {
                    count += 1;
                    p.hand.push(c);
                } else {
                    p.discard.push(c);
                }
            },
            None => break,
        }
    }
}


/* ---------------------------- Testing ---------------------------- */


#[cfg(test)]
mod tests {
    use card = super::super::card;
    use error = super::super::error;
    use collections::{DList,HashMap};
    use super::super::{Card, PlayerState, Supply, Game, Discard};
    use std::rc::Rc;
    use std::cell::RefCell;
    use std::vec::Vec;

    macro_rules! assert_no_error(
        ($val:expr) => (
            match $val {
                None => (),
                Some(err) => match err {
                    error::InvalidPlay => fail!("Invalid play!"),
                    error::NoActions => fail!("No actions left!"),
                    _ => fail!("Unknown error!"),
                },
            }
        )
    )

    fn dont_play(_: &mut PlayerState) {
    }

    fn setup(hand: Vec<Card>, deck: Vec<Card>) -> PlayerState {
        let trash = Vec::new();

        let mut supply: Supply = HashMap::new();
        supply.insert(card::COPPER,   30);
        supply.insert(card::SILVER,   30);
        supply.insert(card::GOLD,     30);
        supply.insert(card::ESTATE,   12);
        supply.insert(card::DUCHY,    12);
        supply.insert(card::PROVINCE, 12);
        supply.insert(card::CURSE,    30);
        supply.insert(card::SMITHY,   10);
        supply.insert(card::WITCH,    10);

        let game = Game{ supply: supply, trash: trash };
        let game_rc = Rc::new(RefCell::new(game));
        let players_rc = Rc::new(RefCell::new(DList::new()));

        PlayerState{
            //name:          ~"PlayerState",
            game_rc:       game_rc.clone(),
            other_players: players_rc.clone(),
            //play:          dont_play,
            deck:          deck,
            discard:       Vec::new(),
            in_play:       Vec::new(),
            hand:          hand,
            actions:       1,
            buys:          1,
            buying_power:  0,
            score:         0,
        }
    }

    #[test]
    fn test_cellar() {
        let mut player = setup(vec!(card::CELLAR, card::ESTATE, card::ESTATE, card::COPPER), vec!(card::SILVER, card::GOLD));
        assert_no_error!(player.play_and(card::CELLAR, vec!(Discard(card::ESTATE), Discard(card::ESTATE)).as_slice()));
        assert_eq!(player.hand.len(), 3);
        assert_eq!(player.actions, 1);
        assert!(*player.hand.get(0) == card::COPPER);
        assert!(*player.hand.get(1) == card::SILVER);
        assert!(*player.hand.get(2) == card::GOLD);
    }
}
