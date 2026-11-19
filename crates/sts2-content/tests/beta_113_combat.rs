//! Non-Dread combat rules verified against the v0.113.1 game models.

use sts2_engine::{Action, CombatSide, Creature, PowerType, ScenarioBuilder, Simulator};

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().expect("static ID")
}

fn creature(simulator: &Simulator, side: CombatSide) -> &Creature {
    simulator.state().combat.as_ref().expect("a fight").creatures.iter()
        .find(|creature| creature.side == side).expect("creature on side")
}

fn end_turn(simulator: &mut Simulator) {
    let action = simulator.legal_actions().iter()
        .find(|action| matches!(action, Action::EndTurn { .. }))
        .cloned().expect("can end turn");
    simulator.step(action).expect("end turn");
}

/// Infested Prism cycles Jab, Jab, Whirlwind, Pulsate. Neither Jab
/// grants Block; Pulsate applies one Vulnerable instead of Block.
#[test]
fn infested_prism_has_two_jabs_and_a_vulnerable_pulsate() {
    for ascension in [0, 9] {
        let mut simulator = ScenarioBuilder::new("PRISM113", id("ENCOUNTER.INFESTED_PRISMS_ELITE"))
            .player_hp(400, 400).energy(3, 3).turn(1, 1)
            .enemy(1, id("MONSTER.INFESTED_PRISM"), 161, 161, "JAB_MOVE")
            .build(sts2_content::registry_at(&id("CHARACTER.IRONCLAD"), ascension))
            .expect("scenario");
        for next in ["JAB_MOVE_2", "WHIRLWIND_MOVE", "PULSATE_MOVE", "JAB_MOVE"] {
            end_turn(&mut simulator);
            let enemy = creature(&simulator, CombatSide::Enemy);
            assert_eq!(enemy.move_state, next);
            assert_eq!(enemy.block, 0);
        }
        assert_eq!(
            creature(&simulator, CombatSide::Player).power_amount("POWER.VULNERABLE_POWER"),
            1
        );
    }
}

/// Plow's Strength is conditional after its damage resolves: breaking the
/// power with retaliatory Thorns suppresses the Strength grant.
#[test]
fn broken_plow_does_not_regain_strength() {
    for (thorns, strength) in [(0, 2), (1, 0)] {
        let mut scenario = ScenarioBuilder::new("PLOW113", id("ENCOUNTER.CEREMONIAL_BEAST_BOSS"))
            .player_hp(400, 400).energy(3, 3).turn(1, 1)
            .enemy(1, id("MONSTER.CEREMONIAL_BEAST"), 252, 252, "PLOW_MOVE")
            .power(1, id("POWER.PLOW_POWER"), 1, PowerType::Buff);
        if thorns > 0 {
            scenario = scenario.power(0, id("POWER.THORNS_POWER"), thorns, PowerType::Buff);
        }
        let mut simulator = scenario.build(sts2_content::standard_registry()).expect("scenario");
        end_turn(&mut simulator);
        assert_eq!(
            creature(&simulator, CombatSide::Enemy).power_amount("POWER.STRENGTH_POWER"),
            strength
        );
    }
}
