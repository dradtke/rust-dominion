//! Dominion card definitions.

use std::collections::HashSet;
use std::vec::Vec;
use super::super::{Card, ActionParameter, Result};

macro_rules! try_action(
    ($action:expr) => ({
        let res = $action;
        if res.is_err() {
            return res;
        }
    })
)

macro_rules! first(
    ($iter:expr else $err:expr) => (
        match $iter.next() {
            Some(x) => x,
            None => return Err($err),
        }
    )
)

/* ---------------------------- Cellar ---------------------------- */

pub static CELLAR: Card = &::CardDef { name: "Cellar", cost: 2, types: [::Action(do_cellar)] };
fn do_cellar(params: &[ActionParameter]) -> Result {
    ::with_active_player(|player| {
        let mut hand = player.hand.clone();
        let mut discarded = ActionParameter::get_discarded(params);
        for card in discarded {
            match hand.iter().position(|&x| x == card) {
                Some(i) => { hand.remove(i); },
                None => return Err(::NotInHand(card)),
            }
        }
        player.hand = hand;
        for _ in range(0, discarded.count()) {
            player.draw();
        }
        player.actions += 1;
        Ok(())
    })
}

/* ---------------------------- Chapel ---------------------------- */

pub static CHAPEL: Card = &::CardDef { name: "Chapel", cost: 2, types: [::Action(do_chapel)] };
fn do_chapel(params: &[ActionParameter]) -> Result {
    ::with_active_player(|player| {
        let mut hand = player.hand.clone();
        for card in ActionParameter::get_trashed(params).take(4) {
            match hand.iter().position(|&x| x == card) {
                Some(i) => { hand.remove(i); },
                None => return Err(::NotInHand(card)),
            }
        }
        player.hand = hand;
        Ok(())
    })
}

/* ---------------------------- Moat ---------------------------- */

pub static MOAT: Card = &::CardDef { name: "Moat", cost: 2, types: [::Action(do_moat)] };
fn do_moat(_: &[ActionParameter]) -> Result {
    ::with_active_player(|player| {
        for _ in range(0u, 2u) {
            player.draw();
        }
        Ok(())
    })
}

/* ---------------------------- Chancellor ---------------------------- */

pub static CHANCELLOR: Card = &::CardDef { name: "Chancellor", cost: 3, types: [::Action(do_chancellor)] };
fn do_chancellor(params: &[ActionParameter]) -> Result {
    ::with_active_player(|player| {
        player.buying_power += 2;
        if ActionParameter::get_confirmed(params) {
            player.discard_deck();
        }
        Ok(())
    })
}

/* ---------------------------- Village ---------------------------- */

pub static VILLAGE: Card = &::CardDef { name: "Village", cost: 3, types: &[::Action(do_village)] };
fn do_village(_: &[ActionParameter]) -> Result {
    ::with_active_player(|player| {
        player.draw();
        player.actions += 2;
        Ok(())
    })
}

/* ---------------------------- Woodcutter ---------------------------- */

pub static WOODCUTTER: Card = &::CardDef { name: "Woodcutter", cost: 3, types: &[::Action(do_woodcutter)] };
fn do_woodcutter(_: &[ActionParameter]) -> Result {
    ::with_active_player(|player| {
        player.buys += 1;
        player.buying_power += 2;
        Ok(())
    })
}

/* ---------------------------- Workshop ---------------------------- */

pub static WORKSHOP: Card = &::CardDef { name: "Workshop", cost: 3, types: &[::Action(do_workshop)] };
fn do_workshop(params: &[ActionParameter]) -> Result {
    ::with_active_player(|player| {
        let card = match ActionParameter::get_gained(params).next() {
            Some(card) => card,
            None => return Err(::NothingToGain),
        };
        if card.cost > 4 {
            return Err(::NotEnoughMoney{ need: card.cost, have: 4 });
        }
        player.gain(card);
        Ok(())
    })
}

/* ---------------------------- Bureaucrat ---------------------------- */

pub static BUREAUCRAT: Card = &::CardDef { name: "Bureaucrat", cost: 4, types: &[::Action(do_bureaucrat)] };
fn do_bureaucrat(_: &[ActionParameter]) -> Result {
    ::with_active_player(|player| {
        player.gain_to_deck(super::SILVER);
    });
    ::attack(|other: &mut ::PlayerState| {
        let options: Vec<Card> = other.hand.iter().filter_map(|&c| if c.is_victory() { Some(c) } else { None }).collect();
        if options.len() > 0 {
            let c = other.myself.bureaucrat_use_victory(options.as_slice());
            if !options.contains(&c) {
                return Err(::InvalidChoice(c));
            }
            other.remove_from_hand(c);
            other.deck.insert(0, c);
        }
        Ok(())
    })
}

/* ---------------------------- Feast ---------------------------- */

