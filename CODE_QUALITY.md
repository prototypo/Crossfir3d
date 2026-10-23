# Code Quality Report

Generated: 2026-10-23 by quality-report v0.4.1 (second run, after Q1–Q13 of the first report)

## Summary

- Languages analyzed: Rust (crates/cfproto, crates/crossfir3d), Python (scripts/, experiments/sprite3d/)
- Files analyzed: 79 (68 Rust, 11 Python)
- Tools used:
  - rust-code-analysis-cli 0.0.25
  - cargo clippy 0.1.98 (unwrap_used, expect_used, excessive_nesting at 4)
  - rustc dead_code
  - radon 6.0.1
  - ruff 0.6.7
  - vulture 2.16 (run one file at a time: on both folders together it crashed with a recursion error)
  - function_metrics.py (quality-report)
- Tools missing: none. Rust nesting depth and comment presence come from a brace and comment scan of each function (estimated).
- Findings: 102 open
- Branch decision: quality-improvements
- Scope notes: Rust `mod tests` blocks are left out of the function counts but kept in file sizes. The integration tests in `crates/crossfir3d/tests/` are counted. The Python files have not changed since the first run, so their numbers are carried over from it.

## Metrics

| Metric | Threshold | Status | Compliance | Source |
|---|---|---|---|---|
| Cyclomatic complexity | > 10 | 🟡 92.2% pass | 🟢 758 · 🟡 46 · 🔴 68 | tool |
| Function length | > 50 lines | 🟢 97.2% pass | 🟢 840 · 🟡 8 · 🔴 24 | tool |
| Nesting depth | > 4 levels | 🟢 98.7% pass | 🟢 838 · 🟡 23 · 🔴 11 | tool (Rust estimated) |
| Argument count | > 5 | 🔴 worst 2.8× limit | 🟢 792 · 🟡 40 · 🔴 40 (95.4% pass) | tool |
| Code duplication | ≥ 5 lines repeated | 🟢 0.1% (estimated) | 1 group | tool + judgment |
| Dead code | any | 🟢 100% (872/872) | 0 unused | tool |
| Error-handling smells | unwrap/expect outside tests; bare except | 🟡 1.0 / 1k lines | 20 Rust, 0 Python | tool |
| File size | > 500 lines | 🟡 94.9% pass | 🟢 72 · 🟡 3 · 🔴 4 | tool |
| Comment quality | WHAT, not HOW, on functions > 10 lines | 🔴 75.5% (estimated) | 🟢 275 · 🔴 89 | judgment |
| Reusability | shared logic | 🟢 0.1% (estimated) | 1 group | judgment |

Comment presence was counted for every function longer than 10 lines; comment quality (WHAT vs HOW) was judged on a sample: the functions with findings and the largest files. In that sample the comments that exist say WHAT, not HOW. Suppressions: 16 `#[allow(clippy::too_many_arguments / type_complexity)]` attributes (22 in the first run), 9 of them in characters3d.rs; their functions are still counted above. No `#[allow(dead_code)]`, `# noqa` or bare `except:`.

## Distribution

| Metric | Compliance | Worst | Median | Std dev |
|---|---|---|---|---|
| Cyclomatic complexity | 92.2% (804/872) | 33 | 3 | 4.9 |
| Function length | 97.2% (848/872) | 135 | 9 | 13.7 |
| Nesting depth | 98.7% (861/872) | 7 | 1 | 1.2 |
| Argument count | 95.4% (832/872) | 14 | 2 | 1.9 |
| File size | 94.9% (75/79) | 1,617 | 225 | 225.7 |

By language: Rust complexity 92.3% (worst 33), length 97.2% (worst 135), nesting 99.0% (worst 7), arguments 95.3% (worst 14), file size 94.1% (worst 1,617). Python complexity 90.0% (worst 29), length 98.0% (worst 71), nesting 94.0% (worst 7), arguments 98.0% (worst 6), file size 100% (worst 327).

### Change since the first run

| Metric | Compliance, first → now | Worst, first → now | Std dev, first → now |
|---|---|---|---|
| Cyclomatic complexity | 87.6% → 92.2% | 159 → 33 | 12.5 → 4.9 |
| Function length | 94.6% → 97.2% | 368 → 135 | 29.4 → 13.7 |
| Nesting depth | 97.4% → 98.7% | 10 → 7 | 1.3 → 1.2 |
| Argument count | 92.9% → 95.4% | 15 → 14 | 2.3 → 1.9 |
| File size | 90.6% → 94.9% | 2,566 → 1,617 | 365.3 → 225.7 |

