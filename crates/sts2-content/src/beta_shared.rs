//! Shared event content introduced by beta v0.113.1.

use sts2_engine::{
    Amount, CardCmd, CardCostRule, CardDefinition, CardOrigin, CardPileCmd, CardRarity,
    CardSelector, CardType, Command, Condition, Count, CreatureCmd, CreatureSelector,
    DrawPilePosition, HookCondition, HookEvent, HookTiming, ModifierHook, PileName, PlayerCmd,
    PowerCmd, PowerDefinition, PowerRef, PowerType, RelicDefinition, RelicRarity, TargetType,
    ValueProps,
};

use crate::cards::{attack, block, buff, card, rarity, sweep, unplayable, upgrading};
use crate::{id, powers, relics};

fn dazed(count: i32) -> Command {
    Command::CardPile(CardPileCmd::AddGeneratedToCombat {
        enchantment: None,
        card: CardOrigin::Model(id("CARD.DAZED")),
        count: Amount::Fixed(count),
        pile: PileName::Draw,
        upgraded: false,
        ethereal: false,
        retain: false,
        temporary_cost: None,
        position: DrawPilePosition::Random,
        grown_by: Amount::Fixed(0),
    })
}

pub(crate) fn cards() -> Vec<CardDefinition> {
    let mut sadness = unplayable(card(
        "CARD.SADNESS",
        CardType::Status,
        TargetType::None,
        -1,
        Vec::new(),
    ));
    sadness.retain = true;
    let mut shield = upgrading(
        card(
            "CARD.MIGHTY_SHIELD",
            CardType::Skill,
            TargetType::SelfTarget,
            1,
            vec![block(20), dazed(3)],
        ),
        vec![block(25), dazed(3)],
    );
    shield.gains_block = true;
    let mut dagger = upgrading(
        card(
            "CARD.DINGY_DAGGER",
            CardType::Attack,
            TargetType::AnyEnemy,
            0,
            vec![attack(7)],
        ),
        vec![attack(11)],
    );
    // TODO(package 4): model further damage values and widen the u8 fingerprint.
    // Keep later upgrades eligible so attempts fail explicitly, not disappear.
    dagger
        .upgrade
        .as_mut()
        .expect("the first upgrade exists")
        .repeatable = true;
    vec![
        my_shinies(),
        rarity(dagger, CardRarity::Event),
        rarity(
            upgrading(
                card(
                    "CARD.GOD_BYRD",
                    CardType::Skill,
                    TargetType::None,
                    0,
                    vec![buff("POWER.GOD_BYRD_POWER", 30)],
                ),
                vec![buff("POWER.GOD_BYRD_POWER", 40)],
            ),
            CardRarity::Event,
        ),
        rarity(shield, CardRarity::Event),
        rarity(
            upgrading(
                card(
                    "CARD.MIGHTY_WEAPON",
                    CardType::Attack,
                    TargetType::AllEnemies,
                    1,
                    vec![sweep(25, 1), dazed(2)],
                ),
                vec![sweep(30, 1), dazed(2)],
            ),
            CardRarity::Event,
        ),
        rarity(sadness, CardRarity::Status),
    ]
}

fn my_shinies() -> CardDefinition {
    let hits = |gold_per_hit| {
        crate::cards::multi_attack(
            Amount::Fixed(1),
            Amount::Quotient {
                of: Box::new(Amount::counting(Count::PlayerGold)),
                by: gold_per_hit,
            },
        )
    };
    rarity(
        upgrading(
            card(
                "CARD.MY_SHINIES",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                vec![hits(20)],
            ),
            vec![hits(10)],
        ),
        CardRarity::Rare,
    )
}