pub static FEAST: Card = &::CardDef { name: "Feast", cost: 4, types: &[::Action(do_feast)] };
fn do_feast(params: &[ActionParameter]) -> Result {
    ::with_active_player(|player| {
        player.trash_from_play(FEAST);
        let card = match ActionParameter::get_gained(params).next() {
            Some(card) => card,
            None => return Err(::NothingToGain),
        };
        if card.cost > 5 {
            return Err(::NotEnoughMoney{ need: card.cost, have: 5 });
        }
        player.gain(card);
        Ok(())
    })
}

/* ---------------------------- Gardens ---------------------------- */

pub static GARDENS: Card = &::CardDef { name: "Gardens", cost: 4, types: &[::Victory(get_gardens_value)] };
fn get_gardens_value() -> int {
    ::with_active_player(|player| {
        (player.deck.len() as int) / 10
    })
}

/* ---------------------------- Militia ---------------------------- */

pub static MILITIA: Card = &::CardDef { name: "Militia", cost: 4, types: &[::Action(do_militia)] };
fn do_militia(_: &[ActionParameter]) -> Result {
    ::with_active_player(|player| player.buying_power += 2);
    ::attack(|other: &mut ::PlayerState| {
        while other.hand.len() > 3 {
            let card = other.myself.militia_discard(other.hand.as_slice());
            try_action!(other.discard(card));
        }
        Ok(())
    })
}

/* ---------------------------- Moneylender ---------------------------- */

pub static MONEYLENDER: Card = &::CardDef { name: "Moneylender", cost: 4, types: &[::Action(do_moneylender)] };
fn do_moneylender(_: &[ActionParameter]) -> Result {
    ::with_active_player(|player| {
        if player.hand_contains(super::COPPER) {
            player.trash(super::COPPER);
            player.buying_power += 3;
        }
        Ok(())
    })
}

/* ---------------------------- Remodel ---------------------------- */

pub static REMODEL: Card = &::CardDef { name: "Remodel", cost: 4, types: &[::Action(do_remodel)] };
fn do_remodel(params: &[ActionParameter]) -> Result {
    let to_trash = first!(ActionParameter::get_trashed(params) else ::NothingToTrash);
    let to_gain = first!(ActionParameter::get_gained(params) else ::NothingToGain);
    let have = to_trash.cost + 2;
    if to_gain.cost > have {
        return Err(::NotEnoughMoney{ need: to_gain.cost, have: have });
    }
    ::with_active_player(|player| {
        try_action!(player.trash(to_trash));
        try_action!(player.gain(to_gain));
        Ok(())
    })
}

/* ---------------------------- Smithy ---------------------------- */

pub static SMITHY: Card = &::CardDef { name: "Smithy", cost: 4, types: &[::Action(do_smithy)] };
fn do_smithy(_: &[ActionParameter]) -> Result {
    ::with_active_player(|player| {
        for _ in range(0u, 3u) {
            player.draw();
        }
        Ok(())
    })
}

/* ---------------------------- Spy ---------------------------- */

pub static SPY: Card = &::CardDef { name: "Spy", cost: 4, types: &[::Action(do_spy)] };
fn do_spy(_: &[ActionParameter]) -> Result {
    ::attack(|other| {
        other.next_card().map(|card| {
            if other.myself.spy_should_discard(card, false) {
                other.discard.push(card);
            } else {
                other.deck.insert(0, card);
            }
        });
        Ok(())
    });
    ::with_active_player(|player| {
        player.draw();
        player.actions += 1;
        player.next_card().map(|card| {
            if player.myself.spy_should_discard(card, true) {
                player.discard.push(card);
            } else {
                player.deck.insert(0, card);
            }
        });
    });
    Ok(())
}

/* ---------------------------- Thief ---------------------------- */

pub static THIEF: Card = &::CardDef { name: "Thief", cost: 4, types: &[::Action(do_thief)] };
fn do_thief(_: &[ActionParameter]) -> Result {
    let mut gained = Vec::new();
    try_action!(::attack(|other| {
        let (mut money, non_money) = other.next_n_cards(2).partition(|c| c.is_money());
        for c in non_money.iter() {
            other.discard(*c);
        }
        if money.is_empty() {
            return Ok(());
        }
        let (chosen, keep) = other.myself.thief_trash_and_keep(money.as_slice());
        match money.iter().position(|m| *m == chosen) {
            Some(i) => { money.remove(i); },
            None => return Err(::InvalidChoice(chosen)),
        }
        other.trash(chosen);
        if keep {
            gained.push(chosen);
        }
        for rest in money.iter() {
            other.discard(*rest);
        }
        Ok(())
    }));
    ::with_active_player(|player| {
        for c in gained.iter() {
            player.gain(*c);
        }
    });
    Ok(())
}

