//! Damage/attack contracts from the v0.113.1 game command and hook implementations.
use std::sync::Arc;

use sts2_core::ModelId;
use sts2_engine::{
    Action, Amount, CardFingerprint, Command, CreatureSelector, DamageCmd, HookCondition,
    HookEvent, HookTiming, ModifierCmd, ModifierHook, PileName, PowerType, ScenarioBuilder,
    Simulator, ValueProps,
};

fn id(value: &str) -> ModelId {
    value.parse().expect("static ID")
}

fn play(simulator: &mut Simulator) {
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::PlayCard { .. }))
        .cloned()
        .expect("playable card");
    simulator.step(action).expect("play");
}

/// Hook.ModifyDamageInternal folds additive, ordinary, late, then cap.
#[test]
fn damage_late_multiplication_precedes_caps() {
    let mut content = (*sts2_content::standard_registry()).clone();
    let mut strike = content.card(&id("CARD.STRIKE_IRONCLAD")).unwrap().clone();
    strike.effects = vec![Command::Damage(DamageCmd::Attack {
        amount: Amount::Fixed(5),
        hits: Amount::Fixed(1),
        target: CreatureSelector::Target,
        props: ValueProps::NONE,
    })];
    content.add_card(strike);
    let mut power = content.power(&id("POWER.STRENGTH_POWER")).unwrap().clone();
    power.hooks = vec![
        ModifierHook {
            event: HookEvent::ModifyDamageAdditive,
            timing: HookTiming::EveryTurn,
            condition: HookCondition::OwnerDealsPoweredDamageFromACard,
            commands: vec![Command::Modify(ModifierCmd::Add(Amount::Fixed(1)))],
        },
        ModifierHook {
            event: HookEvent::ModifyDamageMultiplicative,
            timing: HookTiming::EveryTurn,
            condition: HookCondition::OwnerDealsPoweredDamageFromACard,
            commands: vec![Command::Modify(ModifierCmd::Scale(Amount::Fixed(200)))],
        },
        ModifierHook {
            event: HookEvent::ModifyDamageMultiplicativeLate,
            timing: HookTiming::EveryTurn,
            condition: HookCondition::OwnerDealsPoweredDamageFromACard,
            commands: vec![Command::Modify(ModifierCmd::Scale(Amount::Fixed(300)))],
        },
        ModifierHook {
            event: HookEvent::ModifyDamageCap,
            timing: HookTiming::EveryTurn,
            condition: HookCondition::OwnerDealsPoweredDamageFromACard,
            commands: vec![Command::Modify(ModifierCmd::CapAt(Amount::Fixed(20)))],
        },
    ];
    content.add_power(power);
    let mut simulator = ScenarioBuilder::new("LATE113", id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(3, 3)
        .turn(1, 1)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 100, 100, "SHRINKER_MOVE")
        .power(0, id("POWER.STRENGTH_POWER"), 1, PowerType::Buff)
        .card(
            PileName::Hand,
            0,
            CardFingerprint::base(id("CARD.STRIKE_IRONCLAD")),
            1,
        )
        .build(Arc::new(content))
        .expect("scenario");
    play(&mut simulator);
    assert_eq!(
        simulator.state().combat.as_ref().unwrap().creatures[1].current_hp,
        80
    );
}

