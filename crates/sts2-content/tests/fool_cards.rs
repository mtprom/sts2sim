//! Scenario rules from v0.113.1's Fool and tarot card bodies.
use sts2_engine::{Action, CardFingerprint, Creature, PileName, ScenarioBuilder, Simulator};

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().expect("static model ID")
}

fn fight(model: &str, upgrade: u8, held: usize, draw: usize, hp: i32) -> Simulator {
    scenario(model, upgrade, held, draw, hp)
        .build(sts2_content::registry_for(&id("CHARACTER.FOOL")))
        .expect("scenario builds")
}

fn scenario(model: &str, upgrade: u8, held: usize, draw: usize, hp: i32) -> ScenarioBuilder {
    let mut source = CardFingerprint::base(id(model));
    source.upgrade_level = upgrade;
    let mut scenario = ScenarioBuilder::new("FOOLCARDS", id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .played_as(id("CHARACTER.FOOL"))
        .player_hp(77, 77)
        .energy(9, 9)
        .turn(1, 1)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), hp, hp, "SHRINKER_MOVE")
        .enemy(2, id("MONSTER.SHRINKER_BEETLE"), 100, 100, "SHRINKER_MOVE")
        .card(PileName::Hand, 0, source, 0);
    for index in 0..held + draw {
        scenario = scenario.card(
            if index < held {
                PileName::Hand
            } else {
                PileName::Draw
            },
            u64::try_from(index + 1).expect("small index"),
            CardFingerprint::base(id("CARD.DEFEND_FOOL")),
            0,
        );
    }
    scenario
}

fn play(sim: &mut Simulator, model: &str) {
    let action = sim
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(
                action, Action::PlayCard { card, .. } if card.fingerprint.model_id == id(model)
            )
        })
        .cloned()
        .expect("card is playable");
    sim.step(action).expect("play succeeds");
}

fn creature(sim: &Simulator, slot: u64) -> &Creature {
    sim.state()
        .combat
        .as_ref()
        .expect("combat")
        .creatures
        .iter()
        .find(|creature| creature.id.get() == slot)
        .expect("creature")
}

fn pile(sim: &Simulator, pile: PileName) -> Vec<String> {
    let piles = &sim.state().combat.as_ref().expect("combat").player.piles;
    match pile {
        PileName::Hand => &piles.hand,
        PileName::Draw => &piles.draw,
        PileName::Discard => &piles.discard,
        PileName::Exhaust => &piles.exhaust,
        PileName::Play => &piles.play,
    }
    .iter()
    .map(|card| card.fingerprint.model_id.to_string())
    .collect()
}

#[test]
fn basics_and_joy_have_their_upgraded_values() {
    for (upgrade, damage, block, gain) in [(0, 6, 5, 1), (1, 9, 8, 2)] {
        let mut sim = fight("CARD.STRIKE_FOOL", upgrade, 0, 0, 100);
        play(&mut sim, "CARD.STRIKE_FOOL");
        assert_eq!(creature(&sim, 1).current_hp, 100 - damage);
        let mut sim = fight("CARD.DEFEND_FOOL", upgrade, 0, 0, 100);
        play(&mut sim, "CARD.DEFEND_FOOL");
        assert_eq!(creature(&sim, 0).block, block);
        let mut sim = fight("CARD.JOY", upgrade, 0, 1, 100);
        play(&mut sim, "CARD.JOY");
        assert_eq!(
            sim.state().combat.as_ref().expect("combat").player.energy,
            9 + gain
        );
        assert_eq!(pile(&sim, PileName::Hand), ["CARD.DEFEND_FOOL"]);
        assert_eq!(pile(&sim, PileName::Exhaust), ["CARD.JOY"]);
    }
}

#[test]
fn sadness_is_retained_unplayable_and_not_upgradeable() {
    let registry = sts2_content::standard_registry();
    let definition = registry
        .card(&id("CARD.SADNESS"))
        .expect("sadness registered");
    assert!(definition.retain && definition.unplayable);
    assert!(definition.upgrade.is_none());
    let sim = fight("CARD.SADNESS", 0, 0, 0, 100);
    assert!(
        !sim.legal_actions()
            .iter()
            .any(|action| matches!(action, Action::PlayCard { .. }))
    );
}

#[test]
fn sadness_cards_resolve_their_primary_effect_before_generation() {
    for (upgrade, block, damage, weak, draws) in [(0, 14, 13, 2, 3), (1, 18, 17, 3, 4)] {
        let mut sim = fight("CARD.JEST", upgrade, 0, 0, 100);
        play(&mut sim, "CARD.JEST");
        assert_eq!(creature(&sim, 0).block, block);
        assert_eq!(pile(&sim, PileName::Hand), ["CARD.SADNESS"]);
        let mut sim = fight("CARD.SORROWFUL_STRIKE", upgrade, 0, 0, 100);
        play(&mut sim, "CARD.SORROWFUL_STRIKE");
        assert_eq!(creature(&sim, 1).current_hp, 100 - damage);
        assert_eq!(pile(&sim, PileName::Hand), ["CARD.SADNESS"; 2]);
        let mut sim = fight("CARD.VITRIOL", upgrade, 0, 0, 100);
        play(&mut sim, "CARD.VITRIOL");
        assert_eq!(creature(&sim, 1).power_amount("POWER.WEAK_POWER"), weak);
        assert_eq!(pile(&sim, PileName::Hand), ["CARD.SADNESS"]);
        let mut sim = fight("CARD.RUMMAGE", upgrade, 0, 5, 100);
        play(&mut sim, "CARD.RUMMAGE");
        let hand = pile(&sim, PileName::Hand);
        assert_eq!(hand.len(), draws + 1);
        assert_eq!(hand.last().expect("sadness"), "CARD.SADNESS");
    }
}