/* ---------------------------- Throne Room ---------------------------- */

pub static THRONE_ROOM: Card = &::CardDef { name: "Throne Room", cost: 4, types: &[::Action(do_throne_room)] };
fn do_throne_room(params: &[ActionParameter]) -> Result {
    let (card, f) = first!(ActionParameter::get_repeated(params) else ::NothingToRepeat);
    if !card.is_action() {
        return Err(::InvalidPlay(card));
    }
    let action = card.get_action();
    for i in range(0u, 2u) {
        action(f(i).as_slice());
    }
    Ok(())
}

/* ---------------------------- Council Room ---------------------------- */

pub static COUNCIL_ROOM: Card = &::CardDef { name: "Council Room", cost: 5, types: &[::Action(do_council_room)] };
fn do_council_room(_: &[ActionParameter]) -> Result {
    ::with_active_player(|player| {
        for _ in range(0u, 4u) {
            player.draw();
        }
        player.buys += 1;
    });
    ::with_other_players(|other| {
        other.draw();
        Ok(())
    });
    Ok(())
}

/* ---------------------------- Festival ---------------------------- */

pub static FESTIVAL: Card = &::CardDef { name: "Festival", cost: 5, types: &[::Action(do_festival)] };
fn do_festival(_: &[ActionParameter]) -> Result {
    ::with_active_player(|player| {
        player.actions += 2;
        player.buys += 1;
        player.buying_power += 2;
        Ok(())
    })
}

/* ---------------------------- Laboratory ---------------------------- */

pub static LABORATORY: Card = &::CardDef { name: "Laboratory", cost: 5, types: &[::Action(do_laboratory)] };
fn do_laboratory(_: &[ActionParameter]) -> Result {
    ::with_active_player(|player| {
        for _ in range(0u, 2u) {
            player.draw();
        }
        player.actions += 1;
        Ok(())
    })
}

/* ---------------------------- Library ---------------------------- */

pub static LIBRARY: Card = &::CardDef { name: "Library", cost: 5, types: &[::Action(do_library)] };
fn do_library(_: &[ActionParameter]) -> Result {
    ::with_active_player(|player| {
        let mut set_aside = Vec::new();
        while player.hand.len() < 7 {
            match player.draw() {
                Some(drawn) => {
                    if drawn.is_action() && player.myself.library_should_discard(drawn) {
                        player.remove_from_hand(drawn);
                        set_aside.push(drawn);
                    }
                },
                None => break,
            }
        }
        player.discard.push_all(set_aside.as_slice());
        Ok(())
    })
}

/* ---------------------------- Market ---------------------------- */

pub static MARKET: Card = &::CardDef { name: "Market", cost: 5, types: &[::Action(do_market)] };
fn do_market(_: &[ActionParameter]) -> Result {
    ::with_active_player(|player| {
        player.draw();
        player.actions += 1;
        player.buys += 1;
        player.buying_power += 1;
        Ok(())
    })
}

/* ---------------------------- Mine ---------------------------- */

pub static MINE: Card = &::CardDef { name: "Mine", cost: 5, types: &[::Action(do_mine)] };
fn do_mine(params: &[ActionParameter]) -> Result {
    let to_trash = first!(ActionParameter::get_trashed(params) else ::NothingToTrash);
    let to_gain = first!(ActionParameter::get_gained(params) else ::NothingToGain);
    if to_gain.treasure_value() > (to_trash.treasure_value() + 3) || !to_gain.is_money() {
        return Err(::InvalidChoice(to_gain));
    }
    ::with_active_player(|player| {
        if !player.hand.contains(&to_trash) || !to_trash.is_money() {
            return Ok(()); // error?
        }
        player.trash(to_trash);
        player.gain_to_hand(to_gain);
        Ok(())
    })
}

/* ---------------------------- Witch ---------------------------- */

pub static WITCH: Card = &::CardDef { name: "Witch", cost: 5, types: &[::Action(do_witch)] };
fn do_witch(_: &[ActionParameter]) -> Result {
    ::with_active_player(|player| {
        for _ in range(0u, 2u) {
            player.draw();
        }
    });
    ::attack(|other| {
        other.curse();
        Ok(())
    });
    Ok(())
}

/* ---------------------------- Adventurer ---------------------------- */

pub static ADVENTURER: Card = &::CardDef { name: "Adventurer", cost: 6, types: &[::Action(do_adventurer)] };
fn do_adventurer(_: &[ActionParameter]) -> Result {
    ::with_active_player(|player| {
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
        Ok(())
    })
}


/* ---------------------------- Dominion Set ---------------------------- */

pub fn set() -> HashSet<&'static str> {
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


/* ---------------------------- Testing ---------------------------- */

#[cfg(test)]
#[path = "tests/dominion.rs"]
mod tests;