pub(crate) fn powers() -> Vec<PowerDefinition> {
    vec![
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterPlayerTurnStart,
                    vec![
                        Command::Creature(CreatureCmd::Damage {
                            target: CreatureSelector::AllOpponents,
                            amount: Amount::counting(Count::ListeningPowerAmount),
                            props: ValueProps::UNPOWERED,
                        }),
                        Command::Power(PowerCmd::Remove {
                            power: PowerRef::listening(),
                        }),
                    ],
                )
                .when(HookCondition::ListeningPowerStoodAtTurnStart),
            ],
            ..powers::counter("POWER.GOD_BYRD_POWER", PowerType::Buff)
        },
        PowerDefinition {
            card_cost_rule: Some(CardCostRule::FreeEnergyInHandOrPlay),
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::BeforeCardPlayed,
                    vec![Command::Power(PowerCmd::Decrement {
                        power: PowerRef::listening(),
                    })],
                )
                .when(HookCondition::OwnerPlayedTheCard),
            ],
            ..powers::counter("POWER.FREE_CARD_POWER", PowerType::Buff)
        },
        // TODO(package 4): ModifyPreBlockDamageTarget and BeforeWuppy loss
        // ordering; retain the loyalty power on the dead companion.
        PowerDefinition {
            survives_owner_death: true,
            owner_removed_from_combat_on_death: false,
            ..powers::single("POWER.WUPPYS_LOYALTY_POWER", PowerType::Buff)
        },
    ]
}

fn heal(amount: Amount) -> Command {
    Command::Creature(CreatureCmd::Heal {
        target: CreatureSelector::Owner,
        amount,
    })
}

