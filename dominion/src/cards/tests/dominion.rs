use super::super::test::{Ai, assert_ok, setup, set_active};
use super::*; // dominion set cards
use super::super::*; // base cards
use super::super::super::*; // functions

#[test]
fn test_cellar() {
    setup(vec![
          Ai{ hand: vec![CELLAR, ESTATE, ESTATE, COPPER], deck: vec![SILVER, GOLD] },
    ]);
    assert_ok(play_card_and(super::CELLAR, vec![Discard(ESTATE), Discard(ESTATE)].as_slice()));
    let hand = get_hand();
    assert_eq!(hand.len(), 3);
    assert_eq!(hand[0], COPPER);
    assert_eq!(hand[1], SILVER);
    assert_eq!(hand[2], GOLD);
    assert_eq!(get_action_count(), 1);
}

#[test]
fn test_chapel() {
    setup(vec![
          Ai{ hand: vec![CHAPEL, ESTATE, ESTATE, COPPER, ESTATE, COPPER], deck: vec![] },
    ]);
    assert!(get_trash().is_empty());
    assert_ok(play_card_and(CHAPEL, vec![Trash(ESTATE), Trash(ESTATE), Trash(ESTATE), Trash(COPPER)].as_slice()));
    let hand = get_hand();
    let trash = get_trash();
    assert_eq!(hand.len(), 1);
    assert_eq!(hand[0], COPPER);
    assert_eq!(trash.len(), 4);
    assert_eq!(trash.iter().filter(|&x| x == &COPPER).count(), 1);
    assert_eq!(trash.iter().filter(|&x| x == &ESTATE).count(), 3);
}

#[test]
fn test_chancellor() {
    // Don't discard the deck.
    setup(vec![
          Ai{ hand: vec![CHANCELLOR], deck: vec![COPPER, COPPER] },
    ]);
    assert_ok(play_card(CHANCELLOR));
    assert_eq!(get_buying_power(), 2);
    assert!(get_discard().is_empty());

    // Discard the deck.
    setup(vec![
          Ai{ hand: vec![CHANCELLOR], deck: vec![COPPER, COPPER] },
    ]);
    assert_ok(play_card_and(CHANCELLOR, vec![Confirm].as_slice()));
    assert_eq!(get_buying_power(), 2);
    assert_eq!(get_discard().len(), 2);
}

#[test]
fn test_moat() {
    setup(vec![
          Ai{ hand: vec![MILITIA], deck: vec![] },
          Ai{ hand: vec![MOAT, COPPER, COPPER, COPPER, COPPER], deck: vec![] },
    ]);
    assert_ok(play_card(MILITIA));
    set_active(1);
    assert_eq!(get_hand().len(), 5);
}

#[test]
fn test_militia() {
    setup(vec![
          Ai{ hand: vec![MILITIA], deck: vec![] },
          Ai{ hand: vec![COPPER, COPPER, COPPER, COPPER, COPPER], deck: vec![] },
    ]);
    assert_ok(play_card(MILITIA));
    set_active(1);
    assert_eq!(get_hand().len(), 3);
}
