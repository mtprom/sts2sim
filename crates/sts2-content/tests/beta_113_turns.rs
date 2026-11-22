//! Turn and play scheduling contracts from the v0.113.1 combat manager and
//! card models.
use std::sync::Arc;

use sts2_core::ModelId;
use sts2_engine::{
    Action, Amount, CardFingerprint, Command, Condition, ContentRegistry, CreatureCmd,
    CreatureSelector, HookCondition, HookEvent, HookTiming, ModifierHook, PileName, PowerType,
    ScenarioBuilder, Simulator, ValueProps,
};

fn id(value: &str) -> ModelId {
    value.parse().expect("static ID")
}

fn end_turn(simulator: &mut Simulator) {
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::EndTurn { .. }))
        .cloned()
        .expect("can end turn");
    simulator.step(action).expect("end turn");
}

/// A stand-in power whose only hooks are the ones a test gives it.
fn observer(content: &mut ContentRegistry, hooks: Vec<ModifierHook>) -> ModelId {
    let mut power = content
        .power(&id("POWER.DEXTERITY_POWER"))
        .expect("dexterity")
        .clone();
    power.hooks = hooks;
    content.add_power(power);
    id("POWER.DEXTERITY_POWER")
}

fn gain_block(amount: i32) -> Command {
    Command::Creature(CreatureCmd::GainBlock {
        target: CreatureSelector::Owner,
        amount: Amount::Fixed(amount),
        props: ValueProps::UNPOWERED,
    })
}

/// `CombatManager` notifies `AfterModifyingHandDraw` between settling the hand
/// draw and drawing, so Draw Cards Next Turn is gone before the first card
/// moves while its two cards are still owed.
#[test]
fn draw_cards_next_turn_leaves_before_the_hand_is_drawn() {
    let mut content = (*sts2_content::standard_registry()).clone();
    let watcher = observer(
        &mut content,
        vec![ModifierHook {
            event: HookEvent::AfterCardDrawn,
            timing: HookTiming::EveryTurn,
            condition: HookCondition::Always,
            commands: vec![Command::When {
                condition: Condition::Not(Box::new(Condition::OwnerHasPower(id(
                    "POWER.DRAW_CARDS_NEXT_TURN_POWER",
                )))),
                then: vec![gain_block(1)],
                otherwise: Vec::new(),
            }],
        }],
    );
    let mut scenario = ScenarioBuilder::new("DRAW113", id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(3, 3)
        .turn(1, 1)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 100, 100, "SHRINKER_MOVE")
        .power(0, watcher, 1, PowerType::Buff)
        .power(0, id("POWER.DRAW_CARDS_NEXT_TURN_POWER"), 2, PowerType::Buff);
    for card in 0..10 {
        scenario = scenario.card(
            PileName::Draw,
            card,
            CardFingerprint::base(id("CARD.STRIKE_IRONCLAD")),
            1,
        );
    }
    let mut simulator = scenario.build(Arc::new(content)).expect("scenario");
    end_turn(&mut simulator);
    let combat = simulator.state().combat.as_ref().expect("a fight");
    assert_eq!(combat.player.piles.hand.len(), 7);
    let player = &combat.creatures[0];
    assert_eq!(player.power_amount("POWER.DRAW_CARDS_NEXT_TURN_POWER"), 0);
    assert_eq!(player.block, 7);
}
