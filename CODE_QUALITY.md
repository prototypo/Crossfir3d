# Code Quality Report

Generated: 2026-10-23 by quality-report v0.4.1

## Summary

- Languages analyzed: Rust (crates/cfproto, crates/crossfir3d), Python (scripts/, experiments/sprite3d/)
- Files analyzed: 64 (53 Rust, 11 Python)
- Tools used:
  - rust-code-analysis-cli 0.0.25
  - cargo clippy 0.1.98 (unwrap_used, expect_used, excessive_nesting at 4)
  - rustc dead_code
  - radon 6.0.1
  - ruff 0.6.7
  - vulture 2.16 (run one file at a time: on both folders together it crashed with a recursion error)
  - function_metrics.py (quality-report)
- Tools missing: none. Rust nesting depth and comment presence come from a brace and comment scan of each function (estimated).
- Findings: 117 open
- Branch decision: quality-improvements
- Scope notes: Rust `mod tests` blocks are left out of the function counts but kept in file sizes. The integration tests in `crates/crossfir3d/tests/` are counted. Many Bevy systems take many arguments because each query is one argument; they are still reported.

## Metrics

| Metric | Threshold | Status | Compliance | Source |
|---|---|---|---|---|
| Cyclomatic complexity | > 10 | 🔴 worst 15.9× limit | 🟢 551 · 🟡 12 · 🔴 80 (87.6% pass) | tool |
| Function length | > 50 lines | 🔴 worst 7.4× limit | 🟢 601 · 🟡 9 · 🔴 35 (94.6% pass) | tool |
| Nesting depth | > 4 levels | 🟢 97.4% pass | 🟢 604 · 🟡 24 · 🔴 17 | tool (Rust estimated) |
| Argument count | > 5 | 🔴 worst 3× limit | 🟢 581 · 🟡 18 · 🔴 46 (92.9% pass) | tool |
| Code duplication | ≥ 5 lines repeated | 🟢 0.7% (estimated) | 2 groups | tool + judgment |
| Dead code | any | 🟢 100% (645/645) | 0 unused | tool |
| Error-handling smells | unwrap/expect outside tests; bare except | 🟡 1.0 / 1k lines | 20 Rust, 0 Python | tool |
| File size | > 500 lines | 🔴 worst 5.1× limit | 🟢 53 · 🟡 5 · 🔴 6 (90.6% pass) | tool |
| Comment quality | WHAT, not HOW, on functions > 10 lines | 🔴 70% (estimated) | 🟢 195 · 🔴 84 | judgment |
| Reusability | shared logic | 🟢 0.7% (estimated) | 2 groups | judgment |

Comment presence was counted for every function longer than 10 lines; comment quality (WHAT vs HOW) was judged on a sample: the functions with findings and the largest files. In that sample the comments that exist say WHAT, not HOW. Suppressions: 22 `#[allow(clippy::too_many_arguments / type_complexity)]` attributes, mostly in characters3d.rs; their functions are still counted above. No `#[allow(dead_code)]`, `# noqa` or bare `except:`.

## Distribution

| Metric | Compliance | Worst | Median | Std dev |
|---|---|---|---|---|
| Cyclomatic complexity | 87.6% (563/643) | 159 | 3 | 12.5 |
| Function length | 94.6% (610/645) | 368 | 9 | 29.4 |
| Nesting depth | 97.4% (628/645) | 10 | 1 | 1.3 |
| Argument count | 92.9% (599/645) | 15 | 2 | 2.3 |
| File size | 90.6% (58/64) | 2,566 | 220.5 | 365.3 |

By language: Rust complexity 87.4% (worst 159), length 94.3% (worst 368), nesting 97.6% (worst 10), arguments 92.4% (worst 15), file size 88.7% (worst 2,566). Python complexity 89.6% (worst 29), length 98.0% (worst 71), nesting 94.0% (worst 7), arguments 98.0% (worst 6), file size 100% (worst 327).

### Counts lie when the denominator moves

Fixing these findings adds functions and files, so a raw count of violations can rise
while the code gets better. When comparing with a later run, lead with the compliance
ratio, quote the worst value and the standard deviation, and say whether the
denominator grew.

## Findings (worst first)