#[test]
fn brighten_up_gives_sadness_hand_space_before_joy() {
    for upgrade in [0, 1] {
        let mut sim = fight("CARD.BRIGHTEN_UP", upgrade, 8, 0, 100);
        play(&mut sim, "CARD.BRIGHTEN_UP");
        let hand = pile(&sim, PileName::Hand);
        assert_eq!(hand.len(), 10);
        assert_eq!(&hand[8..], ["CARD.SADNESS"; 2]);
        assert_eq!(
            pile(&sim, PileName::Discard),
            vec!["CARD.JOY"; 2 + usize::from(upgrade)]
        );
    }
}

#[test]
fn timber_splashes_only_when_the_first_attack_kills() {
    for (upgrade, damage, splash) in [(0, 20, 30), (1, 25, 35)] {
        let mut sim = fight("CARD.TIMBER", upgrade, 0, 0, 100);
        play(&mut sim, "CARD.TIMBER");
        assert_eq!(creature(&sim, 1).current_hp, 100 - damage);
        assert_eq!(creature(&sim, 2).current_hp, 100);
        let mut sim = fight("CARD.TIMBER", upgrade, 0, 0, damage);
        play(&mut sim, "CARD.TIMBER");
        assert_eq!(creature(&sim, 2).current_hp, 100 - splash);
    }
}

#[test]
fn priestess_and_emperor_draw_and_grant_energy_then_exhaust() {
    for (upgrade, draws, gain) in [(0, 4, 7), (1, 5, 9)] {
        let mut sim = fight("CARD.THE_HIGH_PRIESTESS", upgrade, 0, 7, 100);
        play(&mut sim, "CARD.THE_HIGH_PRIESTESS");
        assert_eq!(pile(&sim, PileName::Hand).len(), draws);
        assert_eq!(pile(&sim, PileName::Exhaust), ["CARD.THE_HIGH_PRIESTESS"]);
        let mut sim = fight("CARD.THE_EMPEROR", upgrade, 0, 0, 100);
        play(&mut sim, "CARD.THE_EMPEROR");
        assert_eq!(
            sim.state().combat.as_ref().expect("combat").player.energy,
            9 + gain
        );
    }
}

#[test]
fn empress_consumes_artifact_on_weak_before_vulnerable() {
    let mut sim = scenario("CARD.THE_EMPRESS", 0, 0, 0, 100)
        .power(
            1,
            id("POWER.ARTIFACT_POWER"),
            1,
            sts2_engine::PowerType::Buff,
        )
        .build(sts2_content::standard_registry())
        .expect("scenario builds");
    play(&mut sim, "CARD.THE_EMPRESS");
    assert_eq!(creature(&sim, 1).power_amount("POWER.WEAK_POWER"), 0);
    assert_eq!(creature(&sim, 1).power_amount("POWER.VULNERABLE_POWER"), 3);
}

#[test]
fn justice_hits_one_enemy_and_chariot_hits_both() {
    for (upgrade, justice, chariot) in [(0, 30, 25), (1, 35, 30)] {
        for (model, damage, other) in [
            ("CARD.JUSTICE", justice, 0),
            ("CARD.THE_CHARIOT", chariot, chariot),
        ] {
            let mut sim = fight(model, upgrade, 0, 0, 100);
            play(&mut sim, model);
            assert_eq!(creature(&sim, 1).current_hp, 100 - damage);
            assert_eq!(creature(&sim, 2).current_hp, 100 - other);
        }
    }
}

#[test]
fn hierophant_generates_distinct_colorless_cards_with_relative_discounts() {
    for upgrade in [0, 1] {
        let mut sim = fight("CARD.THE_HIEROPHANT", upgrade, 0, 0, 100);
        play(&mut sim, "CARD.THE_HIEROPHANT");
        let combat = sim.state().combat.as_ref().expect("combat");
        let hand = &combat.player.piles.hand;
        assert_eq!(hand.len(), 3 + usize::from(upgrade));
        let unique: std::collections::BTreeSet<_> =
            hand.iter().map(|card| &card.fingerprint.model_id).collect();
        assert_eq!(unique.len(), hand.len());
        for card in hand {
            assert!(card.temporary_costs.iter().any(|cost| cost.relative
                && cost.cost == -1
                && cost.clears_when_turn_ends
                && cost.clears_when_played));
        }
    }
}

#[test]
fn magician_adds_slots_before_channeling_and_moon_fills_only_empty_hand_slots() {
    for upgrade in [0, 1] {
        let mut sim = fight("CARD.THE_MAGICIAN", upgrade, 0, 0, 100);
        play(&mut sim, "CARD.THE_MAGICIAN");
        let player = &sim.state().combat.as_ref().expect("combat").player;
        assert_eq!(player.orbs.orbs.len(), 3 + usize::from(upgrade));
        let mut sim = fight("CARD.THE_MOON", upgrade, 7, 0, 100);
        play(&mut sim, "CARD.THE_MOON");
        let hand = &sim
            .state()
            .combat
            .as_ref()
            .expect("combat")
            .player
            .piles
            .hand;
        assert_eq!(hand.len(), 10);
        let shivs: Vec<_> = hand
            .iter()
            .filter(|card| card.fingerprint.model_id == id("CARD.SHIV"))
            .collect();
        assert_eq!(shivs.len(), 3);
        assert!(
            shivs
                .iter()
                .all(|card| card.fingerprint.upgrade_level == upgrade)
        );
    }
}