The denominator grew: 645 → 872 functions and 64 → 79 files, because the fixes split big functions and files into smaller ones. Every ratio rose, every worst value fell, and every standard deviation fell.

### Counts lie when the denominator moves

Fixing these findings adds functions and files, so a raw count of violations can rise
while the code gets better. When comparing with a later run, lead with the compliance
ratio, quote the worst value and the standard deviation, and say whether the
denominator grew.

## Findings (worst first)

### [x] 🔴 Q1: complexity — crates/crossfir3d/src/held.rs:198 `class_look`

- Measured: complexity 33 (threshold 10)
- Recommendation: Move the back and over `match`es into their own `class_back` and `class_over` functions; the rest is one row per class, so its complexity is data, not logic.

### [x] 🔴 Q2: file size — crates/crossfir3d/src/world.rs

- Measured: 1,617 lines (threshold 500)
- Recommendation: Move each family of spawn helpers (`spawn_part`, the ground, the creatures, the arrows) into a `world/` submodule, as `buildings/` now is.

### [x] 🔴 Q3: complexity — crates/crossfir3d/src/buildings/castle.rs:7 `castle`

- Measured: complexity 31 (threshold 10)
- Recommendation: Split the castle into one function per piece (walls, towers, gate, keep), as the other building families already are.

### [x] 🔴 Q4: complexity, length, nesting, arguments — crates/crossfir3d/src/characters_ui.rs:31 `characters_screen`

- Measured: complexity 29 (threshold 10); length 91 (threshold 50); nesting 5 (threshold 4); arguments 7 (threshold 5)
- Recommendation: Split into the character list, the buttons and the login handling, and group its 7 system arguments into a `SystemParam` struct like `new_character`'s helpers.

### [x] 🔴 Q5: complexity — experiments/sprite3d/catalog.py:208 `classify`

- Measured: complexity 29 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [x] 🔴 Q6: complexity, length, nesting, arguments — crates/crossfir3d/src/characters3d.rs:790 `dress_people`

- Measured: complexity 28 (threshold 10); length 72 (threshold 50); nesting 5 (threshold 4); arguments 10 (threshold 5)
- Recommendation: Group its 10 system arguments into a `SystemParam` struct and move each clothing step (body, hair, hat, held items) into its own function, as `dress_robes` now is.

### [x] 🔴 Q7: complexity, length, arguments — crates/crossfir3d/src/input.rs:86 `send_moves`

- Measured: complexity 17 (threshold 10); length 66 (threshold 50); arguments 14 (threshold 5)
- Recommendation: Group the queries and resources into a `SystemParam` struct.

### [x] 🔴 Q8: complexity, length — crates/crossfir3d/src/login.rs:77 `on_message`

- Measured: complexity 28 (threshold 10); length 67 (threshold 50)
- Recommendation: Handle each login message kind in its own function; `on_message` keeps only the `match`.

### [x] 🔴 Q9: complexity, length, nesting, arguments — crates/crossfir3d/src/settings_ui.rs:27 `settings_screen`

- Measured: complexity 22 (threshold 10); length 135 (threshold 50); nesting 5 (threshold 4); arguments 6 (threshold 5)
- Recommendation: Draw each settings group (window, graphics, sound, log) in its own function, and use one shared overlay frame (see the reusability finding).

### [x] 🔴 Q10: complexity, length — scripts/obj2gltf.py:29 `convert`

- Measured: complexity 27 (threshold 10); length 71 (threshold 50)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [x] 🔴 Q11: complexity, length — crates/crossfir3d/src/buildings/items.rs:194 `item`

- Measured: complexity 26 (threshold 10); length 71 (threshold 50)
- Recommendation: Split the item `match` into family tables (weapons, armour, jewels, potions), like `buildings::parts`.

### [x] 🔴 Q12: complexity, arguments — crates/crossfir3d/src/characters3d.rs:469 `wear_helmet`

- Measured: complexity 12 (threshold 10); arguments 13 (threshold 5)
- Recommendation: Group its 13 arguments into a `SystemParam` struct or a small struct, as `hat::HatKit` does for `put_on`.

### [x] 🔴 Q13: complexity, length, arguments — crates/crossfir3d/src/combat_fx.rs:48 `watch`

- Measured: complexity 24 (threshold 10); length 66 (threshold 50); arguments 13 (threshold 5)
- Recommendation: Group its 13 system arguments into a `SystemParam` struct and move each effect (flash, damage number, hit text) into its own function.

### [x] 🔴 Q14: complexity — crates/crossfir3d/src/held.rs:66 `worn`