### [x] 🔴 Q1: complexity, length — crates/crossfir3d/src/buildings.rs:155 `parts`

- Measured: complexity 159 (threshold 10); length 272 (threshold 50)
- Recommendation: Split the recipe `match` into family tables: one function per family (village, castle, item, furniture), each a small match, and keep `parts` as the dispatcher.

### [x] 🔴 Q2: complexity, length, nesting, arguments — crates/crossfir3d/src/world.rs:471 `sync_visuals`

- Measured: complexity 135 (threshold 10); length 368 (threshold 50); nesting 6 (threshold 4); arguments 12 (threshold 5)
- Recommendation: Pull each `if ... Visual::X` spawn branch (beast, monster, skeleton, person, statue, building, model, card) into its own `spawn_<kind>` function; the loop then only decides which one to call.

### [x] 🔴 Q3: complexity, length, nesting, arguments — crates/crossfir3d/src/keys_ui.rs:179 `keys_screen`

- Measured: complexity 128 (threshold 10); length 338 (threshold 50); nesting 10 (threshold 4); arguments 10 (threshold 5)
- Recommendation: Split into `draw_bindings_list`, `draw_key_capture` and `draw_quick_slots` helpers; most of the depth comes from closures inside closures.

### [x] 🔴 Q4: complexity, length, nesting — crates/cfproto/src/server.rs:76 `parse`

- Measured: complexity 110 (threshold 10); length 152 (threshold 50); nesting 6 (threshold 4)
- Recommendation: Move each server command's body into its own `parse_<command>` function; `parse` keeps only the `match` on the command word.

### [x] 🔴 Q5: complexity, length, arguments — crates/crossfir3d/src/inventory_ui.rs:74 `inventory_screen`

- Measured: complexity 61 (threshold 10); length 168 (threshold 50); arguments 12 (threshold 5)
- Recommendation: Draw each of the three columns with one `item_column` function; move the key handling into its own function.

### [x] 🔴 Q6: complexity, length — crates/crossfir3d/src/models.rs:186 `parse_visual`

- Measured: complexity 57 (threshold 10); length 67 (threshold 50)
- Recommendation: Give each visual kind (model, building, wall, person, beast, monster, skeleton, statue) its own small parser called from one `match`.

### [x] 🔴 Q7: complexity, length, nesting — crates/crossfir3d/src/characters_ui.rs:123 `new_character`

- Measured: complexity 53 (threshold 10); length 156 (threshold 50); nesting 6 (threshold 4)
- Recommendation: Split into the race/class pickers, the stats panel and the start-place picker.

### [x] 🔴 Q8: complexity, length, nesting — crates/crossfir3d/src/main.rs:245 `pump_network`

- Measured: complexity 52 (threshold 10); length 119 (threshold 50); nesting 6 (threshold 4)
- Recommendation: Handle each `ServerMsg` variant in its own function; `pump_network` keeps the read loop and the `match`.

### [x] 🔴 Q9: file size — crates/crossfir3d/src/buildings.rs

- Measured: 2,566 lines (threshold 500)
- Recommendation: Move the families into submodules: `buildings/village.rs`, `castle.rs`, `items.rs`, `furniture.rs`, `walls.rs`, with the shared `Part` helpers in `buildings/mod.rs`.

### [x] 🔴 Q10: complexity, length, arguments — crates/crossfir3d/src/robe.rs:202 `dress_robes`

- Measured: complexity 43 (threshold 10); length 122 (threshold 50); arguments 10 (threshold 5)
- Recommendation: Split the robe, back piece, over piece and rags into one function each.

### [ ] 🔴 Q11: complexity, length, arguments — crates/crossfir3d/src/input.rs:268 `world_keys`

- Measured: complexity 40 (threshold 10); length 105 (threshold 50); arguments 15 (threshold 5)
- Recommendation: Group the queries and resources into one or two `SystemParam` structs; split movement keys from action keys.

### [ ] 🔴 Q12: complexity, length, arguments — crates/crossfir3d/src/hat.rs:365 `put_on`

- Measured: complexity 35 (threshold 10); length 142 (threshold 50); arguments 8 (threshold 5)
- Recommendation: Give each hat family its own builder function, chosen by one `match`.