/// Thumbscrew modifies a multi-hit Attack card once, including `HitCountVar`'s
/// null-CardPlay preview, but never turns a single hit into two.
#[test]
fn thumbscrew_preview_matches_attack_hits() {
    for (card, gold, damage, hits) in [
        ("CARD.TWIN_STRIKE", 0, 5, 3),
        ("CARD.STRIKE_IRONCLAD", 0, 6, 1),
        ("CARD.MY_SHINIES", 40, 1, 3),
        ("CARD.MY_SHINIES", 20, 1, 1),
    ] {
        let mut simulator = ScenarioBuilder::new("HITS113", id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
            .player_hp(80, 80)
            .energy(3, 3)
            .turn(1, 1)
            .gold(gold)
            .relics(&[id("RELIC.THUMBSCREW")])
            .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 100, 100, "SHRINKER_MOVE")
            .card(PileName::Hand, 0, CardFingerprint::base(id(card)), 1)
            .build(sts2_content::standard_registry())
            .expect("scenario");
        assert_eq!(
            simulator.agent_observation().hand_previews[0].hits,
            Some(hits),
            "{card}"
        );
        play(&mut simulator);
        assert_eq!(
            simulator.state().combat.as_ref().unwrap().creatures[1].current_hp,
            100 - damage * hits,
            "{card}"
        );
    }
    let mut content = (*sts2_content::standard_registry()).clone();
    let mut skill = content.card(&id("CARD.TWIN_STRIKE")).unwrap().clone();
    skill.card_type = sts2_engine::CardType::Skill;
    content.add_card(skill);
    let mut simulator = ScenarioBuilder::new("SKILLHITS113", id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .relics(&[id("RELIC.THUMBSCREW")])
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 100, 100, "SHRINKER_MOVE")
        .card(
            PileName::Hand,
            0,
            CardFingerprint::base(id("CARD.TWIN_STRIKE")),
            1,
        )
        .build(Arc::new(content))
        .unwrap();
    assert_eq!(simulator.agent_observation().hand_previews[0].hits, Some(2));
    play(&mut simulator);
    assert_eq!(
        simulator.state().combat.as_ref().unwrap().creatures[1].current_hp,
        90
    );
}

/// Lethality's in-Play subtraction requires a supplied `CardPlay`. A card
/// moved to Play for its held-at-turn-end effect is a card source without a
/// play, so the turn's one started Attack already used the bonus.
#[test]
fn lethality_requires_a_real_play_for_the_current_attack_exception() {
    let mut content = (*sts2_content::standard_registry()).clone();
    let mut held = content.card(&id("CARD.DEFEND_IRONCLAD")).unwrap().clone();
    held.end_of_turn_in_hand = vec![Command::Damage(DamageCmd::Attack {
        amount: Amount::Fixed(6),
        hits: Amount::Fixed(1),
        target: CreatureSelector::AllOpponents,
        props: ValueProps::NONE,
    })];
    content.add_card(held);
    let mut simulator = ScenarioBuilder::new("LETHALITY113", id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(1, 1)
        .turn(1, 1)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 100, 100, "SHRINKER_MOVE")
        .power(0, id("POWER.LETHALITY_POWER"), 100, PowerType::Buff)
        .card(
            PileName::Hand,
            0,
            CardFingerprint::base(id("CARD.STRIKE_IRONCLAD")),
            1,
        )
        .card(
            PileName::Hand,
            1,
            CardFingerprint::base(id("CARD.DEFEND_IRONCLAD")),
            9,
        )
        .build(Arc::new(content))
        .expect("scenario");
    play(&mut simulator);
    assert_eq!(
        simulator.state().combat.as_ref().unwrap().creatures[1].current_hp,
        88
    );
    let end = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::EndTurn { .. }))
        .cloned()
        .unwrap();
    simulator.step(end).unwrap();
    assert_eq!(
        simulator.state().combat.as_ref().unwrap().creatures[1].current_hp,
        82
    );
}

fn step_matching(simulator: &mut Simulator, wanted: impl Fn(&Action) -> bool) {
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| wanted(action))
        .cloned()
        .expect("legal action");
    simulator.step(action).expect("step");
}

fn shrink_on(simulator: &Simulator, creature: usize) -> bool {
    let creature = &simulator.state().combat.as_ref().unwrap().creatures[creature];
    creature
        .powers
        .iter()
        .any(|power| power.model_id == id("POWER.SHRINK_POWER"))
}