- Measured: complexity 26 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [x] 🔴 Q15: complexity, length, nesting — crates/crossfir3d/src/net.rs:39 `connect`

- Measured: complexity 26 (threshold 10); length 61 (threshold 50); nesting 5 (threshold 4)
- Recommendation: Move the address parsing, the socket set-up and the handshake into their own functions.

### [x] 🔴 Q16: length, arguments — crates/crossfir3d/src/sky.rs:153 `update_sky`

- Measured: length 62 (threshold 50); arguments 13 (threshold 5)
- Recommendation: Group its 13 system arguments into a `SystemParam` struct; move the sun, the clouds and the stars into their own functions.

### [x] 🔴 Q17: complexity, nesting — crates/cfproto/src/stats.rs:73 `apply`

- Measured: complexity 25 (threshold 10); nesting 5 (threshold 4)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [x] 🔴 Q18: complexity, length, nesting, arguments — crates/crossfir3d/src/hud.rs:70 `hud`

- Measured: complexity 25 (threshold 10); length 95 (threshold 50); nesting 6 (threshold 4); arguments 9 (threshold 5)
- Recommendation: Draw the bars, the quick slots, the log and the parchment box in their own functions, and group its 9 system arguments into a `SystemParam` struct.

### [x] 🔴 Q19: complexity — crates/crossfir3d/src/modes.rs:68 `act`

- Measured: complexity 25 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [x] 🔴 Q20: complexity, length, arguments — crates/crossfir3d/src/world.rs:1112 `spawn_part`

- Measured: complexity 25 (threshold 10); length 106 (threshold 50); arguments 8 (threshold 5)
- Recommendation: Move each `Shape` arm with more than a few lines (lamps, mist, models, signs) into its own function.

### [x] 🔴 Q21: complexity, length, nesting — crates/cfproto/src/map2.rs:25 `parse_map2`

- Measured: complexity 24 (threshold 10); length 51 (threshold 50); nesting 7 (threshold 4)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [x] 🔴 Q22: complexity, arguments — crates/crossfir3d/src/characters3d.rs:381 `hold_items`

- Measured: complexity 15 (threshold 10); arguments 12 (threshold 5)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [x] 🔴 Q23: complexity, length — crates/crossfir3d/src/back.rs:45 `tabard_straps`

- Measured: complexity 22 (threshold 10); length 66 (threshold 50)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [x] 🔴 Q24: complexity — crates/crossfir3d/src/held.rs:15 `held_model`

- Measured: complexity 22 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [x] 🔴 Q25: complexity — crates/crossfir3d/src/input.rs:153 `act_key`

- Measured: complexity 22 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [x] 🔴 Q26: complexity, length — crates/crossfir3d/src/ui.rs:56 `typed_bind`

- Measured: complexity 22 (threshold 10); length 66 (threshold 50)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [x] 🔴 Q27: complexity, length, arguments — crates/crossfir3d/src/ui.rs:294 `command_line`

- Measured: complexity 14 (threshold 10); length 56 (threshold 50); arguments 11 (threshold 5)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [x] 🔴 Q28: complexity, length, arguments — crates/crossfir3d/src/world.rs:1301 `tint_model`

- Measured: complexity 14 (threshold 10); length 51 (threshold 50); arguments 11 (threshold 5)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [x] 🔴 Q29: complexity, length, arguments — crates/crossfir3d/src/world.rs:1386 `follow_player`

- Measured: complexity 12 (threshold 10); length 57 (threshold 50); arguments 11 (threshold 5)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [x] 🔴 Q30: complexity, nesting — experiments/sprite3d/catalog.py:114 `parse_arch`

- Measured: complexity 22 (threshold 10); nesting 7 (threshold 4)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [x] 🔴 Q31: file size — crates/crossfir3d/src/characters3d.rs

- Measured: 1,075 lines (threshold 500)
- Recommendation: Move the dressing systems (`dress_people`, `wear_helmet`, `put_on_head`) into a `characters3d/dress.rs` submodule.

### [x] 🔴 Q32: complexity — crates/cfproto/src/items.rs:83 `parse_upditem`

- Measured: complexity 21 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [x] 🔴 Q33: complexity, length — crates/crossfir3d/src/skeleton.rs:26 `bones`

- Measured: complexity 21 (threshold 10); length 105 (threshold 50)
- Recommendation: Build each limb with one shared helper; the arms and legs repeat the same pattern.

### [x] 🔴 Q34: complexity — crates/crossfir3d/tests/live_server.rs:77 `pick_up_and_drop`

- Measured: complexity 21 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [!] 🔴 Q35: complexity — crates/cfproto/src/client.rs:33 `encode`

