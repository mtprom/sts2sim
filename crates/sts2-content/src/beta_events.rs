//! Shared beta event pages and their eligibility.

use sts2_engine::{
    Amount, CardFingerprint, CardPileCmd, Command, Count, EventAmount, EventCondition,
    EventDefinition, EventEffect, EventGeneration,
};

use crate::{events::option, id};

fn add(model: &str) -> EventEffect {
    EventEffect::AddCard(CardFingerprint::base(id(model)))
}

fn event(model: &str, allowed: EventCondition, generation: EventGeneration) -> EventDefinition {
    EventDefinition {
        model_id: id(model),
        combat_layout: None,
        heals_on_entry: false,
        allowed,
        vars: Vec::new(),
        generation,
    }
}

pub(crate) fn museum_of_torture() -> EventDefinition {
    // AncientDialogue is the existing one-event-stream-draw page generator:
    // here each candidate page carries the relic rolled by CalculateVars.
    let dialogues = [
        "RELIC.OBSIDIAN_VICE",
        "RELIC.THUMBSCREW",
        "RELIC.TORMENTOUS_COLLAR",
    ]
    .into_iter()
    .map(|relic| {
        vec![option(
            "MUSEUM_OF_TORTURE.pages.INITIAL.options.CONTINUE",
            "Continue",
            vec![EventEffect::SetPage(vec![option(
                "MUSEUM_OF_TORTURE.pages.LOCKED.options.CONTINUE",
                "Continue",
                vec![EventEffect::SetPage(vec![
                    option(
                        "MUSEUM_OF_TORTURE.pages.DISPLAY.options.TAKE",
                        "Take",
                        vec![
                            EventEffect::ObtainRelic(id(relic)),
                            // stream: port-v0113-foundation (non_combat_generation).
                            EventEffect::Run(vec![Command::CardPile(
                                CardPileCmd::AddCursesToDeck {
                                    pool: id("CARD_POOL.CURSE_CARD_POOL"),
                                    count: 1,
                                },
                            )]),
                        ],
                    ),
                    option(
                        "MUSEUM_OF_TORTURE.pages.DISPLAY.options.BREAK_OUT",
                        "Break out",
                        vec![EventEffect::LoseMaxHp(EventAmount::Fixed(10))],
                    ),
                ])],
            )])],
        )]
    })
    .collect();
    event(
        "EVENT.MUSEUM_OF_TORTURE",
        EventCondition::ActIndexIs(1),
        EventGeneration::AncientDialogue { dialogues },
    )
}

pub(crate) fn shrine_of_the_warlord() -> EventDefinition {
    event(
        "EVENT.SHRINE_OF_THE_WARLORD",
        EventCondition::ActAncientIsNot(id("EVENT.TANX")),
        EventGeneration::Fixed(vec![
            option(
                "SHRINE_OF_THE_WARLORD.pages.INITIAL.options.TAKE_WEAPON",
                "Take weapon",
                vec![add("CARD.MIGHTY_WEAPON")],
            ),
            option(
                "SHRINE_OF_THE_WARLORD.pages.INITIAL.options.TAKE_SHIELD",
                "Take shield",
                vec![add("CARD.MIGHTY_SHIELD")],
            ),
            option(
                "SHRINE_OF_THE_WARLORD.pages.INITIAL.options.TAKE_WINGS",
                "Take wings",
                vec![
                    add("CARD.CLUMSY"),
                    EventEffect::ObtainRelic(id("RELIC.MIGHTY_WINGS")),
                ],
            ),
        ]),
    )
}

