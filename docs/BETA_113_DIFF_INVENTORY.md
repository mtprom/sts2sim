# v0.111.0 to v0.113.1 beta: verified difference inventory

Audit date: 2026-11-18. Target: the installed Steam public beta, v0.113.1, commit `d09d48e5`, model hash `1748280997`. The old side is the v0.111.0 beta binary that the simulator at `6fff624` was ported to (commit `41cef1ea`, model hash `1568834832`). Both sides are game code, so unlike the 0.111 inventory this one compares game against game, and then checks the simulator against the new game.

This is an inventory, not a port. Nothing in the simulator has changed yet.

## How this was checked

1. Both `sts2.dll` files were decompiled outside Git with the same decompiler (ICSharpCode.Decompiler 9.1, whole-project mode), against the reference assemblies the game itself ships in `data_sts2_macos_arm64`. The game was not launched.
   - v0.113.1: SHA-256 `a671e9efb86a7b80405133105665867b053c787670d475b4347cf5ccc15cb5fb`, file dated 2026-11-14.
   - v0.111.0: SHA-256 `9cb4f1ad8c9f284aa8fec3122ffd6d780bbf543d875c817abdd12ff63fbf12b4`.
2. The two decompiled trees were diffed directly. 459 changed files and 322 new files sit outside UI, audio, platform and multiplayer-transport namespaces; every one of them was read. Changes that only rename a variable, add a sound or animation, or thread a new parameter through (for example `AfterDeath` and `Kill` gaining a `CardModel? source`, `LoseMaxHp(isFromCard)` becoming `LoseMaxHp(cardSource)`, `RepeatVar` becoming `HitCountVar`) are listed once as refactors and not per file.
3. `tools/beta_diff/run_all.sh` (from SpireFish) was run twice against the simulator at `6fff624`: once with the 0.111 tree, to get the baseline of known script noise, and once with the 0.113.1 tree. Only the difference between the two runs is new. The scripts are heuristics and missed several changes the direct diff found (Infested Prism, Expertise, Ringing Triangle, and every RNG stream move), so the direct diff is the ground truth here.
4. Version constants come from the game's own startup log, not from code: the build reads `Resources/release_info.json` (version, commit, date), and `ModelIdSerializationCache` logs the model hash. Two v0.113.1 launches log `Release Version: v0.113.1`, `Release Commit: d09d48e5`, `Release Date: 11/14/2026`, and `Categories: 21 Entries: 1864 Epochs: 66 Properties: 53 Hash: 1748280997`. The hash algorithm (`ModelIdSerializationCache`) is unchanged; 0.111 logged 20 / 1658 / 57 / 47. v0.113.0 (commit `16ac0fe3`) logged the same hash, so 0.113.0 and 0.113.1 share an ID set.

## Summary

The patch is large. Ordered by what it costs a replay simulator:

