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

fn play(simulator: &mut Simulator, model_id: &str) {
    let wanted = id(model_id);
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, Action::PlayCard { card, .. } if card.fingerprint.model_id == wanted)
        })
        .cloned()
        .unwrap_or_else(|| panic!("{model_id} is playable"));
    simulator.step(action).expect("play");
}

fn hand(simulator: &Simulator) -> Vec<sts2_engine::CardInstance> {
    simulator.state().combat.as_ref().expect("a fight").player.piles.hand.clone()
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

/// `JugglingPower` remembers the third Attack as it starts and clones it only
/// after that play finishes: a fourth Attack the third plays mid-body cannot
/// claim the count, and the copy carries what the third did to itself.
#[test]
fn juggling_clones_the_third_started_attack_after_it_finishes() {
    let mut content = (*sts2_content::standard_registry()).clone();
    let mut rampage = content.card(&id("CARD.RAMPAGE")).expect("rampage").clone();
    let mut body = vec![Command::CardPile(sts2_engine::CardPileCmd::AutoPlayFromDrawPile {
        count: Amount::Fixed(1),
        position: sts2_engine::DrawPilePosition::Top,
        force_exhaust: false,
        upgrade: false,
    })];
    body.extend(rampage.effects.clone());
    rampage.effects = body;
    content.add_card(rampage);
    let strike = || CardFingerprint::base(id("CARD.STRIKE_IRONCLAD"));
    let mut simulator = ScenarioBuilder::new("JUGGLE113", id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(9, 9)
        .turn(1, 1)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 200, 200, "SHRINKER_MOVE")
        .power(0, id("POWER.JUGGLING_POWER"), 1, PowerType::Buff)
        .card(PileName::Hand, 0, strike(), 1)
        .card(PileName::Hand, 1, strike(), 1)
        .card(PileName::Hand, 2, CardFingerprint::base(id("CARD.RAMPAGE")), 1)
        .card(PileName::Draw, 3, strike(), 1)
        .build(Arc::new(content))
        .expect("scenario");
    play(&mut simulator, "CARD.STRIKE_IRONCLAD");
    play(&mut simulator, "CARD.STRIKE_IRONCLAD");
    play(&mut simulator, "CARD.RAMPAGE");
    let hand = hand(&simulator);
    assert_eq!(hand.len(), 1, "one copy, and none of the nested Strike");
    assert_eq!(hand[0].fingerprint.model_id, id("CARD.RAMPAGE"));
    let combat = simulator.state().combat.as_ref().expect("a fight");
    let original = combat
        .player
        .piles
        .all_cards()
        .find(|card| card.id == sts2_core::CardInstanceId::from(2))
        .expect("the third attack");
    assert!(original.damage_bonus > 0, "Rampage grew itself");
    assert_eq!(hand[0].damage_bonus, original.damage_bonus);
}

/// `CardModel.CreateClone` follows a dupe back to its original: a clone made
/// of a dupe that changed itself is the original as it stands, linked to the
/// original, and is not itself a dupe.
#[test]
fn a_clone_of_a_dupe_copies_the_original() {
    let mut content = (*sts2_content::standard_registry()).clone();
    let mut rampage = content.card(&id("CARD.RAMPAGE")).expect("rampage").clone();
    rampage.effects = vec![
        Command::Card(sts2_engine::CardCmd::GrowDamage {
            cards: sts2_engine::CardSelector::Source,
            amount: Amount::Fixed(5),
        }),
        Command::CardPile(sts2_engine::CardPileCmd::AddGeneratedToCombat {
            enchantment: None,
            card: sts2_engine::CardOrigin::CloneOfSource,
            grown_by: Amount::Fixed(0),
            count: Amount::Fixed(1),
            pile: PileName::Hand,
            ethereal: false,
            retain: false,
            upgraded: false,
            temporary_cost: None,
            position: sts2_engine::DrawPilePosition::Bottom,
        }),
    ];
    content.add_card(rampage);
    let mut defend = content
        .card(&id("CARD.DEFEND_IRONCLAD"))
        .expect("defend")
        .clone();
    defend.effects = vec![Command::Card(sts2_engine::CardCmd::AutoPlayDupe {
        cards: sts2_engine::CardSelector::Pile {
            pile: PileName::Hand,
            filter: sts2_engine::CardFilter::OfType(sts2_engine::CardType::Attack),
        },
    })];
    content.add_card(defend);
    let mut simulator = ScenarioBuilder::new("DUPE113", id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(3, 3)
        .turn(1, 1)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 100, 100, "SHRINKER_MOVE")
        .card(PileName::Hand, 0, CardFingerprint::base(id("CARD.DEFEND_IRONCLAD")), 1)
        .card(PileName::Hand, 1, CardFingerprint::base(id("CARD.RAMPAGE")), 1)
        .build(Arc::new(content))
        .expect("scenario");
    play(&mut simulator, "CARD.DEFEND_IRONCLAD");
    let hand = hand(&simulator);
    assert_eq!(hand.len(), 2, "the original and one clone; the dupe is gone");
    let original = sts2_core::CardInstanceId::from(1);
    assert_eq!(hand[0].id, original);
    assert_eq!(hand[0].damage_bonus, 0);
    let clone = &hand[1];
    assert_eq!(clone.clone_of, Some(original));
    assert_eq!(clone.damage_bonus, 0, "not the dupe's growth");
    assert!(!clone.dupe);
}

/// Swift draws while Normal and is disabled only by its own card's
/// `AfterCardPlayed`, so a draw another listener makes as that play finishes
/// still sees it Normal, and the card ends the play Disabled.
#[test]
fn swift_is_disabled_after_its_play_finishes() {
    let mut content = (*sts2_content::standard_registry()).clone();
    let mut swift = content
        .enchantment(&id("ENCHANTMENT.SWIFT"))
        .expect("swift")
        .clone();
    swift.hooks.push(ModifierHook {
        event: HookEvent::AfterCardDrawn,
        timing: HookTiming::EveryTurn,
        condition: HookCondition::Always,
        commands: vec![Command::When {
            condition: Condition::ListeningEnchantmentIsNormal,
            then: vec![gain_block(1)],
            otherwise: Vec::new(),
        }],
    });
    content.add_enchantment(swift);
    let drawer = observer(
        &mut content,
        vec![ModifierHook {
            event: HookEvent::AfterCardPlayed,
            timing: HookTiming::EveryTurn,
            condition: HookCondition::Always,
            commands: vec![Command::CardPile(sts2_engine::CardPileCmd::Draw {
                count: Amount::Fixed(1),
            })],
        }],
    );
    let mut card = CardFingerprint::base(id("CARD.DEFEND_IRONCLAD"));
    card.enchantment = Some(sts2_engine::EnchantmentFingerprint::new(
        id("ENCHANTMENT.SWIFT"),
        1,
    ));
    let mut scenario = ScenarioBuilder::new("SWIFT113", id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(3, 3)
        .turn(1, 1)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 100, 100, "SHRINKER_MOVE")
        .power(0, drawer, 1, PowerType::Buff)
        .card(PileName::Hand, 0, card, 1);
    for draw in 1..4 {
        scenario = scenario.card(
            PileName::Draw,
            draw,
            CardFingerprint::base(id("CARD.STRIKE_IRONCLAD")),
            1,
        );
    }
    let mut simulator = scenario.build(Arc::new(content)).expect("scenario");
    play(&mut simulator, "CARD.DEFEND_IRONCLAD");
    let combat = simulator.state().combat.as_ref().expect("a fight");
    assert_eq!(combat.player.piles.hand.len(), 2);
    assert_eq!(
        combat.creatures[0].block, 7,
        "Defend's 5, and both draws saw Swift Normal"
    );
    assert_eq!(
        combat.player.piles.discard[0].enchantment_status,
        sts2_engine::EnchantmentStatus::Disabled
    );
}

/// `NightmarePower.SetSelectedCard` keeps a clone with its affliction cleared
/// and `EndOfTurnCleanup` run: the copies dealt before the next hand draw keep
/// the card's fight-long changes, not its turn-only ones, and making the
/// template leaves the original alone.
#[test]
fn nightmare_copies_a_cleaned_clone_before_the_draw() {
    let mut scenario = ScenarioBuilder::new("NIGHTMARE113", id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(9, 9)
        .turn(1, 1)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 100, 100, "SHRINKER_MOVE")
        .card(PileName::Hand, 0, CardFingerprint::base(id("CARD.NIGHTMARE")), 0)
        .card_with(
            PileName::Hand,
            1,
            CardFingerprint::base(id("CARD.SLICE")),
            0,
            |card| {
                card.cost_this_combat = -1;
                card.cost_this_turn = 2;
                card.retain_this_turn = true;
                card.sly_this_turn = true;
                card.affliction = Some(id("AFFLICTION.SMOG"));
            },
        );
    for draw in 2..7 {
        scenario = scenario.card(
            PileName::Draw,
            draw,
            CardFingerprint::base(id("CARD.DEFLECT")),
            0,
        );
    }
    let mut simulator = scenario
        .build(sts2_content::standard_registry())
        .expect("scenario");
    play(&mut simulator, "CARD.NIGHTMARE");
    let original = hand(&simulator)
        .into_iter()
        .find(|card| card.id == sts2_core::CardInstanceId::from(1))
        .expect("the original stays in hand");
    assert_eq!(original.cost_this_turn, 2);
    assert_eq!(original.affliction, Some(id("AFFLICTION.SMOG")));
    end_turn(&mut simulator);
    let hand = hand(&simulator);
    let copies: Vec<usize> = (0..hand.len())
        .filter(|at| hand[*at].clone_of.is_some())
        .collect();
    assert_eq!(copies.len(), 3);
    let first_draw = hand
        .iter()
        .position(|card| card.fingerprint.model_id == id("CARD.DEFLECT"))
        .expect("the draw");
    for at in copies {
        let copy = &hand[at];
        assert!(at < first_draw, "copies come before the draw");
        assert_eq!(copy.cost_this_combat, -1);
        assert_eq!(copy.cost_this_turn, 0);
        assert!(!copy.retain_this_turn && !copy.sly_this_turn);
        assert_eq!(copy.affliction, None);
    }
}

/// `Hook.ShouldTakeExtraTurn` asks the early pass first and tells only the
/// winner: an unused Pael's Eye takes the first extra turn and Ambergris'
/// Extra Turn power is spent only on the next one.
#[test]
fn pael_s_eye_wins_the_extra_turn_before_the_power() {
    let mut simulator = ScenarioBuilder::new("EYE113", id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(3, 3)
        .turn(1, 1)
        .relics(&[id("RELIC.PAELS_EYE")])
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 100, 100, "SHRINKER_MOVE")
        .power(0, id("POWER.EXTRA_TURN_POWER"), 1, PowerType::Buff)
        .build(sts2_content::standard_registry())
        .expect("scenario");
    let extra_turns = |simulator: &Simulator| {
        let combat = simulator.state().combat.as_ref().expect("a fight");
        (
            combat.creatures[0].power_amount("POWER.EXTRA_TURN_POWER"),
            simulator.state().run_player.relics[0].latched,
        )
    };
    end_turn(&mut simulator);
    assert_eq!(extra_turns(&simulator), (1, true), "the eye's turn");
    end_turn(&mut simulator);
    assert_eq!(extra_turns(&simulator), (0, true), "the power's turn");
    let enemy_moves = |simulator: &Simulator| {
        simulator.state().combat.as_ref().expect("a fight").creatures[1]
            .performed_log
            .len()
    };
    assert_eq!(enemy_moves(&simulator), 0, "no enemy turn yet");
}

/// `PaelsLegion` binds the first `CardPlay` whose block it doubles: that play
/// keeps doubling after a nested play, the nested play's block is not doubled,
/// and only the bound play's finishing starts the cooldown.
#[test]
fn paels_legion_doubles_only_the_play_it_bound() {
    let mut content = (*sts2_content::standard_registry()).clone();
    let mut defend = content
        .card(&id("CARD.DEFEND_IRONCLAD"))
        .expect("defend")
        .clone();
    let gain = defend.effects.clone();
    defend.effects = gain.clone();
    defend
        .effects
        .push(Command::CardPile(sts2_engine::CardPileCmd::AutoPlayFromDrawPile {
            count: Amount::Fixed(1),
            position: sts2_engine::DrawPilePosition::Top,
            force_exhaust: false,
            upgrade: false,
        }));
    defend.effects.extend(gain);
    content.add_card(defend);
    let mut simulator = ScenarioBuilder::new("LEGION113", id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(3, 3)
        .turn(1, 1)
        .relics(&[id("RELIC.PAELS_LEGION")])
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 100, 100, "SHRINKER_MOVE")
        .card(PileName::Hand, 0, CardFingerprint::base(id("CARD.DEFEND_IRONCLAD")), 1)
        .card(PileName::Draw, 1, CardFingerprint::base(id("CARD.SHRUG_IT_OFF")), 1)
        .card(PileName::Draw, 2, CardFingerprint::base(id("CARD.STRIKE_IRONCLAD")), 1)
        .build(Arc::new(content))
        .expect("scenario");
    play(&mut simulator, "CARD.DEFEND_IRONCLAD");
    let combat = simulator.state().combat.as_ref().expect("a fight");
    assert_eq!(combat.creatures[0].block, 10 + 8 + 10);
    let legion = &simulator.state().run_player.relics[0];
    assert_eq!(legion.counter, 2, "the outer play's finishing starts it");
    assert_eq!(legion.bound_card_play, None);
}
