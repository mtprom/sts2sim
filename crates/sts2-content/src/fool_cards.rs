//! Fool pool order and the first content batch, pinned to v0.113.1.
//! Bodies needing arrows, melodies or Stretch fail rather than approximate.

use sts2_engine::{
    Amount, CardDefinition, CardFilter, CardOrigin, CardPileCmd, CardPoolRef, CardRarity, CardType,
    Command, Condition, Count, CreatureSelector, DrawPilePosition, HookCondition, HookEvent,
    ModifierHook, OrbCmd, OrbSource, PileName, PlayerCmd, RelicDefinition, RelicRarity, TargetType,
    TemporaryCost, ValueProps,
};

use crate::cards::{
    attack, block, card, dedefending, draw, exhausting, rarity, striking, sweep, upgrading,
};
use crate::id;

pub(crate) const POOL: [(&str, CardType, CardRarity, i32); 90] = [
    ("CARD.ALL_IS_WELL", CardType::Power, CardRarity::Uncommon, 1),
    ("CARD.ANGUISH", CardType::Attack, CardRarity::Uncommon, 1),
    ("CARD.ARCHERY", CardType::Power, CardRarity::Uncommon, 1),
    ("CARD.ARROW_STAB", CardType::Attack, CardRarity::Common, 1),
    (
        "CARD.BLOSSOM_SHOT",
        CardType::Attack,
        CardRarity::Uncommon,
        2,
    ),
    (
        "CARD.BLOT_OUT_THE_SKY",
        CardType::Skill,
        CardRarity::Uncommon,
        1,
    ),
    ("CARD.BOUNCE", CardType::Skill, CardRarity::Rare, 1),
    ("CARD.BOW_SMACK", CardType::Attack, CardRarity::Common, 1),
    ("CARD.BRACE", CardType::Skill, CardRarity::Uncommon, 0),
    ("CARD.BRIGHTEN_UP", CardType::Skill, CardRarity::Rare, 1),
    ("CARD.CHO_BARBS", CardType::Skill, CardRarity::Common, 1),
    ("CARD.CHO_CHORUS", CardType::Skill, CardRarity::Rare, 3),
    ("CARD.CHO_FIRM", CardType::Skill, CardRarity::Common, 1),
    ("CARD.CHO_KI", CardType::Skill, CardRarity::Uncommon, 1),
    ("CARD.CHO_LIMBS", CardType::Skill, CardRarity::Uncommon, 2),
    ("CARD.CHO_READY", CardType::Skill, CardRarity::Common, 1),
    ("CARD.CODA", CardType::Attack, CardRarity::Rare, 6),
    ("CARD.COMPOST", CardType::Power, CardRarity::Uncommon, 0),
    ("CARD.COVERING_FIRE", CardType::Power, CardRarity::Rare, 1),
    ("CARD.DEEP_STRETCH", CardType::Skill, CardRarity::Common, 1),
    ("CARD.DEFEND_FOOL", CardType::Skill, CardRarity::Basic, 1),
    (
        "CARD.DISLIKE_YOU",
        CardType::Attack,
        CardRarity::Uncommon,
        1,
    ),
    ("CARD.DIVINE_SHOT", CardType::Attack, CardRarity::Ancient, 0),
    ("CARD.ECSTASY", CardType::Power, CardRarity::Rare, 2),
    ("CARD.ENCORE", CardType::Skill, CardRarity::Rare, 0),
    ("CARD.ENNUI", CardType::Power, CardRarity::Rare, 1),
    ("CARD.ENRAPTURE", CardType::Skill, CardRarity::Uncommon, 1),
    ("CARD.ENSEMBLE", CardType::Skill, CardRarity::Rare, 3),
    (
        "CARD.EPIC_RETRIEVAL",
        CardType::Attack,
        CardRarity::Uncommon,
        2,
    ),
    (
        "CARD.EXPLOSIVE_ARROWS",
        CardType::Power,
        CardRarity::Rare,
        1,
    ),
    ("CARD.FLORESCENCE", CardType::Skill, CardRarity::Rare, 0),
    ("CARD.GROWTH", CardType::Skill, CardRarity::Uncommon, 1),
    (
        "CARD.HARMONIC_SHIELD",
        CardType::Skill,
        CardRarity::Uncommon,
        1,
    ),
    ("CARD.HEAVY_SHOT", CardType::Attack, CardRarity::Common, 2),
    (
        "CARD.HIDDEN_ROOTS",
        CardType::Power,
        CardRarity::Uncommon,
        1,
    ),
    (
        "CARD.HOBBLING_SHOT",
        CardType::Attack,
        CardRarity::Uncommon,
        1,
    ),
    ("CARD.HONING", CardType::Power, CardRarity::Rare, 3),
    ("CARD.JEST", CardType::Skill, CardRarity::Common, 2),
    ("CARD.KI_CHANT", CardType::Skill, CardRarity::Common, 1),
    ("CARD.KI_HOP", CardType::Attack, CardRarity::Common, 1),
    ("CARD.KI_REST", CardType::Skill, CardRarity::Uncommon, 2),
    ("CARD.KI_VERSE", CardType::Skill, CardRarity::Rare, 1),
    ("CARD.KI_WHIP", CardType::Attack, CardRarity::Common, 1),
    ("CARD.LOCKED_IN", CardType::Skill, CardRarity::Uncommon, 1),
    ("CARD.FOOLISH_SHOT", CardType::Attack, CardRarity::Basic, 1),
    ("CARD.LOST_LUTE", CardType::Power, CardRarity::Ancient, 3),
    ("CARD.LULLABY", CardType::Power, CardRarity::Rare, 0),
    ("CARD.MELODIC_SHOT", CardType::Attack, CardRarity::Rare, 1),
    ("CARD.MOTIF", CardType::Power, CardRarity::Uncommon, 2),
    (
        "CARD.MOURNING_BLOW",
        CardType::Attack,
        CardRarity::Common,
        0,
    ),
    ("CARD.MYTHIC_FORM", CardType::Power, CardRarity::Rare, 3),
    ("CARD.NAIL", CardType::Attack, CardRarity::Rare, 2),
    ("CARD.NOCK", CardType::Skill, CardRarity::Common, 0),
    (
        "CARD.PUNCH_OF_JOY",
        CardType::Attack,
        CardRarity::Uncommon,
        2,
    ),
    ("CARD.QUICK_SHOT", CardType::Attack, CardRarity::Common, 0),
    ("CARD.QUIETUS", CardType::Skill, CardRarity::Rare, 1),
    ("CARD.QUIVER", CardType::Power, CardRarity::Rare, 1),
    ("CARD.RAIN_SHOT", CardType::Attack, CardRarity::Common, 1),
    ("CARD.RAPID_SHOT", CardType::Attack, CardRarity::Uncommon, 1),
    ("CARD.RECALL", CardType::Skill, CardRarity::Common, 1),
    ("CARD.REPEAT_SHOT", CardType::Attack, CardRarity::Rare, 0),
    (
        "CARD.RHYTHM_STRIKE",
        CardType::Attack,
        CardRarity::Uncommon,
        0,
    ),
    ("CARD.RUMMAGE", CardType::Skill, CardRarity::Uncommon, 1),
    ("CARD.SELF_DEPRECATE", CardType::Skill, CardRarity::Rare, 1),
    (
        "CARD.SERRATED_ARROWS",
        CardType::Power,
        CardRarity::Uncommon,
        1,
    ),
    ("CARD.SHOW_OFF", CardType::Skill, CardRarity::Rare, 1),
    ("CARD.SIMPLETON", CardType::Power, CardRarity::Rare, 2),
    ("CARD.SKY_SHOT", CardType::Attack, CardRarity::Rare, 1),
    ("CARD.SONG_PATH", CardType::Power, CardRarity::Uncommon, 1),
    (
        "CARD.SORROWFUL_STRIKE",
        CardType::Attack,
        CardRarity::Common,
        1,
    ),
    (
        "CARD.STEADY_CADENCE",
        CardType::Attack,
        CardRarity::Uncommon,
        1,
    ),
    ("CARD.STRIKE_FOOL", CardType::Attack, CardRarity::Basic, 1),
    ("CARD.TIMBER", CardType::Attack, CardRarity::Uncommon, 3),
    ("CARD.TRICK_SHOT", CardType::Attack, CardRarity::Uncommon, 1),
    ("CARD.TUNE", CardType::Power, CardRarity::Uncommon, 2),
    ("CARD.UPROOT", CardType::Skill, CardRarity::Uncommon, 1),
    (
        "CARD.VIOLENT_CHORD",
        CardType::Power,
        CardRarity::Uncommon,
        1,
    ),
    ("CARD.VITRIOL", CardType::Skill, CardRarity::Uncommon, 0),
    ("CARD.WAH_BLAST", CardType::Attack, CardRarity::Uncommon, 2),
    ("CARD.WAH_CHO", CardType::Attack, CardRarity::Uncommon, 0),
    ("CARD.WAH_CHO_KI", CardType::Attack, CardRarity::Rare, 1),
    ("CARD.WAH_HAMMER", CardType::Attack, CardRarity::Common, 1),
    ("CARD.WAH_KI", CardType::Attack, CardRarity::Uncommon, 1),
    ("CARD.WAH_SYMPHONY", CardType::Attack, CardRarity::Rare, 2),
    ("CARD.WAH_SWEEP", CardType::Attack, CardRarity::Common, 1),
    ("CARD.WAH_TOUCH", CardType::Attack, CardRarity::Common, 1),
    (
        "CARD.WHISTLING_ARROWS",
        CardType::Skill,
        CardRarity::Uncommon,
        0,
    ),
    ("CARD.WREST", CardType::Skill, CardRarity::Uncommon, 0),
    ("CARD.YAH", CardType::Power, CardRarity::Rare, 1),
    ("CARD.YOGA", CardType::Skill, CardRarity::Uncommon, 1),
];