### [ ] 🔴 Q13: complexity, nesting — crates/cfproto/src/newchar.rs:49 `parse_arch_info`

- Measured: complexity 33 (threshold 10); nesting 5 (threshold 4)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q14: complexity — crates/crossfir3d/src/held.rs:198 `class_look`

- Measured: complexity 33 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q15: complexity — crates/crossfir3d/src/buildings.rs:498 `castle`

- Measured: complexity 31 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q16: complexity, length, nesting, arguments — crates/crossfir3d/src/characters_ui.rs:31 `characters_screen`

- Measured: complexity 29 (threshold 10); length 91 (threshold 50); nesting 5 (threshold 4); arguments 7 (threshold 5)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q17: complexity — experiments/sprite3d/catalog.py:208 `classify`

- Measured: complexity 29 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q18: complexity, length — crates/crossfir3d/src/login.rs:77 `on_message`

- Measured: complexity 28 (threshold 10); length 67 (threshold 50)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q19: complexity, length, nesting, arguments — crates/crossfir3d/src/characters3d.rs:789 `dress_people`

- Measured: complexity 28 (threshold 10); length 72 (threshold 50); nesting 5 (threshold 4); arguments 10 (threshold 5)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q20: complexity, length, arguments — crates/crossfir3d/src/input.rs:89 `send_moves`

- Measured: complexity 17 (threshold 10); length 66 (threshold 50); arguments 14 (threshold 5)
- Recommendation: Group the queries and resources into a `SystemParam` struct.

### [ ] 🔴 Q21: file size — crates/crossfir3d/src/world.rs

- Measured: 1,392 lines (threshold 500)
- Recommendation: Move model spawning (`spawn_part`, `spawn_beast`, `tint_model`) into `spawn.rs` and the camera/lighting systems into their own module.

### [ ] 🔴 Q22: complexity, length, nesting, arguments — crates/crossfir3d/src/settings_ui.rs:27 `settings_screen`

- Measured: complexity 22 (threshold 10); length 135 (threshold 50); nesting 5 (threshold 4); arguments 6 (threshold 5)
- Recommendation: Draw each settings group (graphics, sound, controls) in its own function.

### [ ] 🔴 Q23: complexity, length — scripts/obj2gltf.py:29 `convert`

- Measured: complexity 27 (threshold 10); length 71 (threshold 50)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q24: complexity, length, arguments — crates/crossfir3d/src/combat_fx.rs:48 `watch`

- Measured: complexity 24 (threshold 10); length 66 (threshold 50); arguments 13 (threshold 5)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q25: complexity, length, nesting — crates/crossfir3d/src/net.rs:39 `connect`

- Measured: complexity 26 (threshold 10); length 61 (threshold 50); nesting 5 (threshold 4)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q26: complexity — crates/crossfir3d/src/held.rs:66 `worn`

- Measured: complexity 26 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q27: complexity, arguments — crates/crossfir3d/src/characters3d.rs:469 `wear_helmet`

- Measured: complexity 12 (threshold 10); arguments 13 (threshold 5)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q28: length, arguments — crates/crossfir3d/src/sky.rs:153 `update_sky`

- Measured: length 62 (threshold 50); arguments 13 (threshold 5)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q29: complexity, length — crates/crossfir3d/src/buildings.rs:1708 `item`

- Measured: complexity 26 (threshold 10); length 71 (threshold 50)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q30: complexity, nesting — crates/cfproto/src/stats.rs:73 `apply`

- Measured: complexity 25 (threshold 10); nesting 5 (threshold 4)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q31: complexity — crates/crossfir3d/src/modes.rs:68 `act`

- Measured: complexity 25 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q32: complexity, length, nesting, arguments — crates/crossfir3d/src/hud.rs:70 `hud`

- Measured: complexity 25 (threshold 10); length 95 (threshold 50); nesting 6 (threshold 4); arguments 9 (threshold 5)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q33: complexity, length, arguments — crates/crossfir3d/src/world.rs:887 `spawn_part`

- Measured: complexity 25 (threshold 10); length 106 (threshold 50); arguments 8 (threshold 5)
- Recommendation: Move each `Shape` arm with more than a few lines (lamps, mist, models, signs) into its own function.

