
use std::vec::Vec;
use super::{with_active_player, with_other_players, attack, Card, CardDef, PlayerState, Money, Victory, Action, Curse, ActionInput};

pub static COPPER: Card = &'static CardDef { name: "Copper", cost: 0, types: &'static[Money(1)] };
pub static SILVER: Card = &'static CardDef { name: "Silver", cost: 3, types: &'static[Money(2)] };
pub static GOLD:   Card = &'static CardDef { name: "Gold", cost: 6, types: &'static[Money(3)] };

pub static ESTATE: Card = &'static CardDef { name: "Estate", cost: 2, types: &'static[Victory(get_estate_value)] };
fn get_estate_value() -> int { 1 }

pub static DUCHY: Card = &'static CardDef { name: "Duchy", cost: 5, types: &'static[Victory(get_duchy_value)] };
fn get_duchy_value() -> int { 3 }

pub static PROVINCE: Card = &'static CardDef { name: "Province", cost: 8, types: &'static[Victory(get_province_value)] };
fn get_province_value() -> int { 6 }

pub static CURSE: Card = &'static CardDef { name: "Curse", cost: 0, types: &'static[Curse(-1)] };

/* ---------------------------- Cellar ---------------------------- */

pub static CELLAR: Card = &'static CardDef { name: "Cellar", cost: 2, types: &'static[Action(do_cellar)] };
fn do_cellar(inputs: &[ActionInput]) {
    with_active_player(|player| {
        player.actions += 1;
        for to_discard in inputs.iter().filter(|i| i.is_discard()) {
            let card = to_discard.unwrap();
            if player.discard(card).is_none() {
                player.draw();
            }
        }
    });
}

/* ---------------------------- Chapel ---------------------------- */

// TODO: refactor all of these to use with_active_state() instead of a hardcoded PlayerState
pub static CHAPEL: Card = &'static CardDef { name: "Chapel", cost: 2, types: &[Action(do_chapel)] };
fn do_chapel(inputs: &[ActionInput]) {
    with_active_player(|player| {
        let mut trashed = 0;
        for to_trash in inputs.iter().filter(|i| i.is_trash()) {
            let card = to_trash.unwrap();
            if player.trash(card).is_none() {
                trashed += 1;
                if trashed >= 4 {
                    break;
                }
            }
        }
    });
}

/* ---------------------------- Moat ---------------------------- */

pub static MOAT: Card = &'static CardDef { name: "Moat", cost: 2, types: &[Action(do_moat)] };
fn do_moat(_: &[ActionInput]) {
    with_active_player(|player| {
        for _ in range(0, 2) {
            player.draw();
        }
    });
}

/* ---------------------------- Chancellor ---------------------------- */

pub static CHANCELLOR: Card = &'static CardDef { name: "Chancellor", cost: 3, types: &[Action(do_chancellor)] };
fn do_chancellor(inputs: &[ActionInput]) {
    with_active_player(|player| {
        player.buying_power += 2;
        if inputs.iter().any(|i| i.is_confirm()) {
            player.discard_deck();
        }
    });
}

/* ---------------------------- Village ---------------------------- */

pub static VILLAGE: Card = &'static CardDef { name: "Village", cost: 3, types: &[Action(do_village)] };
fn do_village(_: &[ActionInput]) {
    with_active_player(|player| {
        player.draw();
        player.actions += 2;
    });
}

/* ---------------------------- Woodcutter ---------------------------- */

pub static WOODCUTTER: Card = &'static CardDef { name: "Woodcutter", cost: 3, types: &[Action(do_woodcutter)] };
fn do_woodcutter(_: &[ActionInput]) {
    with_active_player(|player| {
        player.buys += 1;
        player.buying_power += 2;
    });
}

/* ---------------------------- Workshop ---------------------------- */

pub static WORKSHOP: Card = &'static CardDef { name: "Workshop", cost: 3, types: &[Action(do_workshop)] };
fn do_workshop(inputs: &[ActionInput]) {
    with_active_player(|player| {
        let card = inputs.iter().find(|i| i.is_gain()).unwrap().unwrap();
        if card.cost <= 4 {
            player.gain(card);
        }
    });
}

