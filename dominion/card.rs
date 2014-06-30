
//! Card definitions.

use std::collections::HashSet;
use std::vec::Vec;
use super::{
    with_active_player, with_other_players, attack,
    Card, CardDef, PlayerState,
    Trash, Gain,
    Money, Victory, Action, Curse, ActionInput,
};

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
        let mut discarded = 0u;
        for card in inputs.iter().filter(|i| i.is_discard()).map(|i| i.get_card()) {
            player.discard(card).unwrap_or_else(|_| fail!("Cellar tried to discard {}, but you don't have it!", card.name));
            discarded += 1;
        }
        for _ in range(0, discarded) {
            player.draw();
        }
    });
}

/* ---------------------------- Chapel ---------------------------- */

pub static CHAPEL: Card = &'static CardDef { name: "Chapel", cost: 2, types: &[Action(do_chapel)] };
fn do_chapel(inputs: &[ActionInput]) {
    with_active_player(|player| {
        for card in inputs.iter().filter(|i| i.is_trash()).map(|i| i.get_card()).take(4) {
            player.trash(card).unwrap_or_else(|_| fail!("Chapel tried to trash {}, but you don't have it!", card.name));
        }
    });
}

/* ---------------------------- Moat ---------------------------- */

pub static MOAT: Card = &'static CardDef { name: "Moat", cost: 2, types: &[Action(do_moat)] };
fn do_moat(_: &[ActionInput]) {
    with_active_player(|player| {
        for _ in range(0u, 2u) {
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
        let card = match inputs.iter().find(|i| i.is_gain()) {
            Some(&Gain(card)) => card,
            _ => fail!("No card to gain provided for Workshop!"),
        };
        if card.cost > 4 {
            fail!("Workshop can't gain {} because {} > 4!", card.name, card.cost);
        }
        player.gain(card);
    });
}

/* ---------------------------- Bureaucrat ---------------------------- */

pub static BUREAUCRAT: Card = &'static CardDef { name: "Bureaucrat", cost: 4, types: &[Action(do_bureaucrat)] };
fn do_bureaucrat(_: &[ActionInput]) {
    with_active_player(|player| {
        player.gain_to_deck(SILVER);
    });
    attack(|other: &mut PlayerState| {
        let options: Vec<Card> = other.hand.iter().filter_map(|&c| if c.is_victory() { Some(c) } else { None }).collect();
        if options.len() > 0 {
            let c = other.myself.bureaucrat_use_victory(options.as_slice());
            if !options.contains(&c) {
                fail!("Bureaucrat tried to choose {}, which wasn't an available option!", c.name);
            }
            other.remove_from_hand(c);
            other.deck.unshift(c);
        }
    });
}

/* ---------------------------- Feast ---------------------------- */

pub static FEAST: Card = &'static CardDef { name: "Feast", cost: 4, types: &[Action(do_feast)] };
fn do_feast(inputs: &[ActionInput]) {
    with_active_player(|player| {
        player.trash_from_play(FEAST);
        let card = match inputs.iter().find(|i| i.is_gain()) {
            Some(&Gain(card)) => card,
            _ => fail!("No card to gain provided for Feast!"),
        };
        if card.cost > 5 {
            fail!("Feast can't gain {} because {} > 5!", card.name, card.cost);
        }
        player.gain(card);
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
        while other.hand.len() > 3 {
            let card = other.myself.militia_discard(other.hand.as_slice());
            other.discard(card).unwrap_or_else(|_| fail!("Militia tried to discard {}, but you don't have it!", card.name));
        }
    });
}

/* ---------------------------- Moneylender ---------------------------- */

pub static MONEYLENDER: Card = &'static CardDef { name: "Moneylender", cost: 4, types: &[Action(do_moneylender)] };
fn do_moneylender(_: &[ActionInput]) {
    with_active_player(|player| {
        if player.hand_contains(COPPER) {
            player.trash(COPPER);
            player.buying_power += 3;
        }
    });
}

/* ---------------------------- Remodel ---------------------------- */

pub static REMODEL: Card = &'static CardDef { name: "Remodel", cost: 4, types: &[Action(do_remodel)] };
fn do_remodel(inputs: &[ActionInput]) {
    let to_trash = match inputs.iter().find(|i| i.is_trash()) {
        Some(&Trash(card)) => card,
        _ => fail!("No card to trash provided for Remodel!"),
    };
    let to_gain = match inputs.iter().find(|i| i.is_gain()) {
        Some(&Gain(card)) => card,
        _ => fail!("No card to gain provided for Remodel!"),
    };
    if to_gain.cost > (to_trash.cost + 2) {
        fail!("Remodel can't trash a card costing {} and gain one costing {}!", to_trash.cost, to_gain.cost);
    }
    with_active_player(|player| {
        player.trash(to_trash).unwrap_or_else(|_| fail!("Remodel tried to trash {}, but you don't have it!", to_trash.name));
        player.gain(to_gain).unwrap_or_else(|_| fail!("Remodel tried to gain {}, but it's not available!", to_gain.name));
    });
}

/* ---------------------------- Smithy ---------------------------- */

pub static SMITHY: Card = &'static CardDef { name: "Smithy", cost: 4, types: &[Action(do_smithy)] };
fn do_smithy(_: &[ActionInput]) {
    with_active_player(|player| {
        for _ in range(0u, 3u) {
            player.draw();
        }
    });
}

/* ---------------------------- Spy ---------------------------- */

pub static SPY: Card = &'static CardDef { name: "Spy", cost: 4, types: &[Action(do_spy)] };
fn do_spy(_: &[ActionInput]) {
    attack(|other| {
        other.next_card().map(|card| {
            if other.myself.spy_should_discard(card, false) {
                other.discard.push(card);
            } else {
                other.deck.unshift(card);
            }
        });
    });
    with_active_player(|player| {
        player.draw();
        player.actions += 1;
        player.next_card().map(|card| {
            if player.myself.spy_should_discard(card, true) {
                player.discard.push(card);
            } else {
                player.deck.unshift(card);
            }
        });
    });
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
        let (chosen, keep) = other.myself.thief_trash_and_keep(money.as_slice());
        match money.iter().position(|m| *m == chosen) {
            Some(i) => { money.remove(i); },
            None => fail!("Thief tried to trash {}, but it wasn't a valid option!", chosen.name),
        }
        other.trash(chosen);
        if keep {
            gained.push(chosen);
        }
        for rest in money.iter() {
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
        for _ in range(0u, 4u) {
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
        for _ in range(0u, 2u) {
            player.draw();
        }
        player.actions += 1;
    });
}

/* ---------------------------- Library ---------------------------- */

pub static LIBRARY: Card = &'static CardDef { name: "Library", cost: 5, types: &[Action(do_library)] };
fn do_library(_: &[ActionInput]) {
    with_active_player(|player| {
        let mut set_aside = Vec::new();
        while player.hand.len() < 7 {
            match player.draw() {
                None => break,
                Some(drawn) => {
                    if drawn.is_action() && player.myself.library_should_discard(drawn) {
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
    let to_trash = inputs.iter().find(|x| x.is_trash()).unwrap().get_card();
    let to_gain = inputs.iter().find(|x| x.is_gain()).unwrap().get_card();
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
        for _ in range(0u, 2u) {
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
        let mut count = 0u;
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


/* ---------------------------- Dominion Set ---------------------------- */

pub fn dominion_set() -> HashSet<&'static str> {
    let mut cards = HashSet::with_capacity(25);
    cards.insert(CELLAR.name);
    cards.insert(CHAPEL.name);
    cards.insert(MOAT.name);
    cards.insert(CHANCELLOR.name);
    cards.insert(VILLAGE.name);
    cards.insert(WOODCUTTER.name);
    cards.insert(WORKSHOP.name);
    cards.insert(BUREAUCRAT.name);
    cards.insert(FEAST.name);
    cards.insert(GARDENS.name);
    cards.insert(MILITIA.name);
    cards.insert(MONEYLENDER.name);
    cards.insert(REMODEL.name);
    cards.insert(SMITHY.name);
    cards.insert(SPY.name);
    cards.insert(THIEF.name);
    cards.insert(THRONE_ROOM.name);
    cards.insert(COUNCIL_ROOM.name);
    cards.insert(FESTIVAL.name);
    cards.insert(LABORATORY.name);
    cards.insert(LIBRARY.name);
    cards.insert(MARKET.name);
    cards.insert(MINE.name);
    cards.insert(WITCH.name);
    cards.insert(ADVENTURER.name);
    cards
}

// This is a hack needed until Rust can properly hash function pointers.
pub fn for_name(name: &'static str) -> Card {
    match name {
        "Cellar" => CELLAR,
        "Chapel" => CHAPEL,
        "Moat" => MOAT,
        "Chancellor" => CHANCELLOR,
        "Village" => VILLAGE,
        "Woodcutter" => WOODCUTTER,
        "Workshop" => WORKSHOP,
        "Bureaucrat" => BUREAUCRAT,
        "Feast" => FEAST,
        "Gardens" => GARDENS,
        "Militia" => MILITIA,
        "Moneylender" => MONEYLENDER,
        "Remodel" => REMODEL,
        "Smithy" => SMITHY,
        "Spy" => SPY,
        "Thief" => THIEF,
        "Throne Room" => THRONE_ROOM,
        "Council Room" => COUNCIL_ROOM,
        "Festival" => FESTIVAL,
        "Laboratory" => LABORATORY,
        "Library" => LIBRARY,
        "Market" => MARKET,
        "Mine" => MINE,
        "Witch" => WITCH,
        "Adventurer" => ADVENTURER,
        _ => fail!("Unrecognized card name: {}", name),
    }
}


/* ---------------------------- Testing ---------------------------- */


#[cfg(test)]
mod tests {
    extern crate sync;

    use super::super::card::*;
    use error = super::super::error;

    use std::collections::{DList, HashMap};
    use super::super::{Card, Player, PlayerState, Supply, Discard, Trash, GameState};
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::vec::Vec;
    use sync::Arc;

    macro_rules! assert_no_error(
        ($val:expr) => (
            match $val {
                Ok(_) => (),
                Err(e) => match e {
                    error::InvalidPlay => fail!("Invalid play!"),
                    error::NoActions => fail!("No actions left!"),
                    _ => fail!("Unknown error!"),
                },
            }
        )
    )


    struct Alice;
    impl Player for Alice {
        fn name(&self) -> &'static str { "Alice" }
        fn take_turn(&self) {}
    }

    fn setup(hand: Vec<Card>, deck: Vec<Card>) {
        let trash = Vec::new();

        let mut supply: Supply = HashMap::new();
        supply.insert(COPPER.to_str(),   30);
        supply.insert(SILVER.to_str(),   30);
        supply.insert(GOLD.to_str(),     30);
        supply.insert(ESTATE.to_str(),   12);
        supply.insert(DUCHY.to_str(),    12);
        supply.insert(PROVINCE.to_str(), 12);
        supply.insert(CURSE.to_str(),    30);
        supply.insert(SMITHY.to_str(),   10);
        supply.insert(WITCH.to_str(),    10);

        let game = GameState{supply: supply, trash: trash};

        // TODO: create a second player Bob for testing attack cards
        let alice = box Alice as Box<Player,Send+Share>;
        ::active_player.replace(Some(alice.name()));

        let mut player_state_map = HashMap::<&'static str, PlayerState>::new();

        player_state_map.insert(alice.name(), PlayerState{
            game_ref:      Rc::new(RefCell::new(game)),
            myself:        Arc::new(alice),
            other_players: DList::new(),
            deck:          deck,
            discard:       Vec::new(),
            in_play:       Vec::new(),
            hand:          hand,
            actions:       1,
            buys:          1,
            buying_power:  0,
            score:         0,
        });

        ::state_map.replace(Some(RefCell::new(player_state_map)));
    }

    #[test]
    fn test_cellar() {
        setup(vec!(CELLAR, ESTATE, ESTATE, COPPER), vec!(SILVER, GOLD));
        assert_no_error!(::play_card_and(CELLAR, vec!(Discard(ESTATE), Discard(ESTATE)).as_slice()));
        let hand = ::get_hand();
        assert_eq!(hand.len(), 3);
        assert_eq!(*hand.get(0), COPPER);
        assert_eq!(*hand.get(1), SILVER);
        assert_eq!(*hand.get(2), GOLD);
        assert_eq!(::get_action_count(), 1);
    }

    #[test]
    fn test_chapel() {
        setup(vec!(CHAPEL, ESTATE, ESTATE, COPPER, ESTATE, COPPER), vec!());
        assert_eq!(::get_trash().len(), 0);
        assert_no_error!(::play_card_and(CHAPEL, vec!(Trash(ESTATE), Trash(ESTATE), Trash(ESTATE), Trash(COPPER)).as_slice()));
        let hand = ::get_hand();
        let trash = ::get_trash();
        assert_eq!(hand.len(), 1);
        assert_eq!(*hand.get(0), COPPER);
        assert_eq!(trash.len(), 4);
        assert_eq!(trash.iter().filter(|&x| x == &COPPER).count(), 1);
        assert_eq!(trash.iter().filter(|&x| x == &ESTATE).count(), 3);
    }


    // #[test]
    // fn test_moat() {
    //     ...
    // }

    #[test]
    fn test_chancellor() {
        // TODO: test the deck-to-discard piece
        setup(vec!(CHANCELLOR), vec!());
        assert_no_error!(::play_card(CHANCELLOR));
        assert_eq!(::get_buying_power(), 2);
    }
}
