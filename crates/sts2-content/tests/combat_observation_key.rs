//! The in-fight search key partitions states exactly as the full
//! observation key does, over every state a random walk through a fight
//! reaches, and it is the same key on the same state every time.

use std::collections::HashMap;

use sts2_core::UnlockPresetManifest;
use sts2_engine::{Action, ObservationKey, Simulator};

const SEED: &str = "NLD6VZXP94";

fn first_combat() -> Simulator {
    let preset = UnlockPresetManifest::pinned().unwrap();
    let character = "CHARACTER.IRONCLAD".parse().unwrap();
    let mut simulator =
        sts2_content::standard_run_on_preset_at(SEED, &character, &preset, 0).unwrap();
    for _ in 0..40 {
        if simulator.state().combat.is_some() {
            return simulator;
        }
        let action: Action = simulator
            .legal_actions()
            .iter()
            .find(|action| matches!(action, Action::ChooseCards { cards, .. } if !cards.is_empty()))
            .or_else(|| simulator.legal_actions().first())
            .cloned()
            .unwrap();
        simulator.step_quietly(&action).unwrap();
    }
    panic!("the seed reaches a combat inside forty decisions");
}

#[test]
fn the_combat_key_partitions_a_fight_like_the_full_key() {
    let start = first_combat();
    let mut full_to_fast: HashMap<ObservationKey, ObservationKey> = HashMap::new();
    let mut fast_to_full: HashMap<ObservationKey, ObservationKey> = HashMap::new();
    let mut seen = 0;
    for walk in 0..40_u64 {
        let mut simulator = start.clone();
        let mut pick = walk.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
        for _ in 0..60 {
            if simulator.state().combat.is_none() || simulator.state().terminal.is_some() {
                break;
            }
            let full = simulator.observation_key().unwrap();
            let fast = simulator.combat_observation_key().unwrap();
            assert_eq!(fast, simulator.combat_observation_key().unwrap());
            assert_eq!(*full_to_fast.entry(full).or_insert(fast), fast);
            assert_eq!(*fast_to_full.entry(fast).or_insert(full), full);
            seen += 1;
            let actions = simulator.legal_actions().to_vec();
            if actions.is_empty() {
                break;
            }
            pick ^= pick << 13;
            pick ^= pick >> 7;
            pick ^= pick << 17;
            let action = actions[usize::try_from(pick % actions.len() as u64).unwrap()].clone();
            if simulator.step_quietly(&action).is_err() {
                break;
            }
        }
    }
    assert!(
        full_to_fast.len() > 50,
        "{} distinct states of {seen}",
        full_to_fast.len()
    );
}