### [ ] 🔴 Q34: complexity, length, nesting — crates/cfproto/src/map2.rs:25 `parse_map2`

- Measured: complexity 24 (threshold 10); length 51 (threshold 50); nesting 7 (threshold 4)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q35: complexity, arguments — crates/crossfir3d/src/characters3d.rs:381 `hold_items`

- Measured: complexity 15 (threshold 10); arguments 12 (threshold 5)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q36: complexity, length — crates/crossfir3d/src/ui.rs:56 `typed_bind`

- Measured: complexity 22 (threshold 10); length 66 (threshold 50)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q37: complexity, length, arguments — crates/crossfir3d/src/ui.rs:294 `command_line`

- Measured: complexity 14 (threshold 10); length 56 (threshold 50); arguments 11 (threshold 5)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q38: complexity — crates/crossfir3d/src/held.rs:15 `held_model`

- Measured: complexity 22 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q39: complexity — crates/crossfir3d/src/input.rs:156 `act_key`

- Measured: complexity 22 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q40: complexity, length — crates/crossfir3d/src/back.rs:45 `tabard_straps`

- Measured: complexity 22 (threshold 10); length 66 (threshold 50)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q41: complexity, length, arguments — crates/crossfir3d/src/world.rs:1076 `tint_model`

- Measured: complexity 14 (threshold 10); length 51 (threshold 50); arguments 11 (threshold 5)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q42: complexity, length, arguments — crates/crossfir3d/src/world.rs:1161 `follow_player`

- Measured: complexity 12 (threshold 10); length 57 (threshold 50); arguments 11 (threshold 5)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q43: complexity, nesting — experiments/sprite3d/catalog.py:114 `parse_arch`

- Measured: complexity 22 (threshold 10); nesting 7 (threshold 4)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q44: file size — crates/crossfir3d/src/characters3d.rs

- Measured: 1,074 lines (threshold 500)
- Recommendation: Split the dressing systems (`dress_people`, `wear_helmet`, `gear_up`) from the body spawning and animation.

### [ ] 🔴 Q45: complexity — crates/crossfir3d/tests/live_server.rs:77 `pick_up_and_drop`

- Measured: complexity 21 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q46: complexity — crates/cfproto/src/items.rs:83 `parse_upditem`

- Measured: complexity 21 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q47: complexity, length — crates/crossfir3d/src/skeleton.rs:26 `bones`

- Measured: complexity 21 (threshold 10); length 105 (threshold 50)
- Recommendation: Build each limb with one shared helper; the arms and legs repeat the same pattern.

### [ ] 🔴 Q48: complexity — crates/cfproto/src/stats.rs:38 `parse_stats`

- Measured: complexity 20 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q49: complexity — crates/cfproto/src/client.rs:33 `encode`

- Measured: complexity 20 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q50: arguments — crates/crossfir3d/src/characters3d.rs:551 `gear_up`

- Measured: arguments 10 (threshold 5)
- Recommendation: This is a Bevy system: group its queries and resources into a `#[derive(SystemParam)]` struct.

### [ ] 🔴 Q51: complexity — crates/crossfir3d/src/races.rs:35 `look_for_race`

- Measured: complexity 19 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q52: complexity — crates/crossfir3d/src/newchar.rs:59 `on_reply`

- Measured: complexity 19 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q53: complexity, arguments — crates/crossfir3d/src/characters3d.rs:242 `animate_beasts`

- Measured: complexity 19 (threshold 10); arguments 8 (threshold 5)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q54: complexity — crates/crossfir3d/src/buildings.rs:2229 `wall_parts`

- Measured: complexity 19 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q55: complexity, arguments — crates/crossfir3d/src/characters3d.rs:319 `bind_bones`

- Measured: complexity 16 (threshold 10); arguments 9 (threshold 5)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q56: complexity — crates/cfproto/src/spells.rs:23 `parse_addspell`

- Measured: complexity 17 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q57: complexity — crates/crossfir3d/src/markup.rs:16 `colour`

- Measured: complexity 17 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q58: complexity — crates/crossfir3d/src/settings.rs:127 `from_text`

- Measured: complexity 17 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q59: complexity — crates/crossfir3d/src/keys.rs:146 `game_key`

