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

#[test]
fn strength_death_temperance_and_star_apply_their_literal_values() {
    for upgrade in [0, 1] {
        let n = i32::from(upgrade);
        let mut sim = fight("CARD.STRENGTH", upgrade, 0, 0, 100);
        play(&mut sim, "CARD.STRENGTH");
        assert_eq!(
            creature(&sim, 0).power_amount("POWER.STRENGTH_POWER"),
            4 + n
        );
        let mut sim = fight("CARD.DEATH", upgrade, 0, 0, 100);
        play(&mut sim, "CARD.DEATH");
        assert_eq!(creature(&sim, 1).current_hp, 100);
        assert_eq!(
            creature(&sim, 1).power_amount("POWER.DOOM_POWER"),
            75 + 24 * n
        );
        let mut sim = fight("CARD.TEMPERANCE", upgrade, 0, 0, 100);
        play(&mut sim, "CARD.TEMPERANCE");
        assert_eq!(creature(&sim, 0).block, 30 + 5 * n);
        let mut sim = fight("CARD.THE_STAR", upgrade, 0, 0, 100);
        play(&mut sim, "CARD.THE_STAR");
        assert_eq!(
            creature(&sim, 0).power_amount("POWER.ENERGY_NEXT_TURN_POWER"),
            3 + n
        );
        assert_eq!(
            creature(&sim, 0).power_amount("POWER.DRAW_CARDS_NEXT_TURN_POWER"),
            3 + n
        );
    }
}

#[test]
fn devil_exhausts_the_snapshot_hand_then_autoplays_that_many_without_forced_exhaust() {
    let mut sim = fight("CARD.THE_DEVIL", 0, 2, 3, 100);
    play(&mut sim, "CARD.THE_DEVIL");
    assert_eq!(pile(&sim, PileName::Hand).len(), 0);
    assert_eq!(pile(&sim, PileName::Draw).len(), 1);
    assert_eq!(pile(&sim, PileName::Discard), ["CARD.DEFEND_FOOL"; 2]);
    assert_eq!(creature(&sim, 0).block, 10);
    assert_eq!(
        pile(&sim, PileName::Exhaust),
        ["CARD.DEFEND_FOOL", "CARD.DEFEND_FOOL", "CARD.THE_DEVIL",]
    );
}

#[test]
fn hermit_exhausts_curses_but_discards_other_cards_before_drawing() {
    for (chosen, destination) in [
        ("CARD.INJURY", PileName::Exhaust),
        ("CARD.DEFEND_FOOL", PileName::Discard),
    ] {
        for upgrade in [0, 1] {
            let mut sim = scenario("CARD.THE_HERMIT", upgrade, 0, 4, 100)
                .card(PileName::Hand, 50, CardFingerprint::base(id(chosen)), 0)
                .build(sts2_content::standard_registry())
                .expect("scenario builds");
            play(&mut sim, "CARD.THE_HERMIT");
            if let Some(choice) = sim
                .legal_actions()
                .iter()
                .find(|action| {
                    matches!(
                        action, Action::ChooseCards { cards, .. } if cards.len() == 1
                    )
                })
                .cloned()
            {
                sim.step(choice).expect("choose discard");
            }
            assert!(pile(&sim, destination).contains(&chosen.to_owned()));
            assert_eq!(pile(&sim, PileName::Hand).len(), 2 + usize::from(upgrade));
        }
    }
}

#[test]
fn lovers_fills_hand_from_other_characters_and_upgrades_only_when_requested() {
    let registry = sts2_content::standard_registry();
    for upgrade in [0, 1] {
        let mut sim = fight("CARD.THE_LOVERS", upgrade, 7, 0, 100);
        play(&mut sim, "CARD.THE_LOVERS");
        let hand = &sim
            .state()
            .combat
            .as_ref()
            .expect("combat")
            .player
            .piles
            .hand;
        assert_eq!(hand.len(), 10);
        for card in &hand[7..] {
            let definition = registry
                .card(&card.fingerprint.model_id)
                .expect("generated model");
            assert_ne!(definition.visual_pool, Some(id("CARD_POOL.FOOL_CARD_POOL")));
            assert_eq!(card.fingerprint.upgrade_level, upgrade);
            assert!(card.temporary_costs.is_empty());
        }
    }
}