pub(crate) fn thermae_nonupeipe() -> EventDefinition {
    let dialogues = [
        "RELIC.BLESSED_ANTLER",
        "RELIC.BRILLIANT_SCARF",
        "RELIC.DELICATE_FROND",
        "RELIC.DIAMOND_DIADEM",
        "RELIC.FUR_COAT",
        "RELIC.GLITTER",
        "RELIC.JEWELRY_BOX",
        "RELIC.SIGNET_RING",
        "RELIC.BEAUTIFUL_BRACELET",
    ]
    .into_iter()
    .map(|relic| {
        vec![
            option(
                "THERMAE_NONUPEIPE.pages.INITIAL.options.THERMAL_BATHS",
                "Thermal baths",
                vec![
                    EventEffect::LoseGold(EventAmount::Fixed(88)),
                    EventEffect::ObtainRelic(id("RELIC.BATH_TOWEL")),
                ],
            ),
            option(
                "THERMAE_NONUPEIPE.pages.INITIAL.options.PRIVATE_OASIS",
                "Private oasis",
                vec![
                    EventEffect::Run(vec![Command::Player(sts2_engine::PlayerCmd::LoseGold(
                        Amount::counting(Count::PlayerGold),
                    ))]),
                    EventEffect::ObtainRelic(id("RELIC.BATH_ROBES")),
                ],
            ),
            option(
                "THERMAE_NONUPEIPE.pages.INITIAL.options.DRINK_WATERS",
                "Drink waters",
                vec![
                    EventEffect::LoseHp(EventAmount::Fixed(31)),
                    add("CARD.GUILTY"),
                    add("CARD.GUILTY"),
                    add("CARD.GUILTY"),
                    EventEffect::ObtainRelic(id(relic)),
                ],
            ),
        ]
    })
    .collect();
    event(
        "EVENT.THERMAE_NONUPEIPE",
        EventCondition::All(vec![
            EventCondition::GoldAtLeast(176),
            EventCondition::CurrentHpAtMostFraction {
                numerator: 3,
                denominator: 4,
            },
            EventCondition::ActAncientIsNot(id("EVENT.NONUPEIPE")),
        ]),
        EventGeneration::AncientDialogue { dialogues },
    )
}

pub(crate) fn blocked_events() -> Vec<EventDefinition> {
    vec![
        // TODO(package 4): relic with preset persistent HP (60 or 30),
        // CurrentHp * 0.05 damage rounded up, and Wuppy interception/restore.
        event(
            "EVENT.WUPPY_IN_TROUBLE",
            EventCondition::NoEventPet,
            EventGeneration::Unsupported,
        ),
    ]
}

pub(crate) fn waterfall_of_obvious_secrets() -> EventDefinition {
    let choices = [
        (
            "BE_NICE",
            EventCondition::Always,
            vec![add("CARD.DINGY_DAGGER")],
        ),
        (
            "BEAT_UP",
            EventCondition::Always,
            vec![
                // stream: port-v0113-foundation (event RNG relic rarity).
                EventEffect::ObtainRolledRelic,
                add("CARD.GUILTY"),
                add("CARD.GUILTY"),
            ],
        ),
        (
            "NEGOTIATE",
            EventCondition::GoldAtLeast(70),
            vec![
                EventEffect::LoseGold(EventAmount::Fixed(70)),
                EventEffect::Run(vec![Command::CardPile(
                    CardPileCmd::AddCreatedCardsToDeck {
                        creation: sts2_engine::CardCreation {
                            pools: vec![sts2_engine::CardPoolRef::OwnCharacter],
                            filter: sts2_engine::CardFilter::OfRarity(
                                sts2_engine::CardRarity::Rare,
                            ),
                            odds: sts2_engine::RarityOdds::Uniform,
                            from_an_encounter: false,
                            count: 1,
                            flags: sts2_engine::CardCreationFlags {
                                no_upgrade_roll: true,
                                ..sts2_engine::CardCreationFlags::default()
                            },
                        },
                    },
                )]),
            ],
        ),
    ]
    .into_iter()
    .map(|(key, available, effects)| sts2_engine::ConditionalOption {
        available,
        option: option(
            &format!("WATERFALL_OF_OBVIOUS_SECRETS.pages.ENTER.options.{key}"),
            key,
            effects,
        ),
        locked_option_id: format!("WATERFALL_OF_OBVIOUS_SECRETS.pages.ENTER.options.{key}_LOCKED"),
    })
    .collect();
    event(
        "EVENT.WATERFALL_OF_OBVIOUS_SECRETS",
        EventCondition::Always,
        EventGeneration::Fixed(vec![option(
            "WATERFALL_OF_OBVIOUS_SECRETS.pages.INITIAL.options.ENTER",
            "Enter",
            vec![EventEffect::SetConditionalPage(choices)],
        )]),
    )
}