// The six multiplayer-only entries and five basics/ancients never enter solo rewards.
pub(crate) const REWARD_POOL: [(&str, CardType, CardRarity, i32); 79] = {
    let mut result = [POOL[0]; 79];
    let mut index = 0;
    let mut output = 0;
    while index < POOL.len() {
        if !matches!(index, 17 | 18 | 20 | 22 | 26 | 27 | 44 | 45 | 63 | 71 | 89) {
            result[output] = POOL[index];
            output += 1;
        }
        index += 1;
    }
    assert!(output == 79);
    result
};

pub(crate) const TOKENS: &[&str] = &[
    "CARD.JOY",
    "CARD.JUDGEMENT",
    "CARD.THE_HIGH_PRIESTESS",
    "CARD.THE_EMPEROR",
    "CARD.THE_EMPRESS",
    "CARD.THE_HIEROPHANT",
    "CARD.THE_CHARIOT",
    "CARD.JUSTICE",
    "CARD.THE_MAGICIAN",
    "CARD.THE_MOON",
    "CARD.THE_TOWER",
];

pub(crate) fn song_bow() -> RelicDefinition {
    RelicDefinition {
        hooks: vec![
            ModifierHook::every_turn(
                HookEvent::AfterRoomEntered,
                vec![Command::Unsupported(id("RELIC.SONG_BOW"))],
            )
            .when(HookCondition::InACombatRoom),
        ],
        ..crate::relics::obtained_only("RELIC.SONG_BOW", RelicRarity::Starter)
    }
}

