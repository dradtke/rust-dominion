//! Universal card definitions.

use super::{Card, CardDef, Money, Victory, Curse};

pub mod dominion;

pub static COPPER: Card = &CardDef { name: "Copper", cost: 0, types: [Money(1)] };
pub static SILVER: Card = &CardDef { name: "Silver", cost: 3, types: [Money(2)] };
pub static GOLD:   Card = &CardDef { name: "Gold", cost: 6, types: [Money(3)] };

pub static ESTATE: Card = &CardDef { name: "Estate", cost: 2, types: [Victory(get_estate_value)] };
fn get_estate_value() -> int { 1 }

pub static DUCHY: Card = &CardDef { name: "Duchy", cost: 5, types: [Victory(get_duchy_value)] };
fn get_duchy_value() -> int { 3 }

pub static PROVINCE: Card = &CardDef { name: "Province", cost: 8, types: [Victory(get_province_value)] };
fn get_province_value() -> int { 6 }

pub static CURSE: Card = &CardDef { name: "Curse", cost: 0, types: [Curse(-1)] };


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
    use super::super::{Card, GameState, InvalidPlay, NoActions, Player, PlayerList, PlayerState, Result, Supply};

    use std::collections::{Deque, HashMap};
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::vec::Vec;
    use sync::Arc;

    pub struct Ai {
        pub hand: Vec<Card>,
        pub deck: Vec<Card>,
    }

    struct Alice;
    impl Player for Alice {
        fn name(&self) -> &'static str { "Alice" }
        fn init(&self, _: &[Card]) -> fn() { take_turn }
    }

    struct Bob;
    impl Player for Bob {
        fn name(&self) -> &'static str { "Bob" }
        fn init(&self, _: &[Card]) -> fn() { take_turn }
    }

    struct Charlie;
    impl Player for Charlie {
        fn name(&self) -> &'static str { "Charlie" }
        fn init(&self, _: &[Card]) -> fn() { take_turn }
    }

    struct Delta;
    impl Player for Delta {
        fn name(&self) -> &'static str { "Delta" }
        fn init(&self, _: &[Card]) -> fn() { take_turn }
    }

    fn take_turn() {}

    pub fn setup(ais: Vec<Ai>) {
        let trash = Vec::new();

        let mut supply: Supply = HashMap::new();
        supply.insert(COPPER.name.to_string(),   30);
        supply.insert(SILVER.name.to_string(),   30);
        supply.insert(GOLD.name.to_string(),     30);
        supply.insert(ESTATE.name.to_string(),   12);
        supply.insert(DUCHY.name.to_string(),    12);
        supply.insert(PROVINCE.name.to_string(), 12);
        supply.insert(CURSE.name.to_string(),    30);

        let game = GameState{supply: supply, trash: trash};
        let game_ref = Rc::new(RefCell::new(game));

        let ai_arcs = ais.iter().enumerate().map(|(index, _)| match index {
            0 => Arc::new(box Alice as Box<Player + Send + Share>),
            1 => Arc::new(box Bob as Box<Player + Send + Share>),
            2 => Arc::new(box Charlie as Box<Player + Send + Share>),
            3 => Arc::new(box Delta as Box<Player + Send + Share>),
            _ => fail!("Unsupported number of players!"),
        }).collect::<Vec<Arc<Box<Player + Send + Share>>>>();

        let mut player_state_map = HashMap::<&'static str, PlayerState>::new();
        ::local_active_player.replace(Some(ai_arcs.get(0).name()));

        let other_players = ai_arcs.clone().move_iter().collect::<PlayerList>();

        for (index, ai) in ai_arcs.move_iter().enumerate() {
            let mut other_players = other_players.clone();
            while other_players.front().unwrap().name() != ai.name() {
                other_players.rotate_backward();
            }
            other_players.pop_front();
            let stub = ais.get(index);
            player_state_map.insert(ai.name(), PlayerState{
                game_ref:      game_ref.clone(),
                myself:        ai.clone(),
                other_players: other_players,
                deck:          stub.deck.clone(),
                discard:       vec![],
                in_play:       vec![],
                hand:          stub.hand.clone(),
                actions:       1,
                buys:          1,
                buying_power:  0,
            });
        }

        ::local_state_map.replace(Some(RefCell::new(player_state_map)));
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