- Measured: complexity 20 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.
- Blocked: a flat match over the 17 message kinds is the clearest form. Splitting it under 10 needs dead `=> Vec::new()` arms that would send an empty packet for a misplaced kind. Kept the old code; added byte-exact golden tests for every kind instead.

### [x] 🔴 Q36: complexity — crates/cfproto/src/stats.rs:38 `parse_stats`

- Measured: complexity 20 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [x] 🔴 Q37: arguments — crates/crossfir3d/src/characters3d.rs:551 `gear_up`

- Measured: arguments 10 (threshold 5)
- Recommendation: This is a Bevy system: group its queries and resources into a `#[derive(SystemParam)]` struct.

### [x] 🔴 Q38: error handling — `unwrap()` on game queries: crates/crossfir3d/src/characters3d.rs:358, crates/crossfir3d/src/robe/dress.rs:84, crates/crossfir3d/src/robe/dress.rs:234, crates/crossfir3d/src/world.rs:679

- Measured: 4 query `get(...).unwrap()` calls that can panic if an entity loses a component; 16 more `unwrap`/`expect` calls guard invariants (fixed-size byte slices, built-in meshes, constant styles). 20 in all, outside tests: 1.0 per 1,000 lines
- Recommendation: Replace the 4 query unwraps with `let Ok(..) = ... else { continue/return }`. Give the remaining bare `unwrap()`s an `expect("why it cannot fail")` (server.rs:110, 113; sky.rs:92, 106; keys_ui.rs:134; hat/wear.rs:45).

### [x] 🔴 Q39: complexity — crates/crossfir3d/src/buildings/walls.rs:75 `wall_parts`

- Measured: complexity 19 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [x] 🔴 Q40: complexity, arguments — crates/crossfir3d/src/characters3d.rs:242 `animate_beasts`

- Measured: complexity 19 (threshold 10); arguments 8 (threshold 5)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [x] 🔴 Q41: complexity — crates/crossfir3d/src/newchar.rs:59 `on_reply`

- Measured: complexity 19 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [x] 🔴 Q42: complexity — crates/crossfir3d/src/races.rs:35 `look_for_race`

- Measured: complexity 19 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [x] 🔴 Q43: reusability — the overlay window frame is copied in 3 places: crates/crossfir3d/src/settings_ui.rs:49, crates/crossfir3d/src/inventory_ui.rs:272, crates/crossfir3d/src/keys_ui.rs:333

- Measured: 5 identical frame lines × 3 (`egui::Frame` fill, gold stroke, radius, margin), plus the fixed, untitled, unmovable `egui::Window` in two of them
- Recommendation: Add one `overlay_frame()` in `ui.rs` (keys_ui.rs already has it as `window_frame`) and make the three screens call it.

### [x] 🔴 Q44: complexity, arguments — crates/crossfir3d/src/characters3d.rs:319 `bind_bones`

- Measured: complexity 16 (threshold 10); arguments 9 (threshold 5)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [x] 🔴 Q45: comment quality — 27 of the flagged functions have no comment saying what they do

- Measured: Missing on, among others: `characters_screen`, `on_message`, `item`, `connect`, `apply` (stats.rs), `hud`, `spawn_part`, `parse_map2`, `watch`, `act_key`, `settings_screen`. Overall 275 of 364 functions longer than 10 lines have a comment (75.5%, estimated)
- Recommendation: Add one `///` line above each saying WHAT it does ("Draws the settings overlay"), not how.

### [x] 🔴 Q46: complexity — crates/cfproto/src/spells.rs:23 `parse_addspell`

- Measured: complexity 17 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [x] 🔴 Q47: complexity — crates/crossfir3d/src/markup.rs:16 `colour`

- Measured: complexity 17 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [x] 🔴 Q48: complexity — crates/crossfir3d/src/settings.rs:127 `from_text`

- Measured: complexity 17 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [x] 🔴 Q49: arguments — crates/crossfir3d/src/characters3d.rs:589 `put_on_head`

- Measured: arguments 8 (threshold 5)
- Recommendation: This is a Bevy system: group its queries and resources into a `#[derive(SystemParam)]` struct.

### [x] 🔴 Q50: arguments — crates/crossfir3d/src/characters3d.rs:641 `put_in_hand`

- Measured: arguments 8 (threshold 5)
- Recommendation: This is a Bevy system: group its queries and resources into a `#[derive(SystemParam)]` struct.

### [x] 🔴 Q51: complexity — crates/crossfir3d/src/keys.rs:146 `game_key`