fn generate(model: &str, count: Amount, pile: PileName, upgraded: bool) -> Command {
    Command::CardPile(CardPileCmd::AddGeneratedToCombat {
        card: CardOrigin::Model(id(model)),
        count,
        pile,
        upgraded,
        enchantment: None,
        ethereal: false,
        retain: false,
        temporary_cost: None,
        position: DrawPilePosition::Bottom,
        grown_by: Amount::Fixed(0),
    })
}

fn sadness(count: i32) -> Command {
    generate("CARD.SADNESS", Amount::Fixed(count), PileName::Hand, false)
}

fn energy(amount: i32) -> Command {
    Command::Player(PlayerCmd::GainEnergy(Amount::Fixed(amount)))
}

fn timber(damage: i32, splash: i32) -> Command {
    Command::Strike {
        amount: Amount::Fixed(damage),
        hits: Amount::Fixed(1),
        target: CreatureSelector::Target,
        props: ValueProps::default(),
        then: vec![Command::When {
            condition: Condition::TheSwingKilledSomething,
            then: vec![sweep(splash, 1)],
            otherwise: Vec::new(),
        }],
    }
}

pub(crate) fn cards() -> Vec<CardDefinition> {
    let mut implemented = first_ten();
    implemented.extend(second_ten());
    let mut result: Vec<_> = POOL
        .iter()
        .filter(|(model, _, _, _)| !implemented.iter().any(|card| card.model_id == id(model)))
        .map(|(model, kind, rank, cost)| {
            let target = if *kind == CardType::Attack {
                TargetType::AnyEnemy
            } else {
                TargetType::SelfTarget
            };
            rarity(
                card(
                    model,
                    *kind,
                    target,
                    *cost,
                    vec![Command::Unsupported(id(model))],
                ),
                *rank,
            )
        })
        .collect();
    result.extend(implemented);
    result
}