- Measured: complexity 16 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q60: complexity, arguments — crates/crossfir3d/src/skeleton.rs:138 `build_skeletons`

- Measured: complexity 16 (threshold 10); arguments 8 (threshold 5)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q61: arguments — crates/crossfir3d/src/characters3d.rs:589 `put_on_head`

- Measured: arguments 8 (threshold 5)
- Recommendation: This is a Bevy system: group its queries and resources into a `#[derive(SystemParam)]` struct.

### [ ] 🔴 Q62: arguments — crates/crossfir3d/src/characters3d.rs:640 `put_in_hand`

- Measured: arguments 8 (threshold 5)
- Recommendation: This is a Bevy system: group its queries and resources into a `#[derive(SystemParam)]` struct.

### [ ] 🔴 Q63: error handling — `unwrap()` on game queries: crates/crossfir3d/src/characters3d.rs:358, crates/crossfir3d/src/robe.rs:221, crates/crossfir3d/src/world.rs:570

- Measured: 3 query `get(...).unwrap()` calls that can panic if an entity loses a component; 17 more `unwrap`/`expect` calls guard invariants (fixed-size byte slices, built-in meshes, constant styles). 20 in all, outside tests: 1.0 per 1,000 lines
- Recommendation: Replace the 3 query unwraps with `let Ok(..) = ... else { continue/return }`. Give the remaining invariant `unwrap()`s an `expect("why it cannot fail")` (server.rs:249, 252; sky.rs:92, 106; keys_ui.rs:133; hat.rs:386).

### [ ] 🔴 Q64: complexity, nesting — crates/cfproto/src/view.rs:31 `apply`

- Measured: complexity 12 (threshold 10); nesting 6 (threshold 4)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q65: complexity — crates/crossfir3d/src/markup.rs:39 `apply`

- Measured: complexity 15 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q66: complexity — crates/crossfir3d/src/characters3d.rs:606 `grip`

- Measured: complexity 15 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q67: complexity, length — crates/crossfir3d/src/characters3d.rs:682 `staff_top`

- Measured: complexity 15 (threshold 10); length 70 (threshold 50)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q68: complexity — crates/crossfir3d/src/buildings.rs:443 `village`

- Measured: complexity 15 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q69: complexity — experiments/sprite3d/auto.py:37 `main`

- Measured: complexity 15 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q70: reusability — the overlay window frame is copied in 3 places: crates/crossfir3d/src/settings_ui.rs:49, crates/crossfir3d/src/inventory_ui.rs:183, crates/crossfir3d/src/keys_ui.rs:237

- Measured: 9 near-identical lines × 3 (`egui::Frame` fill, gold stroke, radius, margin, then a fixed, untitled, unmovable `egui::Window` over the screen)
- Recommendation: Add one `overlay(ctx, name, margin, |ui| ...)` function in `ui.rs` that builds the frame and window; the three screens call it.

### [ ] 🔴 Q71: complexity — crates/crossfir3d/tests/live_server.rs:141 `create_chosen_character`

- Measured: complexity 14 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q72: complexity — crates/crossfir3d/src/keys.rs:169 `reserved`

- Measured: complexity 14 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q73: complexity — crates/crossfir3d/src/keys.rs:225 `parse_typed`

- Measured: complexity 14 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q74: arguments — crates/crossfir3d/src/characters3d.rs:159 `spawn_character`

- Measured: arguments 7 (threshold 5)
- Recommendation: This is a Bevy system: group its queries and resources into a `#[derive(SystemParam)]` struct.

### [ ] 🔴 Q75: arguments — crates/crossfir3d/src/characters3d.rs:188 `spawn_monster`

- Measured: arguments 7 (threshold 5)
- Recommendation: This is a Bevy system: group its queries and resources into a `#[derive(SystemParam)]` struct.

### [ ] 🔴 Q76: arguments — crates/crossfir3d/src/characters3d.rs:213 `spawn_beast`

- Measured: arguments 7 (threshold 5)
- Recommendation: This is a Bevy system: group its queries and resources into a `#[derive(SystemParam)]` struct.

### [ ] 🔴 Q77: arguments — crates/crossfir3d/src/characters3d.rs:959 `update_preview`