- Measured: complexity 16 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [x] 🔴 Q52: complexity, arguments — crates/crossfir3d/src/skeleton.rs:138 `build_skeletons`

- Measured: complexity 16 (threshold 10); arguments 8 (threshold 5)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [x] 🔴 Q53: file size — crates/crossfir3d/src/keys_ui.rs

- Measured: 768 lines (threshold 500)
- Recommendation: Move the keyboard drawing into `keys_ui/keyboard.rs` and the lists into `keys_ui/lists.rs`.

### [x] 🔴 Q54: complexity, nesting — crates/cfproto/src/view.rs:31 `apply`

- Measured: complexity 12 (threshold 10); nesting 6 (threshold 4)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [x] 🔴 Q55: complexity — crates/crossfir3d/src/buildings/village.rs:7 `village`

- Measured: complexity 15 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [x] 🔴 Q56: complexity — crates/crossfir3d/src/characters3d.rs:607 `grip`

- Measured: complexity 15 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [x] 🔴 Q57: complexity, length — crates/crossfir3d/src/characters3d.rs:683 `staff_top`

- Measured: complexity 15 (threshold 10); length 70 (threshold 50)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [x] 🔴 Q58: complexity — crates/crossfir3d/src/markup.rs:39 `apply`

- Measured: complexity 15 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [x] 🔴 Q59: complexity — experiments/sprite3d/auto.py:37 `main`

- Measured: complexity 15 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [x] 🔴 Q60: complexity, arguments — crates/crossfir3d/src/back.rs:161 `panel`

- Measured: complexity 13 (threshold 10); arguments 7 (threshold 5)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [x] 🔴 Q61: arguments — crates/crossfir3d/src/characters3d.rs:159 `spawn_character`

- Measured: arguments 7 (threshold 5)
- Recommendation: This is a Bevy system: group its queries and resources into a `#[derive(SystemParam)]` struct.

### [x] 🔴 Q62: arguments — crates/crossfir3d/src/characters3d.rs:188 `spawn_monster`

- Measured: arguments 7 (threshold 5)
- Recommendation: This is a Bevy system: group its queries and resources into a `#[derive(SystemParam)]` struct.

### [x] 🔴 Q63: arguments — crates/crossfir3d/src/characters3d.rs:213 `spawn_beast`

- Measured: arguments 7 (threshold 5)
- Recommendation: This is a Bevy system: group its queries and resources into a `#[derive(SystemParam)]` struct.

### [x] 🔴 Q64: arguments — crates/crossfir3d/src/characters3d.rs:960 `update_preview`

- Measured: arguments 7 (threshold 5)
- Recommendation: This is a Bevy system: group its queries and resources into a `#[derive(SystemParam)]` struct.

### [x] 🔴 Q65: complexity — crates/crossfir3d/src/keys.rs:169 `reserved`

- Measured: complexity 14 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [x] 🔴 Q66: complexity — crates/crossfir3d/src/keys.rs:225 `parse_typed`

- Measured: complexity 14 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [x] 🔴 Q67: arguments — crates/crossfir3d/src/main.rs:290 `dev_commands`

- Measured: arguments 7 (threshold 5)
- Recommendation: This is a Bevy system: group its queries and resources into a `#[derive(SystemParam)]` struct.

### [x] 🔴 Q68: complexity — crates/crossfir3d/tests/live_server.rs:141 `create_chosen_character`

- Measured: complexity 14 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [x] 🔴 Q69: complexity — crates/crossfir3d/src/buildings/creatures.rs:215 `chess_piece`

- Measured: complexity 13 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [x] 🔴 Q70: complexity, arguments — crates/crossfir3d/src/graphics.rs:79 `apply_settings`

- Measured: complexity 13 (threshold 10); arguments 6 (threshold 5)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [x] 🔴 Q71: complexity — crates/crossfir3d/src/signs.rs:39 `paint`

- Measured: complexity 13 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [x] 🔴 Q72: complexity — scripts/pick_models.py:193 `copy_model`

- Measured: complexity 13 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [x] 🔴 Q73: nesting — scripts/pick_models.py:240 `grey_copies`

- Measured: nesting 5 (threshold 4)
- Recommendation: Return early or extract the innermost block into a helper to cut the nesting.

### [x] 🔴 Q74: nesting — experiments/sprite3d/catalog.py:196 `map_faces`

- Measured: nesting 5 (threshold 4)
- Recommendation: Return early or extract the innermost block into a helper to cut the nesting.

### [x] 🔴 Q75: length — crates/crossfir3d/src/main.rs:139 `main`