fn first_ten() -> Vec<CardDefinition> {
    let mut status = card(
        "CARD.SADNESS",
        CardType::Status,
        TargetType::None,
        -1,
        Vec::new(),
    );
    status.unplayable = true;
    status.retain = true;
    vec![
        striking(upgrading(
            card(
                "CARD.STRIKE_FOOL",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                vec![attack(6)],
            ),
            vec![attack(9)],
        )),
        defending(upgrading(
            card(
                "CARD.DEFEND_FOOL",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![block(5)],
            ),
            vec![block(8)],
        )),
        rarity(
            exhausting(upgrading(
                card(
                    "CARD.JOY",
                    CardType::Skill,
                    TargetType::SelfTarget,
                    0,
                    vec![energy(1), draw(1)],
                ),
                vec![energy(2), draw(1)],
            )),
            CardRarity::Token,
        ),
        rarity(status, CardRarity::Status),
        upgrading(
            card(
                "CARD.JEST",
                CardType::Skill,
                TargetType::SelfTarget,
                2,
                vec![block(14), sadness(1)],
            ),
            vec![block(18), sadness(1)],
        ),
        upgrading(
            card(
                "CARD.RUMMAGE",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![draw(3), sadness(1)],
            ),
            vec![draw(4), sadness(1)],
        ),
        striking(upgrading(
            card(
                "CARD.SORROWFUL_STRIKE",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                vec![attack(13), sadness(2)],
            ),
            vec![attack(17), sadness(2)],
        )),
        upgrading(
            card(
                "CARD.VITRIOL",
                CardType::Skill,
                TargetType::AnyEnemy,
                0,
                vec![debuff("POWER.WEAK_POWER", 2), sadness(1)],
            ),
            vec![debuff("POWER.WEAK_POWER", 3), sadness(1)],
        ),
        exhausting(upgrading(
            card(
                "CARD.BRIGHTEN_UP",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![
                    sadness(2),
                    generate("CARD.JOY", Amount::Fixed(2), PileName::Hand, false),
                ],
            ),
            vec![
                sadness(2),
                generate("CARD.JOY", Amount::Fixed(3), PileName::Hand, false),
            ],
        )),
        upgrading(
            card(
                "CARD.TIMBER",
                CardType::Attack,
                TargetType::AnyEnemy,
                3,
                vec![timber(20, 30)],
            ),
            vec![timber(25, 35)],
        ),
    ]
}