#[test]
fn sun_draws_then_grants_energy_and_puts_soot_in_discard() {
    for upgrade in [0, 1] {
        let mut sim = fight("CARD.THE_SUN", upgrade, 0, 4, 100);
        play(&mut sim, "CARD.THE_SUN");
        assert_eq!(pile(&sim, PileName::Hand).len(), 2 + usize::from(upgrade));
        assert_eq!(
            sim.state().combat.as_ref().expect("combat").player.energy,
            11 + i32::from(upgrade)
        );
        assert_eq!(pile(&sim, PileName::Discard), ["CARD.SOOT"]);
    }
}

#[test]
fn world_moves_random_discard_cards_only_up_to_its_limit_and_free_space() {
    for (upgrade, held, expected) in [(0, 0, 5), (1, 0, 7), (1, 8, 2)] {
        let mut scenario = scenario("CARD.THE_WORLD", upgrade, held, 0, 100);
        for index in 0..9 {
            scenario = scenario.card(
                PileName::Discard,
                50 + index,
                CardFingerprint::base(id("CARD.STRIKE_FOOL")),
                0,
            );
        }
        let mut sim = scenario
            .build(sts2_content::standard_registry())
            .expect("scenario builds");
        play(&mut sim, "CARD.THE_WORLD");
        assert_eq!(pile(&sim, PileName::Hand).len(), held + expected);
        assert_eq!(pile(&sim, PileName::Discard).len(), 9 - expected);
    }
}

#[test]
fn wheel_draws_even_when_hand_is_empty_and_discards_otherwise() {
    for upgrade in [0, 1] {
        for held in [0, 2] {
            let mut sim = fight("CARD.WHEEL_OF_FORTUNE", upgrade, held, 10, 100);
            play(&mut sim, "CARD.WHEEL_OF_FORTUNE");
            assert_eq!(
                pile(&sim, PileName::Hand).len(),
                7 + 2 * usize::from(upgrade)
            );
            assert_eq!(pile(&sim, PileName::Discard).len(), held);
        }
    }
}

#[test]
fn upgraded_devil_upgrades_each_card_as_it_is_taken_for_autoplay() {
    let mut sim = fight("CARD.THE_DEVIL", 1, 2, 3, 100);
    play(&mut sim, "CARD.THE_DEVIL");
    assert_eq!(creature(&sim, 0).block, 16);
    let piles = &sim.state().combat.as_ref().expect("combat").player.piles;
    assert!(
        piles
            .discard
            .iter()
            .all(|card| card.fingerprint.upgrade_level == 1)
    );
    assert!(
        piles
            .draw
            .iter()
            .all(|card| card.fingerprint.upgrade_level == 0)
    );
}

#[test]
fn judgement_sets_a_target_at_or_below_its_threshold_to_zero() {
    for (upgrade, threshold) in [(0, 30), (1, 40)] {
        let mut sim = fight("CARD.JUDGEMENT", upgrade, 0, 0, threshold);
        play(&mut sim, "CARD.JUDGEMENT");
        let combat = sim.state().combat.as_ref().expect("combat");
        assert!(
            combat
                .creatures
                .iter()
                .all(|creature| creature.id.get() != 1 || !creature.alive())
        );
        assert_eq!(creature(&sim, 2).current_hp, 100);
        assert_eq!(pile(&sim, PileName::Exhaust), ["CARD.JUDGEMENT"]);
        let mut sim = fight("CARD.JUDGEMENT", upgrade, 0, 0, threshold + 1);
        play(&mut sim, "CARD.JUDGEMENT");
        assert_eq!(creature(&sim, 1).current_hp, threshold + 1);
    }
}

#[test]
fn tower_deals_its_multiplier_times_the_total_floor() {
    let run = sts2_content::standard_run_on_preset(
        "N3Z9EE2CXB",
        &id("CHARACTER.FOOL"),
        &sts2_core::UnlockPresetManifest::pinned().expect("preset"),
    )
    .expect("Fool run starts");
    for (upgrade, per_floor) in [(0, 2), (1, 3)] {
        let mut state = fight("CARD.THE_TOWER", upgrade, 0, 0, 100).state().clone();
        let mut walked = run.state().run.clone().expect("run state");
        walked.floor = 7;
        state.run = Some(walked);
        let mut sim =
            Simulator::from_scenario(state, sts2_content::registry_for(&id("CHARACTER.FOOL")))
                .expect("scenario with a run");
        play(&mut sim, "CARD.THE_TOWER");
        assert_eq!(creature(&sim, 1).current_hp, 100 - 7 * per_floor);
        assert_eq!(creature(&sim, 2).current_hp, 100);
    }
}