/* ---------------------------- Bureaucrat ---------------------------- */

pub static BUREAUCRAT: Card = &'static CardDef { name: "Bureaucrat", cost: 4, types: &[Action(do_bureaucrat)] };
fn do_bureaucrat(_: &[ActionInput]) {
    with_active_player(|player| {
        player.gain_to_deck(SILVER);
    });
    // allow other players input on what card is used?
    attack(|other: &mut PlayerState| {
        match other.hand.iter().find(|c| c.is_victory()) {
            Some(c) => other.deck.unshift(*c),
            None => (),
        }
    });
}

/* ---------------------------- Feast ---------------------------- */

pub static FEAST: Card = &'static CardDef { name: "Feast", cost: 4, types: &[Action(do_feast)] };
fn do_feast(inputs: &[ActionInput]) {
    with_active_player(|player| {
        player.trash_from_play(FEAST);
        let card = inputs.iter().find(|i| i.is_gain()).unwrap().unwrap();
        if card.cost <= 5 {
            player.gain(card);
        }
    });
}

/* ---------------------------- Gardens ---------------------------- */

pub static GARDENS: Card = &'static CardDef { name: "Gardens", cost: 4, types: &[Victory(get_gardens_value)] };
fn get_gardens_value() -> int {
    with_active_player(|player| {
        (player.deck.len() as int) / 10
    })
}

/* ---------------------------- Militia ---------------------------- */