- Measured: arguments 7 (threshold 5)
- Recommendation: This is a Bevy system: group its queries and resources into a `#[derive(SystemParam)]` struct.

### [ ] 🔴 Q78: arguments — crates/crossfir3d/src/main.rs:403 `dev_commands`

- Measured: arguments 7 (threshold 5)
- Recommendation: This is a Bevy system: group its queries and resources into a `#[derive(SystemParam)]` struct.

### [ ] 🔴 Q79: complexity, arguments — crates/crossfir3d/src/back.rs:161 `panel`

- Measured: complexity 13 (threshold 10); arguments 7 (threshold 5)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q80: reusability — creature spawn branches repeat in `sync_visuals`: crates/crossfir3d/src/world.rs:677, 690, 699

- Measured: 5–6 similar lines × 3 (position in the middle of the square, scene load, `Transform`, `Glide`, `Assembled`, then spawn and `continue`)
- Recommendation: Fold into one `spawn_creature(kind, tile, ...)` function; this also shortens `sync_visuals` (Q2).

### [ ] 🔴 Q81: complexity — crates/crossfir3d/src/signs.rs:39 `paint`

- Measured: complexity 13 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q82: complexity, arguments — crates/crossfir3d/src/graphics.rs:79 `apply_settings`

- Measured: complexity 13 (threshold 10); arguments 6 (threshold 5)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q83: complexity — crates/crossfir3d/src/buildings.rs:1916 `chess_piece`

- Measured: complexity 13 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q84: complexity — scripts/pick_models.py:193 `copy_model`

- Measured: complexity 13 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q85: comment quality — 34 of the 107 flagged functions have no comment saying what they do

- Measured: Missing on, among others: `sync_visuals`, `keys_screen`, `parse` (server.rs:76), `inventory_screen`, `pump_network`, `spawn_part`, `settings_screen`, `new_character`, `hud`, `connect`, `parse_map2`. Overall 195 of 279 functions longer than 10 lines have a comment (70%, estimated)
- Recommendation: Add one `///` line above each saying WHAT it does ("Draws the key bindings overlay and handles its keys"), not how.

### [ ] 🔴 Q86: nesting — scripts/pick_models.py:240 `grey_copies`

- Measured: nesting 5 (threshold 4)
- Recommendation: Return early or extract the innermost block into a helper to cut the nesting.

### [ ] 🔴 Q87: nesting — experiments/sprite3d/catalog.py:196 `map_faces`

- Measured: nesting 5 (threshold 4)
- Recommendation: Return early or extract the innermost block into a helper to cut the nesting.

### [ ] 🔴 Q88: length — crates/crossfir3d/src/main.rs:138 `main`

- Measured: length 61 (threshold 50)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q89: file size — crates/crossfir3d/src/hat.rs

- Measured: 604 lines (threshold 500)
- Recommendation: Move each hat family's builder to its own file under `hat/`, or move the long tests to `hat/tests.rs`.

### [ ] 🔴 Q90: complexity — crates/cfproto/src/items.rs:69 `parse_item2`

- Measured: complexity 12 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q91: complexity — crates/crossfir3d/src/signs.rs:57 `icon`

- Measured: complexity 12 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q92: arguments — crates/crossfir3d/src/ui.rs:125 `capture_bind_key`

- Measured: arguments 6 (threshold 5)
- Recommendation: This is a Bevy system: group its queries and resources into a `#[derive(SystemParam)]` struct.

### [ ] 🔴 Q93: arguments — crates/crossfir3d/src/ui.rs:180 `login_screen`

- Measured: arguments 6 (threshold 5)
- Recommendation: This is a Bevy system: group its queries and resources into a `#[derive(SystemParam)]` struct.

### [ ] 🔴 Q94: complexity — crates/crossfir3d/src/keys.rs:281 `save_bindings`

- Measured: complexity 12 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q95: arguments — crates/crossfir3d/src/graphics.rs:153 `remember_window`

- Measured: arguments 6 (threshold 5)
- Recommendation: This is a Bevy system: group its queries and resources into a `#[derive(SystemParam)]` struct.

### [ ] 🔴 Q96: arguments — crates/crossfir3d/src/graphics.rs:260 `fit_scene_image`