- **Every shuffle draws differently.** `UnstableShuffle` (and so `StableShuffle`, which the combat draw pile uses) now draws one `NextUnsignedLong` from the stream it is given, seeds a fresh `Rng` with it, and runs Fisher-Yates on that child generator. Each shuffle advances the parent stream by exactly one draw instead of n-1.
- **The draw pile sorts differently before it shuffles.** `CardModel.CompareTo` gains tie-breakers after ID and upgrade level: enchantment ID, affliction ID, resolved energy cost, base replay count, then every dynamic variable's base value in ordinal key order.
- **A new per-player RNG stream, `non_combat_generation`,** takes over most out-of-combat random picks that used to share `Niche`, `CombatPotionGeneration` or `Rewards`. Several in-combat picks moved off the `Shuffle` stream to `CombatCardSelection`. Merchant card rarity and upgrades now roll on `Shops`, and events pull relics on the event's own RNG, so neither touches the card-reward odds anymore.
- **Merchant generation changed order.** The on-sale slot is drawn last instead of first, and card entries are filled after relics and potions.
- **Map generation changed.** Rest sites are allowed from row 5 (was 6); elites still from row 6. The point-type queue no longer rotates rejected types to the back.
- **The Fool** is a sixth character with 90 pool cards, 23 new tokens, 8 relics, 3 potions, about 40 powers and four new mechanics (arrows, melody tags, Stretch, tarot generation).
- **Two new game modes.** Blitz is a short linear run with its own map, merchant and rewards. Dread adds a per-act "dominion" debuff to every non-boss fight and buffs every boss.
- **Balance changes** for Silent (Expertise, Memento Mori, Speedster, Phantom Blades), Ironclad (Expect a Fight, Forgotten Ritual), Salvo, Minion Dive Bomb, five monsters, five relics and three events. Several 0.113 patch-note items (Stomp, Bullet Time, Nightmare's numbers) have no code difference between the two binaries; the installed 0.111 build already had them, which matches the simulator.

## 1. RNG, seeding and shuffle

| Area | 0.111 | 0.113.1 | Where (game) |
|---|---|---|---|
| `UnstableShuffle(list, rng)` | Fisher-Yates from the top on `rng` | `child = new Rng(rng.NextUnsignedLong())`, Fisher-Yates from the top on `child`. Parent advances by one draw. | `Extensions/ListExtensions.cs` |
| `StableShuffle` | sort, then `UnstableShuffle` | unchanged wrapper, so it inherits the child generator | same |
| `Rng.Shuffle` (in-place on the stream) | existed; used by Neow's Bones and The Architect | removed. Neow's Bones now uses `UnstableShuffle` on `non_combat_generation`; The Architect uses `Rng.Chaotic` (no seeded draw). | `Random/Rng.cs` |
| Card sort key (draw pile, `StableShuffle`) | ID, then upgrade level | ID, upgrade level, enchantment ID, affliction ID (null sorts first), resolved energy cost, base replay count, then dynamic-var base values by ordinal key | `Models/CardModel.cs` `CompareTo`, `Models/ModelId.cs` `Compare` |
| Per-player streams | `rewards`, `shops`, `transformations` | adds `non_combat_generation` (`Rng(playerSeed, "non_combat_generation")`). Old saves are backfilled with fresh streams for missing types. | `Random/PlayerRngSet.cs`, `Entities.Rngs/PlayerRngType.cs` |
| Run streams | 12 | unchanged list; loading now backfills missing streams | `Runs/RunRngSet.cs` |
| `Rng.Chaotic` | seeded from Unix time | seeded from 8 crypto-random bytes (never affects gameplay draws) | `Random/Rng.cs` |
| Card rarity roll | always on the odds object's own RNG (the player's `rewards`) | optional RNG override; merchant passes `shops` | `Odds/CardRarityOdds.cs`, `Factories/CardFactory.cs` |
| Relic rarity roll | `RollRarity(player)` on `rewards` | `RollRarity(rng)`; rewards use `RollRarityForReward(player)` = `rewards`; merchant uses `shops`; events use the event RNG | `Factories/RelicFactory.cs` |

Stream moves, one row per call site. "Before" is the stream 0.111 used.

