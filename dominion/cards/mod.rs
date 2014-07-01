//! Universal card definitions.

use super::{Card, CardDef, Money, Victory, Curse};

pub mod dominion;

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


/// This is a hack needed until Rust can properly hash function pointers.
pub fn for_name(name: &'static str) -> Card {
    match name {
        "Cellar"       => dominion::CELLAR,
        "Chapel"       => dominion::CHAPEL,
        "Moat"         => dominion::MOAT,
        "Chancellor"   => dominion::CHANCELLOR,
        "Village"      => dominion::VILLAGE,
        "Woodcutter"   => dominion::WOODCUTTER,
        "Workshop"     => dominion::WORKSHOP,
        "Bureaucrat"   => dominion::BUREAUCRAT,
        "Feast"        => dominion::FEAST,
        "Gardens"      => dominion::GARDENS,
        "Militia"      => dominion::MILITIA,
        "Moneylender"  => dominion::MONEYLENDER,
        "Remodel"      => dominion::REMODEL,
        "Smithy"       => dominion::SMITHY,
        "Spy"          => dominion::SPY,
        "Thief"        => dominion::THIEF,
        "Throne Room"  => dominion::THRONE_ROOM,
        "Council Room" => dominion::COUNCIL_ROOM,
        "Festival"     => dominion::FESTIVAL,
        "Laboratory"   => dominion::LABORATORY,
        "Library"      => dominion::LIBRARY,
        "Market"       => dominion::MARKET,
        "Mine"         => dominion::MINE,
        "Witch"        => dominion::WITCH,
        "Adventurer"   => dominion::ADVENTURER,
        _ => fail!("Unrecognized card name: {}", name),
    }
}

#[cfg(test)]
mod test {
    extern crate sync;

    use super::{COPPER, SILVER, GOLD, ESTATE, DUCHY, PROVINCE, CURSE};
    use super::super::{Card, GameState, InvalidPlay, NoActions, Player, PlayerState, Result, Supply};

    use std::collections::{DList, HashMap};
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::vec::Vec;
    use sync::Arc;

    struct Alice;
    impl Player for Alice {
        fn name(&self) -> &'static str { "Alice" }
        fn take_turn(&self) {}
    }

    pub fn setup(hand: Vec<Card>, deck: Vec<Card>) {
        let trash = Vec::new();

        let mut supply: Supply = HashMap::new();
        supply.insert(COPPER.to_str(),   30);
        supply.insert(SILVER.to_str(),   30);
        supply.insert(GOLD.to_str(),     30);
        supply.insert(ESTATE.to_str(),   12);
        supply.insert(DUCHY.to_str(),    12);
        supply.insert(PROVINCE.to_str(), 12);
        supply.insert(CURSE.to_str(),    30);

        let game = GameState{supply: supply, trash: trash};

        // TODO: create a second player Bob for testing attack cards
        let alice = box Alice as Box<Player + Send + Share>;
        ::ACTIVE_PLAYER.replace(Some(alice.name()));

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
        });

        ::STATE_MAP.replace(Some(RefCell::new(player_state_map)));
    }

    pub fn assert_ok(r: Result) {
        match r {
            Ok(_)  => (),
            Err(e) => match e {
                    InvalidPlay(_) => fail!("Invalid play!"),
                    NoActions => fail!("No actions left!"),
                    _ => fail!("Unknown error!"),
            },
        }
    }
}