- Measured: arguments 6 (threshold 5)
- Recommendation: This is a Bevy system: group its queries and resources into a `#[derive(SystemParam)]` struct.

### [ ] 🔴 Q97: arguments — crates/crossfir3d/src/characters3d.rs:426 `wear_armour`

- Measured: arguments 6 (threshold 5)
- Recommendation: This is a Bevy system: group its queries and resources into a `#[derive(SystemParam)]` struct.

### [ ] 🔴 Q98: complexity, arguments — crates/crossfir3d/src/characters3d.rs:882 `animate`

- Measured: complexity 12 (threshold 10); arguments 6 (threshold 5)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q99: arguments — crates/crossfir3d/src/messages.rs:38 `push`

- Measured: arguments 6 (threshold 5)
- Recommendation: This is a Bevy system: group its queries and resources into a `#[derive(SystemParam)]` struct.

### [ ] 🔴 Q100: length — crates/crossfir3d/src/sky.rs:91 `spawn_sky`

- Measured: length 60 (threshold 50)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q101: arguments — crates/crossfir3d/src/input.rs:226 `binding_commands`

- Measured: arguments 6 (threshold 5)
- Recommendation: This is a Bevy system: group its queries and resources into a `#[derive(SystemParam)]` struct.

### [ ] 🔴 Q102: arguments — crates/crossfir3d/src/world.rs:379 `camera_transform`

- Measured: arguments 6 (threshold 5)
- Recommendation: This is a Bevy system: group its queries and resources into a `#[derive(SystemParam)]` struct.

### [ ] 🔴 Q103: complexity, arguments — crates/crossfir3d/src/world.rs:1222 `hide_entered`

- Measured: complexity 11 (threshold 10); arguments 6 (threshold 5)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q104: arguments — crates/crossfir3d/src/world.rs:1283 `lighting`

- Measured: arguments 6 (threshold 5)
- Recommendation: This is a Bevy system: group its queries and resources into a `#[derive(SystemParam)]` struct.

### [ ] 🔴 Q105: arguments — crates/crossfir3d/src/buildings.rs:1202 `arc`

- Measured: arguments 6 (threshold 5)
- Recommendation: This is a Bevy system: group its queries and resources into a `#[derive(SystemParam)]` struct.

### [ ] 🔴 Q106: complexity — crates/crossfir3d/src/buildings.rs:1474 `die`

- Measured: complexity 12 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q107: arguments — crates/crossfir3d/src/buildings.rs:1680 `hoop`

- Measured: arguments 6 (threshold 5)
- Recommendation: This is a Bevy system: group its queries and resources into a `#[derive(SystemParam)]` struct.

### [ ] 🔴 Q108: arguments — scripts/obj2gltf.py:62 `add`

- Measured: arguments 6 (threshold 5)
- Recommendation: This is a Bevy system: group its queries and resources into a `#[derive(SystemParam)]` struct.

### [ ] 🔴 Q109: length — crates/crossfir3d/src/races.rs:93 `classify_player_face`

- Measured: length 59 (threshold 50)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q110: file size — crates/crossfir3d/src/keys_ui.rs

- Measured: 585 lines (threshold 500)
- Recommendation: Splitting `keys_screen` (see its finding) will bring this under the limit; put the drawing helpers in `keys_ui/draw.rs` if not.

### [ ] 🔴 Q111: complexity — crates/crossfir3d/tests/live_server.rs:192 `play_as`

- Measured: complexity 11 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q112: complexity — crates/crossfir3d/src/clock.rs:17 `parse_time_reply`

- Measured: complexity 11 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q113: complexity — crates/crossfir3d/src/signs.rs:75 `sign_pixels`

- Measured: complexity 11 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q114: complexity — crates/crossfir3d/src/races.rs:154 `clothes_for_class`

- Measured: complexity 11 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q115: complexity — crates/crossfir3d/src/buildings.rs:733 `moat`

- Measured: complexity 11 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q116: length — crates/crossfir3d/src/buildings.rs:2138 `volcano`

- Measured: length 53 (threshold 50)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.

### [ ] 🔴 Q117: file size — crates/crossfir3d/src/models.rs