- Measured: length 61 (threshold 50)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [x] 🔴 Q76: complexity — crates/cfproto/src/items.rs:69 `parse_item2`

- Measured: complexity 12 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [x] 🔴 Q77: complexity — crates/crossfir3d/src/buildings/items.rs:49 `die`

- Measured: complexity 12 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [x] 🔴 Q78: arguments — crates/crossfir3d/src/buildings/items.rs:166 `hoop`

- Measured: arguments 6 (threshold 5)
- Recommendation: This is a Bevy system: group its queries and resources into a `#[derive(SystemParam)]` struct.

### [!] 🔴 Q79: arguments — crates/crossfir3d/src/buildings/props.rs:71 `arc`

- Measured: arguments 6 (threshold 5)
- Recommendation: This is a Bevy system: group its queries and resources into a `#[derive(SystemParam)]` struct.
- Blocked: `arc` is a geometry helper with 6 clear, all-varying inputs at 6 call sites; a struct would make every call harder to read without making anything safer.

### [x] 🔴 Q80: arguments — crates/crossfir3d/src/characters3d.rs:426 `wear_armour`

- Measured: arguments 6 (threshold 5)
- Recommendation: This is a Bevy system: group its queries and resources into a `#[derive(SystemParam)]` struct.

### [x] 🔴 Q81: complexity, arguments — crates/crossfir3d/src/characters3d.rs:883 `animate`

- Measured: complexity 12 (threshold 10); arguments 6 (threshold 5)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [x] 🔴 Q82: arguments — crates/crossfir3d/src/graphics.rs:153 `remember_window`

- Measured: arguments 6 (threshold 5)
- Recommendation: This is a Bevy system: group its queries and resources into a `#[derive(SystemParam)]` struct.

### [x] 🔴 Q83: arguments — crates/crossfir3d/src/graphics.rs:260 `fit_scene_image`

- Measured: arguments 6 (threshold 5)
- Recommendation: This is a Bevy system: group its queries and resources into a `#[derive(SystemParam)]` struct.

### [x] 🔴 Q84: arguments — crates/crossfir3d/src/input.rs:223 `binding_commands`

- Measured: arguments 6 (threshold 5)
- Recommendation: This is a Bevy system: group its queries and resources into a `#[derive(SystemParam)]` struct.

### [x] 🔴 Q85: complexity — crates/crossfir3d/src/keys.rs:281 `save_bindings`

- Measured: complexity 12 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [x] 🔴 Q86: arguments — crates/crossfir3d/src/messages.rs:38 `push`

- Measured: arguments 6 (threshold 5)
- Recommendation: This is a Bevy system: group its queries and resources into a `#[derive(SystemParam)]` struct.

### [x] 🔴 Q87: complexity — crates/crossfir3d/src/signs.rs:57 `icon`

- Measured: complexity 12 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [x] 🔴 Q88: length — crates/crossfir3d/src/sky.rs:91 `spawn_sky`

- Measured: length 60 (threshold 50)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [x] 🔴 Q89: arguments — crates/crossfir3d/src/ui.rs:125 `capture_bind_key`

- Measured: arguments 6 (threshold 5)
- Recommendation: This is a Bevy system: group its queries and resources into a `#[derive(SystemParam)]` struct.

### [x] 🔴 Q90: arguments — crates/crossfir3d/src/ui.rs:180 `login_screen`

- Measured: arguments 6 (threshold 5)
- Recommendation: This is a Bevy system: group its queries and resources into a `#[derive(SystemParam)]` struct.

### [ ] 🔴 Q91: arguments — crates/crossfir3d/src/world.rs:380 `camera_transform`

- Measured: arguments 6 (threshold 5)
- Recommendation: This is a Bevy system: group its queries and resources into a `#[derive(SystemParam)]` struct.

### [ ] 🔴 Q92: complexity, arguments — crates/crossfir3d/src/world.rs:1447 `hide_entered`

- Measured: complexity 11 (threshold 10); arguments 6 (threshold 5)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q93: arguments — crates/crossfir3d/src/world.rs:1508 `lighting`

- Measured: arguments 6 (threshold 5)
- Recommendation: This is a Bevy system: group its queries and resources into a `#[derive(SystemParam)]` struct.

### [x] 🔴 Q94: arguments — scripts/obj2gltf.py:62 `add`

- Measured: arguments 6 (threshold 5)
- Recommendation: This is a Bevy system: group its queries and resources into a `#[derive(SystemParam)]` struct.

### [ ] 🔴 Q95: length — crates/crossfir3d/src/races.rs:93 `classify_player_face`