| Call site | Before | After |
|---|---|---|
| Beat Down, Catastrophe (both picks), Uproar (both picks) | run `Shuffle` | run `CombatCardSelection` |
| Stampede power | run `Shuffle` | run `CombatCardSelection` |
| Entropy power (transform) | run `CombatCardSelection` | run `CombatCardGeneration` |
| Thieving Hopper (card to steal) | run `CombatCardGeneration` | run `CombatCardSelection`; it also no longer targets Quest cards |
| Power Cell | one `StableShuffle` on `CombatCardSelection` | non-Innate cards shuffled first, then Innate ones, two shuffles on the same stream |
| Entropic Brew, Delicate Frond, Phial Holster, Alchemical Coffer | run `CombatPotionGeneration` | player `non_combat_generation` |
| Whetstone, War Paint, War Hammer, Sand Castle, Fragrant Mushroom, Kaleidoscope, Royal Stamp, Beautiful Bracelet, Distinguished Cape, Fishing Rod, Neow's Bones (curse pick) | run `Niche` | player `non_combat_generation` |
| Neow's Bones (list shuffle) | player `rewards`, in-place `Rng.Shuffle` | player `non_combat_generation`, `UnstableShuffle` |
| Cursed Run modifier (curse pick) | run `Niche` | player `non_combat_generation` |
| Astrolabe, Pandora's Box, New Leaf (transforms) | run `Niche` | player `transformations` |
| Wing Charm | run `Niche` | player `rewards` |
| Merchant card rarity and upgrade roll | player `rewards` (odds RNG) and the default upgrade RNG | player `shops` |
| Merchant relic rarity | player `rewards` | player `shops` |
| Battleworn Dummy, Luminous Choir, Ranwid the Elder, Relic Trader, Round Tea Party, This or That, Trial, Unrest Site (relic pulls) | player `rewards` for rarity | the event's own RNG |
| Battleworn Dummy relic reward | rewards RNG | `RelicReward` with the event RNG as override |
| The Architect (vfx shuffle and damage split) | event RNG | `Rng.Chaotic`, so the event RNG is no longer advanced |
| Vakuu options | three `UnstableShuffle`s, take each list's first | one `NextItem` per pool through `GetOptionsFromThreePools` |
| Orobas, Pael, Tezcatara, Vakuu | one `NextItem` per pool | same draws, but options whose relic fails `IsAllowed` are filtered first; if a pool comes up empty the missing options are drawn from the union of the pools. Only Blitz and late-act-3 filtering can trigger it. |
| Darv, Tanx, Nonupeipe | unfiltered | filtered through `AllowedOptions` before drawing |
| Map path pruning | prunes only matching points | counts only `CanBeModified` points; when pruning a non-Unknown type, also appends the Unknown points, `StableShuffle`d on the same RNG |

The 0.113.0 note "events and the merchant no longer affect card reward rolls" is exactly the merchant `shops` override plus the event-RNG relic pulls above.

## 2. Model IDs

Hash `1748280997`: 21 categories, 1864 entries, 66 epochs, 53 saved properties.