fn tarot(
    model: &str,
    kind: CardType,
    target: TargetType,
    base: Vec<Command>,
    upgrade: Vec<Command>,
) -> CardDefinition {
    rarity(
        exhausting(upgrading(card(model, kind, target, 0, base), upgrade)),
        CardRarity::Token,
    )
}

fn hierophant(count: i32) -> Command {
    Command::CardPile(CardPileCmd::AddGeneratedFromPool {
        pool: CardPoolRef::Named(id("CARD_POOL.COLORLESS_CARD_POOL")),
        count: Amount::Fixed(count),
        distinct: true,
        filter: CardFilter::Any,
        pile: PileName::Hand,
        position: DrawPilePosition::Bottom,
        excludes_source: false,
        upgraded: false,
        temporary_cost: Some(TemporaryCost {
            cost: -1,
            relative: true,
            clears_when_turn_ends: true,
            clears_when_played: true,
            reduce_only: false,
            frees_the_star_cost: false,
        }),
        ethereal: false,
        rolled_one_at_a_time: false,
    })
}

fn magician(count: i32) -> Vec<Command> {
    vec![
        Command::Orb(OrbCmd::AddSlots(Amount::Fixed(count))),
        Command::Repeat {
            times: Amount::Fixed(count),
            then: vec![Command::Orb(OrbCmd::Channel(OrbSource::RandomOnCombatOrbs))],
        },
    ]
}

fn second_ten() -> Vec<CardDefinition> {
    let mut priestess = tarot(
        "CARD.THE_HIGH_PRIESTESS",
        CardType::Skill,
        TargetType::SelfTarget,
        vec![draw(4)],
        vec![draw(5)],
    );
    priestess.retain = true;
    vec![
        tarot(
            "CARD.JUDGEMENT",
            CardType::Skill,
            TargetType::AnyEnemy,
            vec![Command::Unsupported(id("CARD.JUDGEMENT"))],
            vec![Command::Unsupported(id("CARD.JUDGEMENT"))],
        ),
        priestess,
        tarot(
            "CARD.THE_EMPEROR",
            CardType::Skill,
            TargetType::SelfTarget,
            vec![energy(7)],
            vec![energy(9)],
        ),
        tarot(
            "CARD.THE_EMPRESS",
            CardType::Skill,
            TargetType::AnyEnemy,
            vec![
                debuff("POWER.WEAK_POWER", 3),
                debuff("POWER.VULNERABLE_POWER", 3),
            ],
            vec![
                debuff("POWER.WEAK_POWER", 4),
                debuff("POWER.VULNERABLE_POWER", 4),
            ],
        ),
        tarot(
            "CARD.THE_HIEROPHANT",
            CardType::Skill,
            TargetType::SelfTarget,
            vec![hierophant(3)],
            vec![hierophant(4)],
        ),
        tarot(
            "CARD.THE_CHARIOT",
            CardType::Attack,
            TargetType::AllEnemies,
            vec![sweep(25, 1)],
            vec![sweep(30, 1)],
        ),
        tarot(
            "CARD.JUSTICE",
            CardType::Attack,
            TargetType::AnyEnemy,
            vec![attack(30)],
            vec![attack(35)],
        ),
        tarot(
            "CARD.THE_MAGICIAN",
            CardType::Skill,
            TargetType::SelfTarget,
            magician(3),
            magician(4),
        ),
        tarot(
            "CARD.THE_MOON",
            CardType::Skill,
            TargetType::SelfTarget,
            vec![generate(
                "CARD.SHIV",
                Amount::counting(Count::SpaceLeftInHand),
                PileName::Hand,
                false,
            )],
            vec![generate(
                "CARD.SHIV",
                Amount::counting(Count::SpaceLeftInHand),
                PileName::Hand,
                true,
            )],
        ),
        tarot(
            "CARD.THE_TOWER",
            CardType::Attack,
            TargetType::AnyEnemy,
            vec![Command::Unsupported(id("CARD.THE_TOWER"))],
            vec![Command::Unsupported(id("CARD.THE_TOWER"))],
        ),
    ]
}
