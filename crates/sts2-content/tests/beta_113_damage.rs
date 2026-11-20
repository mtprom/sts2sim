//! Damage/attack contracts from the v0.113.1 game command and hook implementations.
use std::sync::Arc;

use sts2_core::ModelId;
use sts2_engine::{
    Action, Amount, CardFingerprint, Command, CreatureSelector, DamageCmd, HookCondition,
    HookEvent, HookTiming, ModifierCmd, ModifierHook, PileName, PowerType, ScenarioBuilder, Simulator,
    ValueProps,
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
        .card(PileName::Hand, 0, CardFingerprint::base(id("CARD.STRIKE_IRONCLAD")), 1)
        .build(Arc::new(content))
        .expect("scenario");
    play(&mut simulator);
    assert_eq!(simulator.state().combat.as_ref().unwrap().creatures[1].current_hp, 80);
}