- Measured: 504 lines (threshold 500)
- Recommendation: Move the model-list tests into `models/tests.rs`; the code itself is under the limit.

## Marginal (🟡) — close to the limit, no action required

- Complexity 10 (limit 10) — crates/cfproto/examples/dump_map2.rs:2 `main`
- Nesting 4 (limit 4) — crates/cfproto/examples/dump_map2.rs:2 `main`
- Arguments 5 (limit 5) — crates/cfproto/src/spells.rs:61 `update`
- Complexity 10 (limit 10) — crates/cfproto/src/items.rs:123 `apply_upd`
- Arguments 5 (limit 5) — crates/crossfir3d/src/slots.rs:32 `slot_commands`
- Complexity 10 (limit 10) — crates/crossfir3d/src/slots.rs:102 `save_slots`
- Complexity 10 (limit 10) — crates/crossfir3d/src/walls.rs:34 `piece_for`
- Complexity 10 (limit 10) — crates/crossfir3d/src/graphics.rs:126 `keep_window_on_screen`
- Arguments 5 (limit 5) — crates/crossfir3d/src/audio.rs:85 `toggle_mute`
- Complexity 10 (limit 10) — crates/crossfir3d/src/audio.rs:131 `play_queued`
- Arguments 5 (limit 5) — crates/crossfir3d/src/audio.rs:131 `play_queued`
- Arguments 5 (limit 5) — crates/crossfir3d/src/inventory_ui.rs:23 `item_rows`
- Arguments 5 (limit 5) — crates/crossfir3d/src/hud.rs:17 `bar`
- Arguments 5 (limit 5) — crates/crossfir3d/src/sky.rs:47 `sky_toward`
- Arguments 5 (limit 5) — crates/crossfir3d/src/camera.rs:50 `zoom`
- Arguments 5 (limit 5) — crates/crossfir3d/src/input.rs:244 `watch_ground`
- Arguments 5 (limit 5) — crates/crossfir3d/src/world.rs:374 `camera_eye`
- Nesting 4 (limit 4) — crates/crossfir3d/src/world.rs:401 `load_textures`
- Arguments 5 (limit 5) — crates/crossfir3d/src/world.rs:859 `sign_material`
- Nesting 4 (limit 4) — crates/crossfir3d/src/world.rs:1030 `reveal_assembled`
- Arguments 5 (limit 5) — crates/crossfir3d/src/world.rs:1030 `reveal_assembled`
- Arguments 5 (limit 5) — crates/crossfir3d/src/buildings.rs:96 `part`
- Nesting 4 (limit 4) — crates/crossfir3d/src/buildings.rs:1111 `flames`
- Length 47 (limit 50) — crates/crossfir3d/src/buildings.rs:1380 `machine`
- Nesting 4 (limit 4) — scripts/pick_models.py:132 `move_uvs`
- Complexity 10 (limit 10) — scripts/pick_models.py:158 `unpack_glb`
- Arguments 5 (limit 5) — experiments/sprite3d/ai3d.py:142 `convert`
- Complexity 10 (limit 10) — experiments/sprite3d/catalog.py:246 `catalog`
- Complexity 10 (limit 10) — experiments/sprite3d/catalog.py:272 `main`
- Arguments 5 (limit 5) — experiments/sprite3d/voxel.py:86 `write_glb`
- File size 499 — crates/cfproto/src/server.rs
- File size 493 — crates/crossfir3d/src/back.rs
- File size 490 — crates/crossfir3d/src/keys.rs
- File size 462 — crates/crossfir3d/src/main.rs
- File size 453 — crates/crossfir3d/src/settings.rs

## Top recommendations

1. Split `buildings.rs::parts` (complexity 159) into per-family recipe functions and move the families into a `buildings/` module: this fixes the worst function and the worst file together.
2. Break `world.rs::sync_visuals` (complexity 135, 368 lines) into one `spawn_<kind>` function per visual kind, starting with the three repeated creature branches.
3. Split `keys_ui.rs::keys_screen` (nesting 10, 338 lines) into drawing helpers, and add one shared `overlay()` for the three overlay screens' frame.
4. Move each server command in `cfproto::server::parse` into its own function.
5. Group the large Bevy systems' queries into `SystemParam` structs; this clears most argument-count findings at once.

