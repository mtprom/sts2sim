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

/// Thumbscrew modifies a multi-hit Attack card once, including HitCountVar's
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

/// Lethality's in-Play subtraction requires a supplied CardPlay. A card
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