- New categories: `Dominion` (12 Dread dominions) and run models. `MultiplayerScalingModel` moves from the removed `Singleton` namespace to the new `RunModel` base, next to `BlitzRunModel` and `DreadRunModel`.
- Cards: +121, none removed. 90 in the Fool pool (6 of them `MultiplayerOnly`), 25 new tokens (the 22 tarot cards `THE_FOOL` ... `THE_WORLD` including `STRENGTH`, `JUSTICE`, `DEATH`, `TEMPERANCE`, `JUDGEMENT`, `WHEEL_OF_FORTUNE`, plus `THE_SHOT`, `JOY`, `AMNESIA`), status `SADNESS`, colorless `MY_SHINIES`, event `DINGY_DAGGER`, `GOD_BYRD`, `MIGHTY_SHIELD`, `MIGHTY_WEAPON`.
- Relics: +17. Fool pool: `SONG_BOW` (starter), `BIZARRE_LAMP`, `CONDUCTORS_BATON`, `EXERCISE_MAT`, `FERTILIZER`, `OCARINA`, `TWINE`, `WHITTLING_KNIFE`. Event pool: `BATH_ROBES`, `BATH_TOWEL`, `BYRDINE`, `MIGHTY_WINGS`, `OBSIDIAN_VICE`, `STORYTELLER`, `THUMBSCREW`, `TORMENTOUS_COLLAR`, `WUPPY`.
- Potions: +3, all Fool: `BOTTLED_SONG`, `CRYSTALLINE_EUPHORIA`, `PRECISION_POTION`.
- Powers: +41, -1. Removed `AMBERGRIS_POWER` (Ambergris now applies the new generic `EXTRA_TURN_POWER`). Non-Fool additions: `AEONGLASS_DOMINION_POWER`, `INSATIABLE_DOMINION_POWER`, `LAGAVULIN_MATRIARCH_DOMINION_POWER`, `YOU_ARE_MINE_POWER`, `EXPLOSIVE_POWER` (Dread), `WUPPYS_LOYALTY_POWER`, `EXTRA_TURN_POWER`, `FREE_CARD_POWER`, `TEMPORARY_THORNS_POWER`, `IMPALED_POWER`, `AIM_POWER`, `STRETCH_POWER`. The rest belong to Fool cards.
- Monsters: +2, `BYRDINE_MONSTER` (Byrdpip's evolved form) and `WUPPY` (event companion). No new encounters.
- Events: +5. Shared list gains `MUSEUM_OF_TORTURE` (act index 1 only). Glory (act 3) gains `SHRINE_OF_THE_WARLORD`, `THERMAE_NONUPEIPE`; Underdocks gains `WATERFALL_OF_OBVIOUS_SECRETS`, `WUPPY_IN_TROUBLE`. Both act lists are re-sorted alphabetically, which moves Trial after Tinker Time.
- Characters: +2, `FOOL` and the non-playable `ROTATING_CHARACTER` (Dread's rotating-streak placeholder). `ModelDb.Orbs` now also lists `GLASS_ORB` (it already existed), but `OrbModel.GetRandomOrb` still draws from its own fixed list, so random orbs are unchanged.
- Epochs: +9 (`FOOL1`..`FOOL7`, `BLITZ`, `DREAD`). The Fool is unlocked by `Fool1Epoch` and the Fool potion pool by `Fool4Epoch`. The veteran unlock preset has to be rebuilt.
- Moves renamed: Rocket `TARGETING_RETICLE_MOVE`, `PRECISION_BEAM_MOVE`, `LASER_MOVE` become `CALCULATING_MOVE`, `PRECISE_SMASH_MOVE`, `ROCKET_CLAW_MOVE` (plus an unreferenced `PRECISE_SMASH_MOVE_2` state); Tunneler `BELOW_MOVE` becomes `FROM_BELOW_MOVE`; Infested Prism `RADIATE_MOVE` becomes `JAB_MOVE_2`; Queen gains `DREAD_INITIAL_MOVE`.

Pool insertion points (existing order preserved): Colorless pool 65 to 66 with My Shinies at index 35 (53 to 54 rolled cards); event card pool 28 to 32 (Dingy Dagger 7, God Byrd 14, Mighty Shield 19, Mighty Weapon 20); status pool 12 to 13 (Sadness 7); token pool 14 to 39; event relic pool 142 to 151. Shared and existing character relic pools, all potion pools other than the new Fool one, and the five existing character card pools are unchanged in membership and order (Memento Mori changes cost and target only).

## 3. The Fool

| Item | Value |
|---|---|
| HP, gold | 77, 99 |
| Starting deck | 5 `STRIKE_FOOL` (6, +3), 4 `DEFEND_FOOL` (5 Block), 1 `FOOLISH_SHOT` (15, +5; shoots an arrow) |
| Starter relic | Song Bow: 6 Aim at the start of each combat |
| Arrow slots | `BaseArrowSlotCount` 1 |
| Card pool | 90: 3 Basic, 20 Common, 38 Uncommon, 27 Rare, 2 Ancient (`DIVINE_SHOT`, `LOST_LUTE`); by type 36 Attack, 34 Skill, 20 Power. 84 are available solo. |
| Relics, potions | 8 relics, 3 potions (pool locked behind `Fool4Epoch`) |
| Ancient hooks | Archaic Tooth maps Foolish Shot to Divine Shot; Touch of Orobas maps Song Bow to Storyteller; Colorful Philosophers and Yummy Cookie gain Fool entries |

New mechanics the engine needs:

1. **Arrows.** `PlayerCombatState` owns a list of `ArrowSlot`s (status Retrieved / shot / impaling). `ArrowCmd` shoots an arrow (consumes an available slot), impales the target with an instanced-per-applier `ImpaledPower` (hit counter 3), and dislodges arrows (retrieving the slot). New combat hooks `AfterArrowShot`, `AfterArrowImpaled`, `AfterArrowDislodged`. A new `ValueProp.FromArrow` flag marks arrow damage. Escape and Tough Egg handle Impaled.
2. **Aim** is a counter buff that adds to the first powered `FromArrow` attack from a card, snapshotted at attack start.
3. **Melody tags.** New `CardTag`s `Wah`, `Cho`, `Ki`, `Shoot`; `MelodyCmd` counts first-of-tag plays per turn and "melodies" per turn from combat history.
4. **Stretch** adds block to the first block-gaining card play, snapshotted at play start (Adroit enchantment now reports `GainsBlock`).
5. **Tarot generation.** Mythic Form generates random tarot tokens into hand each turn on `CombatCardSelection` (filtered by `IsAllowedInRun`); The Fool token autoplays random hand cards.
6. **Card-model features.** `TransformIntoAfterPlay` (a card transforms itself after its play resolves), `ExtraReplayThisTurn`, keywords added or removed "from outside" that survive upgrades, `TryModifyKeywordsInCombat` (used by Phantom Blades too), `HitCountVar` previews through `ModifyAttackHitCount`.
7. Joy (0-cost token: +1 energy, draw 1, Exhaust) and Sadness (unplayable Retain status).

Scope estimate: roughly the size of a full existing character (about 120 card, relic, potion and power definitions) plus the arrow subsystem, which touches the damage path, the combat state, the observation and the replay projection. It is the largest single work item in this patch.

## 4. Game modes

`GameMode` loses `Custom` (now a separate `IsCustomGame` flag) and gains `Dread` and `Blitz`. Each run carries `RunModels`: always `MultiplayerScalingModel`, plus `DreadRunModel` or `BlitzRunModel`. Run models and the active dominion are hook listeners in combat, after the existing listeners.

**Dread.** Ascension still runs up to 10.
- At every act start after the first, every player gets Ascender's Bane.
- The act's boss defines a dominion, applied to every combat in the act except the boss fight itself: Aeonglass (Aeonglass Dominion power on players), Ceremonial Beast (enemies Ritual 1), Insatiable (power on players), Kaiser Crab (enemies Plating 7), Kin (players Weak 2, Frail 2), Knowledge Demon (Sloth and Amnesia into hand before draw), Lagavulin Matriarch (power on players), Queen (You Are Mine 3), Soul Fysh (on players entering), Test Subject (enemies Strength 3), Vantom (enemies Slippery, including ones added later), Waterfall Giant (non-minion enemies Explosive, amount by encounter).
- Every boss reads `AscensionHelper.IsDread()`: Aeonglass Wither +1 and shuffled into the draw pile; Ceremonial Beast +30 HP, Plow Strength 3, Ritual 2; Crusher and Rocket Plating 14, Rocket Claw +5; Knowledge Demon Disintegration 8/9/10; Lagavulin Matriarch +22 HP, Slash 2 +2 damage and block, debuffs -3; Queen opens with a Dread move (Chains of Binding 3, Frail/Weak/Vulnerable 99) and gains 2 Strength instead of 1; Soul Fysh Beckon 3 (one into hand); Test Subject Enrage +1, Multi Claw 4, Burning Growl +3; The Insatiable +20 HP, Sandpit 3; The Kin three followers; Torch Head Amalgam Strong Tackle +5; Vantom Slippery +2, Prepare 3; Waterfall Giant Steam Eruption 4.

**Blitz.** Ascension 0 to 5, mapped to A1, A2, A3 (Tough Enemies), A4 (Deadly Enemies), A5 (Ascender's Bane).
- Map: `BlitzActMap`, an Ancient start then six fixed nodes (Monster, Monster, Shop, Elite, Treasure, Rest) and the boss. The Ancient is entered at every act start. Act room generation uses one weak encounter instead of the act's usual count.
- Rewards replace the defaults: hallway Common card reward (plus a potion if not weak), elite Uncommon card reward and two relics, act 1 and 2 boss Rare card reward and a potion. From act index 1 combat card rewards get one (act 2) or two (act 3) upgraded options, picked on `rewards`.
- Neow gives a card choice from 10 (Blitz Neow odds 0.55/0.4/0.05) before any modifiers.
- Merchant: `CreateForBlitzMerchant`, six entries (five drawn without replacement from the 14 basic entry types, rare ones at half weight, then one from the 6 premium types from Glam on), three purchases, all on `shops`. Shop rarity odds 0.4/0.5/0.1. Enchantment options Sharp 2, Nimble 3, Adroit 3, Steady 1, Swift 2, Vigorous 5.
- 44 relics are banned in Blitz (`RelicLists.notAllowedInBlitz`), as are Debt, Hand of Greed and Royalties; Potion Belt is banned late in act 3. The Gremlin Merc does not steal.

Late-act-3 relic bans are now one list (`notAllowedLateInAct3`, same 17 relics as before) checked from `RelicModel.IsAllowed`; the threshold is unchanged outside Blitz.

## 5. Cards

Values are base(upgraded). "Refactor" means `RepeatVar` or a literal hit count became `HitCountVar` with the same number; it only matters for hit-count previews.

Silent

| Card | 0.111 | 0.113.1 |
|---|---|---|
| Expertise | draw 2(3), the drawn cards Retain this turn | draw 2(3), then Retain Hand (whole hand retains this turn) |
| Memento Mori | cost 1, any enemy, 9 + 4 per discard this turn (+2/+1) | cost 3, Sly, 19(23) to the lowest-HP hittable enemy (target type RandomEnemy) |
| Speedster | Speedster power 2 | 3 (upgrade still adds Innate) |
| Phantom Blades | 9 (+3) | 8 (+3); Retain on Shivs is now a dynamic keyword while the power is up, instead of being applied to each Shiv |
| Predator (via Draw Cards Next Turn) | power removed at the owner's turn start | removed right after hand-draw modification |
| Nightmare | | the copy it adds gets end-of-turn cleanup |
| Dagger Spray, Flechettes, Finisher, Ricochet | | refactor |
| Grand Finale, Echoing Slash | | timing or animation only |

Ironclad

| Card | 0.111 | 0.113.1 |
|---|---|---|
| Expect a Fight | cost 3, Block 15(16) + 5(8) per Strength | cost 3, Block 15(20); Attacks in hand cost 1 less until played (not X-cost) |
| Forgotten Ritual | gain 3(4) energy | gain 3(4) energy only if a card was exhausted this turn |
| Juggling power | clone queued in `BeforeCardPlayed` | clone remembered before, made after the card play resolves |
| Fiend Fire | | hit count now previews (hand size excluding itself); play unchanged |
| Twin Strike, Thrash, Dismantle, Fight Me, Spite, Sword Boomerang, Tear Asunder, Conflagration | | refactor |
| Stomp, Bullet Time | | no code change (patch notes describe the state the 0.111 build already had) |

Other characters and pools

| Card | Change |
|---|---|
| Salvo (colorless) | 12 to 10 damage (+4) |
| My Shinies (colorless, new) | cost 1 Rare Attack, 1 damage per hit, hits = gold / 20 (upgrade: / 10) |
| Omnislice (colorless) | splash damage is no longer Unpowered, and Thorns no longer special-cases it |
| Minion Dive Bomb (token) | 13(16) to 11(13) |
| Enlightenment (event) | loses Exhaust |
| Brightest Flame (event) | max-HP loss now counts as from a card; Buffer does not block it |
| Dowsing (quest) | 5 to 4 rooms; after 3 rooms, unknown map points can no longer roll Monster or Elite |
| Byrdonis Egg (quest) | not offered when a Hatch option exists |
| Spoils Map (quest) | re-adds its quest marker when it returns to the deck in the same act |
| Helix Drill (Defect) | cost refund reads energy actually spent on the current play |
| Bone Shards, Sacrifice (Necrobinder) | Osty kill carries the card as source |
| Debt, Hand of Greed, Royalties | banned in Blitz |
| Astral Pulse, Celestial Might, Seven Stars, Sovereign Blade, Lunar Blast, Radiate (Regent); Barrage, Flak Cannon, Gunk Up, Refract, Uproar (Defect); Pull From Below, Rattle (Necrobinder); Exterminate, Mad Science, Maul, Peck, Rip and Tear (event) | refactor |

Engine-level card changes: Sly triggers on discard only from the hand; a transform reports the card it produced; cloning a dupe clones its original.

## 6. Relics

| Relic | Change |
|---|---|
| Brilliant Scarf (Ancient) | 5 to 6 cards |
| Jeweled Mask (Ancient) | the card becomes free this combat, not this turn |
| Ringing Triangle | was "don't flush the hand on turn 1"; now applies Retain Hand at combat start |
| Pael's Eye | extra turn granted in the new early pass (before other extra-turn sources) |
| Pael's Legion | doubles only the affected card play; other plays get x1 |
| Juzu Bracelet | also removes Elite from unknown-room outcomes |
| Book Repair Knife | counts enemies only (was every creature but the owner) |
| Velvet Choker | counts plays from combat history instead of its own counter |
| Kusarigama, Burning Sticks, Rainbow Ring | counter reset or set order fixed |
| Byrdpip | counts Byrd Swoop plays; after 6 it adds an Evolve rest-site option |
| Bing Bong, Hoarder (modifier) | the "skip the clone" guard is gone; clones are tagged by `clonedBy` instead |
| Large Capsule, Toy Box, Pael's Wing, Dig option | renamed reward-pull path; same `rewards` stream |
| Dusty Tome | tolerates no valid ancient card |
| Lord's Parasol | merchant purchase path refactor |
| Potion Belt | banned late in act 3 in Blitz |
| RNG moves | see section 1 |

New event relics: Bath Robes, Bath Towel (Thermae Nonupeipe), Mighty Wings (Shrine of the Warlord), Obsidian Vice, Thumbscrew, Tormentous Collar (Museum of Torture), Storyteller (Touch of Orobas), Byrdine, Wuppy (Wuppy in Trouble; a companion that takes damage before block through the new `ModifyPreBlockDamageTarget` hook and the `BeforeWuppy` HP-loss phase).

## 7. Potions

Ambergris applies `ExtraTurnPower` instead of `AmbergrisPower`. Entropic Brew moves to `non_combat_generation`. Three new Fool potions. The shared pool is unchanged.

## 8. Powers and keywords

| Power | Change |
|---|---|
| Doom | does not kill a creature already dead or out of combat |
| Ritual | "just applied by enemy" is set only when applied during the enemy side's turn |
| Lethality | the extra hit counts only during a real card play |
| Imitation Learning, One for All | autoplay handling (multiplayer cards) |
| Shrink | does not fire when the applier is a player |
| Sic 'Em | instanced per applier |
| Covered, Intercept, Tank | clean up their partner powers when removed (multiplayer) |
| Buffer | does not stop Brightest Flame |
| Thorns | Omnislice no longer special-cased |
| Glam, Sown, Swift (enchantments) | disabled after the card play instead of during it |
| Tezcatara's Ember (enchantment) | its damage is a plain variable, no longer a powered `DamageVar` |

## 9. Monsters and encounters

Outside Dread:

| Monster | 0.111 | 0.113.1 |
|---|---|---|
| Axebot | HP 76-86 at Tough Enemies, 70-78 below | HP 80 at Tough Enemies, 70 below, no range |
| Infested Prism | Radiate (13/11 damage + 13/11 Block); Pulsate gives Block 22/20 | Radiate replaced by a second Jab (17/15); Pulsate applies Vulnerable 1 instead of Block |
| Tunneler | Below 26/23 | From Below 28/24 |
| Rocket (Kaiser Crab) | Targeting Reticle, Precision Beam, Laser | renamed Calculating, Precise Smash, Rocket Claw; same numbers and cycle |
| Ceremonial Beast | Plow always ends with +2 Strength | only if Plow power is still on it (a broken Plow gives no Strength) |
| Thieving Hopper | steals Basic or Quest | Basic only, not Quest; `CombatCardSelection` stream |
| Gremlin Merc | | no stealing in Blitz |
| Twig Slime S | | gains Phobia spine skin flag (visual) |

The bosses SpireFish tracks:

| Boss | Outside Dread | Dread |
|---|---|---|
| Test Subject | unchanged | yes (section 4) |
| Queen | unchanged (non-Dread Strength still 1) | yes |
| Aeonglass | unchanged | yes |
| The Insatiable | unchanged | yes |
| Ceremonial Beast | Plow Strength rule above | yes |
| Waterfall Giant | unchanged | yes |
| Terror Eel | unchanged (file identical) | no change |
| Skulking Colony | unchanged (file identical) | no change |

## 10. Combat timing and hooks

- Damage pipeline: `ModifyPreBlockDamageTarget` runs before block (Wuppy); HP-loss phases are now `BeforeWuppy`, `BeforeOsty`, `AfterOsty`. A new `ModifyDamageMultiplicativeLate` pass runs after the multiplicative pass and before caps (no content uses it yet).
- Extra turns: `ShouldTakeExtraTurnEarly` is asked of every listener before `ShouldTakeExtraTurn`; the first source wins, and only that source's `AfterTakingExtraTurn` fires.
- `AfterDeath` and `Kill` carry the card that caused the death.
- End of turn: the end-turn wait now blocks on any player-driven action still executing; turn-end cards removed from state mid-resolution are skipped. Players who cannot back out cannot un-end their turn.
- `CombatManager.ShouldSkipCombatEffect` is the old "over or ending and not starting" check under a name.
- Hook listeners: run models and the dominion are added after modifiers and badges.

## 11. Map, rest sites, shops, events, Neow, rewards

- Map: rest sites allowed from row 5 (was 6), elites still from row 6. The point-type queue picks the first valid type in queue order and removes it, instead of rotating rejected types to the back. This changes generated maps for the same seed. Point-type counts and pruning ignore points with `CanBeModified` false.
- Smith option: always one card (the `SmithCount` field is gone; nothing in 0.111 set it above 1).
- Mend option: modifiers can add text (Night Terrors). Heal preview refactor only.
- Merchant: see section 1. Order is relic rarity rolls and relic fills, potions, then card fills, then the on-sale character card index (`NextInt(5)` on `shops`) last. Card removal can be disabled by a hook (`ShouldAllowMerchantCardRemoval`).
- Rewards: `TryReplaceDefaultRoomRewards` lets a run model replace room rewards (Blitz only). `RelicReward` and `GoldReward`/`PotionReward` accept an RNG override (Crystal Sphere, events, Blitz merchant).
- Events: Doors of Light and Dark's Light option upgrades one card the player picks (was two random); Zen Weaver's Breathing Techniques costs 25 (was 50) and gives one card (was two); Waterlogged Scriptorium's `PricklySpongeGold` is 120 (was 99); Fake Merchant builds its inventory through the new merchant path. New events: Museum of Torture (act index 1, Max HP 10, relic from Obsidian Vice / Thumbscrew / Tormentous Collar), Shrine of the Warlord (Mighty Weapon, Mighty Shield or Mighty Wings; not when Tanx is the act's ancient), Thermae Nonupeipe (gold over 175 and HP at or below 75%; 88 gold, all gold, or 31 HP and 3 Guilty), Waterfall of Obvious Secrets (Dingy Dagger, a relic and 2 Guilty, or 70 gold), Wuppy in Trouble (Wuppy at 60 or 30 HP, or ignore).
- Neow: the solo option list is unchanged; options lost their custom "done" text. In Blitz, Neow gives the Blitz card choice first.
- Dowsing and Juzu Bracelet modify the unknown-room outcome set through `ModifyUnknownMapPointRoomTypes`.

## 12. Save and recorder format

- Run save version 21: adds `is_custom_game` (old `game_mode: "custom"` migrates to Standard plus the flag). Extra run fields gain `used_debug_command`. `GameMode` values are now Standard, Daily, Dread, Blitz (Custom removed).
- Player RNG sets serialize a fourth stream, `non_combat_generation`. A recorder projection that lists each stream's state needs the new stream.
- Map points serialize `CanBeModified`; Blitz maps restore through `BlitzActMap`.
- Run history V11 and the progress save add Dread streaks and Blitz records (not gameplay).

## 13. Not verified yet

- Line-by-line behaviour of the 121 new cards, 41 new powers and 17 new relics; this inventory lists them and the mechanics they need.
- Monster AI beyond the files that changed (unchanged files were not re-read).
- Whether the Fool cards' `IsAllowedInRun` and unlock gates leave any card out of a veteran profile's pool.
- Multiplayer-only behaviour.
- Any recording: no 0.113 recording exists yet, so none of this has been checked against the game in motion.

## Reproducing

```sh
# outside this repository
<decompiler> sts2.dll -> <decomp-dir>          # whole-project decompile, refs = the game's data_sts2_macos_arm64
diff -ru <decomp-0.111.0> <decomp-0.113.1>
# from a SpireFish checkout
tools/beta_diff/run_all.sh <decomp-dir> <sts2sim-checkout> <out-dir>
```

The decompiled trees and the script outputs used for this inventory are kept outside Git, next to the copied binaries.