/// ShrinkPower.AfterDeath removes itself when its applier dies, unless that
/// applier is a player. A monster applier's death takes it off the player.
#[test]
fn shrink_ignores_a_player_appliers_death() {
    // A single player's death ends the combat before anything could read
    // the power, so the player half is the definition's own flag.
    let content = sts2_content::standard_registry();
    let shrink = content.power(&id("POWER.SHRINK_POWER")).unwrap();
    assert!(shrink.removed_when_applier_dies && shrink.kept_when_player_applier_dies);

    let mut simulator =
        ScenarioBuilder::new("SHRINKMONSTER113", id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
            .player_hp(80, 80)
            .energy(3, 3)
            .turn(1, 1)
            .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 4, 100, "SHRINKER_MOVE")
            .card(
                PileName::Draw,
                0,
                CardFingerprint::base(id("CARD.STRIKE_IRONCLAD")),
                1,
            )
            .build(sts2_content::standard_registry())
            .expect("scenario");
    step_matching(&mut simulator, |action| {
        matches!(action, Action::EndTurn { .. })
    });
    assert!(shrink_on(&simulator, 0));
    play(&mut simulator);
    assert!(!shrink_on(&simulator, 0));
}

/// Omnislice splashes its first result's total plus overkill as powered damage
/// that only the cap pass modifies: no second Strength, but caps, Thorns and
/// powered-hit listeners all apply.
#[test]
fn omnislice_splash_is_powered_and_cap_only() {
    let simulator = ScenarioBuilder::new("OMNI113", id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(3, 3)
        .turn(1, 1)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 3, 100, "SHRINKER_MOVE")
        .enemy(2, id("MONSTER.SHRINKER_BEETLE"), 100, 100, "SHRINKER_MOVE")
        .enemy(3, id("MONSTER.SHRINKER_BEETLE"), 100, 100, "SHRINKER_MOVE")
        .power(0, id("POWER.STRENGTH_POWER"), 2, PowerType::Buff)
        .power(2, id("POWER.CURL_UP_POWER"), 5, PowerType::Buff)
        .power(3, id("POWER.HARD_TO_KILL_POWER"), 1, PowerType::Buff)
        .power(3, id("POWER.THORNS_POWER"), 3, PowerType::Buff)
        .card(
            PileName::Hand,
            0,
            CardFingerprint::base(id("CARD.OMNISLICE")),
            1,
        )
        .build(sts2_content::standard_registry())
        .expect("scenario");
    let mut state = simulator.state().clone();
    state.combat.as_mut().unwrap().creatures[1].block = 4;
    let mut simulator = Simulator::from_scenario(state, sts2_content::standard_registry()).unwrap();
    step_matching(
        &mut simulator,
        |action| matches!(action, Action::PlayCard { target: Some(target), .. } if target.combat_id == sts2_core::CreatureCombatId::from(1)),
    );
    let combat = simulator.state().combat.as_ref().unwrap();
    let enemy = |id: u64| {
        combat
            .creatures
            .iter()
            .find(|creature| creature.id == sts2_core::CreatureCombatId::from(id))
            .map(|creature| (creature.current_hp, creature.block))
    };
    // 8 + 2 Strength: 4 blocked, 3 lost, 3 overkill, so the splash base is 10.
    assert_eq!(enemy(1), None);
    assert_eq!(enemy(2), Some((90, 5)));
    assert_eq!(enemy(3), Some((99, 0)));
    assert_eq!(combat.creatures[0].current_hp, 77);
}

/// `LoseMaxHp` damages the HP above the new maximum with its card as the
/// source. Buffer returns the amount unchanged for Brightest Flame, so it is
/// not a modifying listener and keeps its stack.
#[test]
fn brightest_flame_bypasses_buffer_through_max_hp_loss() {
    let mut simulator = ScenarioBuilder::new("FLAME113", id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(3, 3)
        .turn(1, 1)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 100, 100, "SHRINKER_MOVE")
        .power(0, id("POWER.BUFFER_POWER"), 1, PowerType::Buff)
        .card(
            PileName::Hand,
            0,
            CardFingerprint::base(id("CARD.BRIGHTEST_FLAME")),
            0,
        )
        .build(sts2_content::standard_registry())
        .expect("scenario");
    play(&mut simulator);
    let player = &simulator.state().combat.as_ref().unwrap().creatures[0];
    assert_eq!((player.current_hp, player.max_hp), (78, 78));
    assert!(
        player
            .powers
            .iter()
            .any(|power| power.model_id == id("POWER.BUFFER_POWER") && power.amount == 1)
    );
}