- Measured: length 59 (threshold 50)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q96: file size — crates/crossfir3d/src/models.rs

- Measured: 579 lines (threshold 500)
- Recommendation: Move the model-list tests into `models/tests.rs`; the code itself is under the limit.

### [ ] 🔴 Q97: complexity — crates/crossfir3d/src/buildings/castle.rs:76 `moat`

- Measured: complexity 11 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q98: complexity — crates/crossfir3d/src/clock.rs:17 `parse_time_reply`

- Measured: complexity 11 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q99: complexity — crates/crossfir3d/src/races.rs:154 `clothes_for_class`

- Measured: complexity 11 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q100: complexity — crates/crossfir3d/src/signs.rs:75 `sign_pixels`

- Measured: complexity 11 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [x] 🔴 Q101: complexity — crates/crossfir3d/tests/live_server.rs:192 `play_as`

- Measured: complexity 11 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q102: length — crates/crossfir3d/src/buildings/terrain.rs:74 `volcano`

- Measured: length 53 (threshold 50)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

## Marginal (🟡) — close to the limit, no action required

- Complexity 10 (limit 10) — crates/cfproto/examples/dump_map2.rs:2 `main`
- Nesting 4 (limit 4) — crates/cfproto/examples/dump_map2.rs:2 `main`
- Complexity 10 (limit 10) — crates/cfproto/src/items.rs:123 `apply_upd`
- Complexity 10 (limit 10) — crates/cfproto/src/newchar.rs:96 `read_field`
- Complexity 10 (limit 10) — crates/cfproto/src/server/commands.rs:110 `set_character_field`
- Complexity 10 (limit 10) — crates/cfproto/src/server/commands.rs:282 `parse_sound2`
- Arguments 5 (limit 5) — crates/cfproto/src/spells.rs:61 `update`
- Arguments 5 (limit 5) — crates/crossfir3d/src/audio.rs:85 `toggle_mute`
- Complexity 10 (limit 10) — crates/crossfir3d/src/audio.rs:131 `play_queued`
- Arguments 5 (limit 5) — crates/crossfir3d/src/audio.rs:131 `play_queued`
- Complexity 10 (limit 10) — crates/crossfir3d/src/buildings/dispatch.rs:30 `masked`
- Nesting 4 (limit 4) — crates/crossfir3d/src/buildings/props.rs:7 `flames`
- Length 47 (limit 50) — crates/crossfir3d/src/buildings/props.rs:170 `machine`
- Arguments 5 (limit 5) — crates/crossfir3d/src/camera.rs:50 `zoom`
- Arguments 5 (limit 5) — crates/crossfir3d/src/characters_ui.rs:123 `new_character`
- Arguments 5 (limit 5) — crates/crossfir3d/src/characters_ui.rs:159 `pick_list`
- Nesting 4 (limit 4) — crates/crossfir3d/src/characters_ui.rs:241 `stats_grid`
- Arguments 5 (limit 5) — crates/crossfir3d/src/characters_ui.rs:264 `stats_column`
- Complexity 10 (limit 10) — crates/crossfir3d/src/characters_ui.rs:304 `dev_autofill`
- Complexity 10 (limit 10) — crates/crossfir3d/src/graphics.rs:126 `keep_window_on_screen`
- Arguments 5 (limit 5) — crates/crossfir3d/src/hat/wear.rs:215 `put_on`
- Arguments 5 (limit 5) — crates/crossfir3d/src/hud.rs:17 `bar`
- Arguments 5 (limit 5) — crates/crossfir3d/src/input.rs:241 `watch_ground`
- Complexity 10 (limit 10) — crates/crossfir3d/src/input/world_keys.rs:105 `screen_act`
- Complexity 10 (limit 10) — crates/crossfir3d/src/input/world_keys.rs:140 `world_act`
- Arguments 5 (limit 5) — crates/crossfir3d/src/inventory_ui.rs:24 `item_rows`
- Complexity 10 (limit 10) — crates/crossfir3d/src/inventory_ui.rs:136 `item_commands`
- Nesting 4 (limit 4) — crates/crossfir3d/src/inventory_ui.rs:136 `item_commands`
- Arguments 5 (limit 5) — crates/crossfir3d/src/inventory_ui.rs:193 `row`
- Arguments 5 (limit 5) — crates/crossfir3d/src/inventory_ui.rs:263 `draw_columns`
- Arguments 5 (limit 5) — crates/crossfir3d/src/inventory_ui.rs:272 `draw_window`
- Arguments 5 (limit 5) — crates/crossfir3d/src/keys_ui.rs:372 `draw_cap`
- Arguments 5 (limit 5) — crates/crossfir3d/src/keys_ui.rs:525 `scroll_list`
- Arguments 5 (limit 5) — crates/crossfir3d/src/keys_ui.rs:566 `apply_clicks`
- Arguments 5 (limit 5) — crates/crossfir3d/src/keys_ui.rs:586 `click_key`
- Arguments 5 (limit 5) — crates/crossfir3d/src/keys_ui.rs:616 `apply`
- Arguments 5 (limit 5) — crates/crossfir3d/src/keys_ui.rs:648 `bind_press`
- Complexity 10 (limit 10) — crates/crossfir3d/src/models.rs:222 `parse_plain`
- Complexity 10 (limit 10) — crates/crossfir3d/src/models.rs:256 `parse_person`
- Arguments 5 (limit 5) — crates/crossfir3d/src/robe/dress.rs:95 `new`
- Arguments 5 (limit 5) — crates/crossfir3d/src/sky.rs:47 `sky_toward`
- Arguments 5 (limit 5) — crates/crossfir3d/src/slots.rs:32 `slot_commands`
- Complexity 10 (limit 10) — crates/crossfir3d/src/slots.rs:102 `save_slots`
- Complexity 10 (limit 10) — crates/crossfir3d/src/walls.rs:34 `piece_for`
- Arguments 5 (limit 5) — crates/crossfir3d/src/world.rs:375 `camera_eye`
- Nesting 4 (limit 4) — crates/crossfir3d/src/world.rs:402 `load_textures`
- Arguments 5 (limit 5) — crates/crossfir3d/src/world.rs:528 `new`
- Arguments 5 (limit 5) — crates/crossfir3d/src/world.rs:597 `mark_solid`
- Arguments 5 (limit 5) — crates/crossfir3d/src/world.rs:636 `remembered`
- Arguments 5 (limit 5) — crates/crossfir3d/src/world.rs:667 `find_orphan`
- Arguments 5 (limit 5) — crates/crossfir3d/src/world.rs:693 `reuse_orphan`
- Arguments 5 (limit 5) — crates/crossfir3d/src/world.rs:767 `mark_owner_or_class`
- Complexity 10 (limit 10) — crates/crossfir3d/src/world.rs:777 `spawn_person`
- Arguments 5 (limit 5) — crates/crossfir3d/src/world.rs:853 `stand_in`
- Arguments 5 (limit 5) — crates/crossfir3d/src/world.rs:934 `add_ground_side`
- Nesting 4 (limit 4) — crates/crossfir3d/src/world.rs:984 `add_pending_model`
- Arguments 5 (limit 5) — crates/crossfir3d/src/world.rs:984 `add_pending_model`
- Arguments 5 (limit 5) — crates/crossfir3d/src/world.rs:1043 `sync_visuals`
- Arguments 5 (limit 5) — crates/crossfir3d/src/world.rs:1084 `sign_material`
- Nesting 4 (limit 4) — crates/crossfir3d/src/world.rs:1255 `reveal_assembled`
- Arguments 5 (limit 5) — crates/crossfir3d/src/world.rs:1255 `reveal_assembled`
- Nesting 4 (limit 4) — scripts/pick_models.py:132 `move_uvs`
- Complexity 10 (limit 10) — scripts/pick_models.py:158 `unpack_glb`
- Arguments 5 (limit 5) — experiments/sprite3d/ai3d.py:142 `convert`
- Complexity 10 (limit 10) — experiments/sprite3d/catalog.py:246 `catalog`
- Complexity 10 (limit 10) — experiments/sprite3d/catalog.py:272 `main`
- Arguments 5 (limit 5) — experiments/sprite3d/voxel.py:86 `write_glb`
- File size 493 (limit 500) — crates/crossfir3d/src/back.rs
- File size 490 (limit 500) — crates/crossfir3d/src/keys.rs
- File size 453 (limit 500) — crates/crossfir3d/src/settings.rs

## Top recommendations

1. Group the remaining large Bevy systems' queries into `SystemParam` structs (`send_moves`, `wear_helmet`, `watch`, `update_sky`, `dress_people`, `hud`): argument count is the one metric still 🔴.
2. Split `world.rs` (1,617 lines) and `characters3d.rs` (1,075 lines) into submodules, as `buildings/` now is.
3. Turn the long data `match`es into tables: `held::class_look`, `buildings::item`, `buildings::castle`.
4. Add one shared overlay frame for the settings, inventory and key bindings screens.
5. Replace the 4 query `unwrap()`s with `let Ok(..) = ... else`, so a missing component cannot crash the game.