#[allow(
    clippy::too_many_lines,
    reason = "one declarative entry per beta event relic"
)]
pub(crate) fn relics() -> Vec<RelicDefinition> {
    vec![
        RelicDefinition {
            latch_property: Some("WasUsed".to_owned()),
            hooks: vec![ModifierHook::every_turn(
                HookEvent::BeforeCombatStart,
                vec![Command::When {
                    condition: Condition::All(vec![
                        Condition::CurrentRoomIs(sts2_engine::ResolvedRoom::Boss),
                        Condition::Not(Box::new(Condition::ListeningRelicIsLatched)),
                    ]),
                    then: vec![
                        heal(Amount::counting(Count::OwnerMaxHp)),
                        Command::Relic(sts2_engine::RelicCmd::SetListeningLatch(true)),
                    ],
                    otherwise: Vec::new(),
                }],
            )],
            ..relics::obtained_only("RELIC.BATH_ROBES", RelicRarity::Event)
        },
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterPlayerTurnStartLate,
                    vec![heal(Amount::Fixed(8))],
                )
                .timed(HookTiming::FirstTurnOnly),
            ],
            ..relics::obtained_only("RELIC.BATH_TOWEL", RelicRarity::Event)
        },
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::BeforeSideTurnEnd,
                    vec![Command::When {
                        condition: Condition::CountAtLeast {
                            count: Count::CardsInPile(PileName::Hand),
                            at_least: 1,
                        },
                        then: vec![Command::Creature(CreatureCmd::Damage {
                            target: CreatureSelector::RandomOpponent,
                            amount: Amount::counting(Count::CardsInPile(PileName::Hand)),
                            props: ValueProps::UNPOWERED,
                        })],
                        otherwise: Vec::new(),
                    }],
                )
                .when(HookCondition::OwnerIsParticipant),
            ],
            ..relics::obtained_only("RELIC.OBSIDIAN_VICE", RelicRarity::Event)
        },
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::BeforeSideTurnStart,
                    vec![Command::When {
                        condition: Condition::CurrentRoomIs(sts2_engine::ResolvedRoom::Elite),
                        then: vec![crate::cards::apply_power(
                            "POWER.WEAK_POWER",
                            CreatureSelector::AllOpponents,
                            Amount::Fixed(2),
                        )],
                        otherwise: Vec::new(),
                    }],
                )
                .when(HookCondition::OwnerIsParticipant)
                .timed(HookTiming::FirstTurnOnly),
            ],
            ..relics::obtained_only("RELIC.TORMENTOUS_COLLAR", RelicRarity::Event)
        },
        RelicDefinition {
            adds_pet: true,
            spawns_pets: true,
            has_upon_pickup_effect: true,
            on_obtained: vec![Command::Card(CardCmd::TransformDeckCards {
                cards: CardSelector::DeckCardsOfModel(id("CARD.BYRD_SWOOP")),
                replacement: sts2_engine::CardTransformReplacement::Named {
                    model: id("CARD.GOD_BYRD"),
                    keeps_upgrade: true,
                    keeps_enchantment: false,
                },
                one_at_a_time: false,
            })],
            hooks: vec![ModifierHook::every_turn(
                HookEvent::BeforeCombatStart,
                vec![Command::Player(PlayerCmd::AddPet {
                    monster: id("MONSTER.BYRDINE_MONSTER"),
                })],
            )],
            ..relics::obtained_only("RELIC.BYRDINE", RelicRarity::Event)
        },
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::ModifyAttackHitCount,
                    vec![Command::Modify(sts2_engine::ModifierCmd::Add(
                        Amount::Fixed(1),
                    ))],
                )
                .when(HookCondition::OwnersMultiHitAttackCard),
            ],
            ..relics::obtained_only("RELIC.THUMBSCREW", RelicRarity::Event)
        },
        RelicDefinition {
            counter_property: Some("CurrentHp".to_owned()),
            second_counter_property: Some("MaxHp".to_owned()),
            counter_kind: Some(sts2_engine::CounterKind::Ledger),
            initial_counter: 60,
            on_obtained: vec![Command::Relic(
                sts2_engine::RelicCmd::SetListeningSecondCounter(Amount::Fixed(60)),
            )],
            adds_pet: true,
            adds_rest_site_option: Some(sts2_engine::RestSiteOffer::of(
                sts2_engine::RestSiteOption::Restore,
            )),
            // TODO(package 4): spawn using CurrentHp/MaxHp, synchronize HP
            // changes, and redirect powered attacks before the owner's block.
            hooks: vec![ModifierHook::every_turn(
                HookEvent::BeforeCombatStart,
                vec![Command::Unsupported(id("POWER.WUPPYS_LOYALTY_POWER"))],
            )],
            ..relics::obtained_only("RELIC.WUPPY", RelicRarity::Event)
        },
        RelicDefinition {
            hooks: [
                HookCondition::OwnerDealsPoweredDamage,
                HookCondition::OwnersPetDealsPoweredDamage,
            ]
            .into_iter()
            .map(|condition| {
                ModifierHook::every_turn(
                    HookEvent::AfterDamageGiven,
                    vec![Command::When {
                        condition: Condition::CountAtLeast {
                            count: Count::TotalDamageOfTheBlow,
                            at_least: 30,
                        },
                        then: vec![buff("POWER.STRENGTH_POWER", 2)],
                        otherwise: Vec::new(),
                    }],
                )
                .when(condition)
            })
            .collect(),
            ..relics::obtained_only("RELIC.MIGHTY_WINGS", RelicRarity::Event)
        },
        // Fool-only Aim is outside the supported five-character baseline.
        RelicDefinition {
            on_obtained: vec![Command::Unsupported(id("RELIC.STORYTELLER"))],
            ..relics::obtained_only("RELIC.STORYTELLER", RelicRarity::Starter)
        },
    ]
}

pub(crate) fn monsters() -> Vec<sts2_engine::MonsterDefinition> {
    let mut byrdine = crate::monsters::byrdpip();
    byrdine.model_id = id("MONSTER.BYRDINE_MONSTER");
    let mut wuppy = crate::monsters::byrdpip();
    wuppy.model_id = id("MONSTER.WUPPY");
    wuppy.min_initial_hp = 50;
    wuppy.max_initial_hp = 50;
    vec![byrdine, wuppy]
}