#[test]
fn missing_primitives_fail_explicitly_without_fake_effects() {
    for (model, upgrade) in [("CARD.FOOLISH_SHOT", 0)] {
        let mut sim = fight(model, upgrade, 0, 0, 100);
        let action = sim
            .legal_actions()
            .iter()
            .find(|action| {
                matches!(
                    action, Action::PlayCard { card, .. } if card.fingerprint.model_id == id(model)
                )
            })
            .cloned()
            .expect("registered playable card");
        let error = sim.step(action).expect_err("unsupported body must fail");
        assert_eq!(error.code, sts2_engine::ErrorCode::UnsupportedContent);
    }
}

#[test]
fn fool_starts_with_77_hp_99_gold_and_the_exact_ten_card_deck() {
    let sim = sts2_content::standard_run_on_preset(
        "N3Z9EE2CXB",
        &id("CHARACTER.FOOL"),
        &sts2_core::UnlockPresetManifest::pinned().expect("preset"),
    )
    .expect("Fool run starts");
    let player = &sim.state().run_player;
    assert_eq!(
        (player.current_hp, player.max_hp, player.gold),
        (77, 77, 99)
    );
    let deck: Vec<_> = player
        .deck
        .iter()
        .map(|card| card.model_id.to_string())
        .collect();
    assert_eq!(&deck[..5], ["CARD.STRIKE_FOOL"; 5]);
    assert_eq!(&deck[5..9], ["CARD.DEFEND_FOOL"; 4]);
    assert_eq!(deck[9], "CARD.FOOLISH_SHOT");
    assert!(
        player
            .relics
            .iter()
            .any(|relic| relic.model_id == id("RELIC.SONG_BOW"))
    );
}

#[test]
fn fool_reward_pool_preserves_order_and_excludes_basics_ancients_and_multiplayer() {
    let registry = sts2_content::standard_registry();
    let pool = registry
        .card_pool(&id("CARD_POOL.FOOL_CARD_POOL"))
        .expect("Fool pool");
    assert_eq!(pool.len(), 79);
    assert_eq!(
        pool.first().expect("first").model_id,
        id("CARD.ALL_IS_WELL")
    );
    assert_eq!(pool.last().expect("last").model_id, id("CARD.YAH"));
    let ids: Vec<_> = pool.iter().map(|card| card.model_id.to_string()).collect();
    assert!(
        ids.iter().position(|model| model == "CARD.LOCKED_IN")
            < ids.iter().position(|model| model == "CARD.LULLABY")
    );
    for excluded in [
        "CARD.STRIKE_FOOL",
        "CARD.DEFEND_FOOL",
        "CARD.FOOLISH_SHOT",
        "CARD.DIVINE_SHOT",
        "CARD.LOST_LUTE",
        "CARD.COMPOST",
        "CARD.COVERING_FIRE",
        "CARD.ENRAPTURE",
        "CARD.ENSEMBLE",
        "CARD.SELF_DEPRECATE",
        "CARD.YOGA",
    ] {
        assert!(!ids.contains(&excluded.to_owned()), "{excluded}");
        assert!(
            registry.card(&id(excluded)).is_some(),
            "{excluded} retains its identity"
        );
    }
    let preset = sts2_core::UnlockPresetManifest::pinned().expect("preset");
    assert_eq!(preset.unlocks.epochs.len(), 66);
    for epoch in 1..=7 {
        assert!(
            preset
                .unlocks
                .epochs
                .contains(&format!("FOOL{epoch}_EPOCH"))
        );
    }
}

#[test]
fn song_bow_stops_combat_entry_rather_than_silently_omitting_aim() {
    let mut sim = sts2_content::standard_run_on_preset(
        "N3Z9EE2CXB",
        &id("CHARACTER.FOOL"),
        &sts2_core::UnlockPresetManifest::pinned().expect("preset"),
    )
    .expect("Fool run starts");
    for _ in 0..200 {
        let action = sim
            .legal_actions()
            .iter()
            .find(|action| {
                matches!(
                    action, Action::ChooseCards { cards, .. } if !cards.is_empty()
                )
            })
            .or_else(|| sim.legal_actions().first())
            .cloned()
            .expect("live run");
        if let Err(error) = sim.step(action) {
            assert_eq!(error.code, sts2_engine::ErrorCode::UnsupportedContent);
            assert!(format!("{error:?}").contains("RELIC.SONG_BOW"), "{error:?}");
            return;
        }
    }
    panic!("walk never entered combat");
}