pub static MILITIA: Card = &'static CardDef { name: "Militia", cost: 4, types: &[Action(do_militia)] };
fn do_militia(_: &[ActionInput]) {
    with_active_player(|player| player.buying_power += 2);
    attack(|other: &mut PlayerState| {
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
fn do_moneylender(_: &[ActionInput]) {
    with_active_player(|player| {
        if !player.hand_contains(COPPER) {
            return;
        }
        player.trash(COPPER);
        player.buying_power += 3;
    });
}

/* ---------------------------- Remodel ---------------------------- */

pub static REMODEL: Card = &'static CardDef { name: "Remodel", cost: 4, types: &[Action(do_remodel)] };
fn do_remodel(inputs: &[ActionInput]) {
    let to_trash = inputs.iter().find(|i| i.is_trash()).unwrap().unwrap();
    let to_gain = inputs.iter().find(|i| i.is_gain()).unwrap().unwrap();
    if to_gain.cost > to_trash.cost + 2 {
        return;
    }
    with_active_player(|player| {
        if !player.hand_contains(to_trash) {
            return;
        }
        player.trash(to_trash);
        player.gain(to_gain);
    });
}

/* ---------------------------- Smithy ---------------------------- */

pub static SMITHY: Card = &'static CardDef { name: "Smithy", cost: 4, types: &[Action(do_smithy)] };
fn do_smithy(_: &[ActionInput]) {
    with_active_player(|player| {
        for _ in range(0, 3) {
            player.draw();
        }
    });
}

/* ---------------------------- Spy ---------------------------- */

pub static SPY: Card = &'static CardDef { name: "Spy", cost: 4, types: &[Action(do_spy)] };
fn do_spy(_: &[ActionInput]) {
    with_active_player(|player| {
        player.draw();
        player.actions += 1;
    });
    attack(|other| {
        // TODO: get input from the player on where to put this
        other.mill();
    });
    // TODO: do the same thing for yourself
}

/* ---------------------------- Thief ---------------------------- */

pub static THIEF: Card = &'static CardDef { name: "Thief", cost: 4, types: &[Action(do_thief)] };
fn do_thief(_: &[ActionInput]) {
    let mut gained = Vec::new();
    attack(|other| {
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
    with_active_player(|player| {
        for c in gained.iter() {
            player.gain(*c);
        }
    });
}

/* ---------------------------- Throne Room ---------------------------- */

pub static THRONE_ROOM: Card = &'static CardDef { name: "Throne Room", cost: 4, types: &[Action(do_throne_room)] };
fn do_throne_room(inputs: &[ActionInput]) {
    let (c, f) = match *inputs.iter().find(|i| i.is_repeat()).unwrap() {
        super::Repeat(c, f) => (c, f),
        _ => fail!("Invalid Throne Room input!"),
    };
    if !c.is_action() {
        fail!("Can't play Throne Room on non-Action card!");
    }
    let action = c.get_action();
    for i in range(0u, 2u) {
        let input = f(i);
        action(input.as_slice());
    }
}

/* ---------------------------- Council Room ---------------------------- */

pub static COUNCIL_ROOM: Card = &'static CardDef { name: "Council Room", cost: 5, types: &[Action(do_council_room)] };
fn do_council_room(_: &[ActionInput]) {
    with_active_player(|player| {
        for _ in range(0, 4) {
            player.draw();
        }
        player.buys += 1;
    });
    with_other_players(|other| {
        other.draw();
    });
}

/* ---------------------------- Festival ---------------------------- */

pub static FESTIVAL: Card = &'static CardDef { name: "Festival", cost: 5, types: &[Action(do_festival)] };
fn do_festival(_: &[ActionInput]) {
    with_active_player(|player| {
        player.actions += 2;
        player.buys += 1;
        player.buying_power += 2;
    });
}

/* ---------------------------- Laboratory ---------------------------- */

pub static LABORATORY: Card = &'static CardDef { name: "Laboratory", cost: 5, types: &[Action(do_laboratory)] };
fn do_laboratory(_: &[ActionInput]) {
    with_active_player(|player| {
        for _ in range(0, 2) {
            player.draw();
        }
        player.actions += 1;
    });
}

/* ---------------------------- Library ---------------------------- */

pub static LIBRARY: Card = &'static CardDef { name: "Library", cost: 5, types: &[Action(do_library)] };
fn do_library(_: &[ActionInput]) {
    // TODO: fix this
    with_active_player(|player| {
        let mut set_aside = Vec::new();
        while player.hand.len() < 7 {
            match player.draw() {
                None => break,
                Some(drawn) => {
                    if drawn.is_action() /* && p.library_should_discard(drawn) */ {
                        player.remove_from_hand(drawn);
                        set_aside.push(drawn);
                    }
                }
            }
        }
        player.discard.push_all(set_aside.as_slice());
    });
}

/* ---------------------------- Market ---------------------------- */

pub static MARKET: Card = &'static CardDef { name: "Market", cost: 5, types: &[Action(do_market)] };
fn do_market(_: &[ActionInput]) {
    with_active_player(|player| {
        player.draw();
        player.actions += 1;
        player.buys += 1;
        player.buying_power += 1;
    });
}

/* ---------------------------- Mine ---------------------------- */

pub static MINE: Card = &'static CardDef { name: "Mine", cost: 5, types: &[Action(do_mine)] };
fn do_mine(inputs: &[ActionInput]) {
    let to_trash = inputs.iter().find(|x| x.is_trash()).unwrap().unwrap();
    let to_gain = inputs.iter().find(|x| x.is_gain()).unwrap().unwrap();
    if to_gain.treasure_value() > (to_trash.treasure_value() + 3) || !to_gain.is_money() {
        return;
    }
    with_active_player(|player| {
        if !player.hand.contains(&to_trash) || !to_trash.is_money() {
            return;
        }
        player.trash(to_trash);
        player.gain_to_hand(to_gain);
    });
}

/* ---------------------------- Witch ---------------------------- */

pub static WITCH: Card = &'static CardDef { name: "Witch", cost: 5, types: &[Action(do_witch)] };
fn do_witch(_: &[ActionInput]) {
    with_active_player(|player| {
        for _ in range(0, 2) {
            player.draw();
        }
    });
    attack(|other| {
        other.curse();
    });
}

/* ---------------------------- Adventurer ---------------------------- */

pub static ADVENTURER: Card = &'static CardDef { name: "Adventurer", cost: 6, types: &[Action(do_adventurer)] };
fn do_adventurer(_: &[ActionInput]) {
    with_active_player(|player| {
        let mut count = 0;
        while count < 2 {
            match player.next_card() {
                Some(c) => {
                    if c.is_money() {
                        count += 1;
                        player.hand.push(c);
                    } else {
                        player.discard.push(c);
                    }
                },
                None => break,
            }
        }
    });
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
