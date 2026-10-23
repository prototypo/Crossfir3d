# Code Quality Report

Generated: 2026-10-24 by quality-report v0.4.1 (codequalityexplorer 0.7.1; third run, after the 100 fixes of the second report)

## Summary

- Languages analyzed: Rust (crates/cfproto, crates/crossfir3d), Python (scripts/, experiments/sprite3d/)
- Files analyzed: 96 (85 Rust, 11 Python)
- Tools used:
  - rust-code-analysis-cli 0.0.25
  - cargo clippy 0.1.98 (unwrap_used, expect_used, excessive_nesting at 4, dead_code; `--workspace --all-targets`)
  - rustc dead_code
  - radon 6.0.1
  - ruff 0.6.7 (from the pyenv `venv` environment; not on the default PATH)
  - vulture 2.16 (run one file at a time, as last time)
  - function_metrics.py (quality-report)
- Tools missing: none. Rust nesting depth and comment presence come from a brace and comment scan of each function (estimated), as last time.
- Findings: 9 open, 2 blocked
- Branch: quality-improvements
- Scope notes: skipped `target/`, `upstream/`, `dist/`, `node_modules` and `experiments/sprite3d/.venv`. Rust `#[cfg(test)]` modules (inline, and the `tests.rs` files under `buildings/`, `characters3d/`, `keys_ui/`, `models/`, `world/`) are left out of the function counts but kept in file sizes. The integration tests in `crates/*/tests/` and the example `crates/cfproto/examples/dump_map2.rs` are counted. 5,506 of the 23,375 lines are unit-test modules.

## Metrics

| Metric | Threshold | Status | Compliance | Source |
|---|---|---|---|---|
| Cyclomatic complexity | > 10 | 🟢 99.9% pass | 🟢 1040 · 🟡 35 · 🔴 1 | tool |
| Function length | > 50 lines | 🟢 100% pass | 🟢 1074 · 🟡 2 · 🔴 0 | tool |
| Nesting depth | > 4 levels | 🟢 100% pass | 🟢 1063 · 🟡 13 · 🔴 0 | tool (Rust estimated) |
| Argument count | > 5 | 🟢 99.9% pass | 🟢 993 · 🟡 82 · 🔴 1 | tool |
| Code duplication | ≥ 5 lines repeated | 🟢 0.3% (estimated) | 2 groups | tool + judgment |
| Dead code | any | 🟢 100% (1076/1076) | 0 unused | tool |
| Error-handling smells | unwrap/expect outside tests; bare except | 🟡 0.8 / 1k lines | 19 Rust, 0 Python | tool |
| File size | > 500 lines | 🟢 95.8% pass | 🟢 88 · 🟡 4 · 🔴 4 | tool |
| Comment quality | WHAT, not HOW, on functions > 10 lines | 🟢 98.6% (estimated) | 🟢 420 · 🔴 6 | judgment |
| Reusability | shared logic | 🟢 0.3% (estimated) | 2 groups | judgment |

Comment presence was counted for every function longer than 10 lines; comment quality (WHAT vs HOW) was judged on a sample: the files with findings and the largest remaining files (settings.rs, held.rs, items.rs, login.rs, hat.rs, races.rs, main.rs, input.rs, newchar.rs, catalog.py). In that sample the comments say WHAT, not HOW; the 6 functions without a comment are all integration tests. Duplication was found by hashing every 5-line window of non-test code (whitespace-normalised) and then judging the hits; the 2 real groups are about 46 lines of about 17,900. Suppressions: 2 `#[allow(clippy::type_complexity)]` (sky.rs:178, characters3d/mod.rs:386), down from 16; no `#[allow(dead_code)]`, `#[allow(clippy::too_many_arguments)]`, `# noqa` or bare `except:`. Clippy's `excessive_nesting` also fires at view.rs:37 and props.rs:14, but it counts the `impl`/`fn` block as a level; measured from the function body (the convention used in every run) both are depth 4, so they are listed as marginal.

## Distribution

| Metric | Compliance | Worst | Median | Std dev |
|---|---|---|---|---|
| Cyclomatic complexity | 99.9% (1075/1076) | 20 | 3 | 2.6 |
| Function length | 100% (1076/1076) | 48 | 9 | 7.1 |
| Nesting depth | 100% (1076/1076) | 4 | 1 | 1.0 |
| Argument count | 99.9% (1075/1076) | 6 | 2 | 1.4 |
| File size | 95.8% (92/96) | 617 | 229 | 138.1 |

By language: Rust complexity 99.9% (998 functions, worst 20), length 100% (worst 48), nesting 100% (worst 4), arguments 99.9% (worst 6), file size 95.3% (85 files, worst 617). Python complexity 100% (78 functions, worst 10), length 100% (worst 34), nesting 100% (worst 4), arguments 100% (worst 5), file size 100% (11 files, worst 392).

## Progress since the last report

Previous report: 2026-10-23 (102 findings: 100 fixed, 2 blocked).

| Metric | Compliance, before → now | Worst, before → now | Median, before → now | Std dev, before → now |
|---|---|---|---|---|
| Cyclomatic complexity | **92.2% (804/872) → 99.9% (1075/1076)** | 33 → 20 | 3 → 3 | 4.9 → 2.6 |
| Function length | **97.2% (848/872) → 100% (1076/1076)** | 135 → 48 | 9 → 9 | 13.7 → 7.1 |
| Nesting depth | **98.7% (861/872) → 100% (1076/1076)** | 7 → 4 | 1 → 1 | 1.2 → 1.0 |
| Argument count | **95.4% (832/872) → 99.9% (1075/1076)** | 14 → 6 | 2 → 2 | 1.9 → 1.4 |
| File size | **94.9% (75/79) → 95.8% (92/96)** | 1,617 → 617 | 225 → 229 | 225.7 → 138.1 |
| Dead code | 100% (872/872) → 100% (1076/1076) | | | |
| Error-handling smells | 1.0 → 0.8 per 1k lines (20 → 19) | | | |
| Comment quality | 75.5% → 98.6% (estimated) | | | |
| Duplication | 0.1% → 0.3% (estimated; 1 → 2 groups) | | | |

The denominator grew again: 872 → 1,076 functions (+204) and 79 → 96 files (+17), because the fixes split big functions and files. Every ratio rose or held, every worst value fell, and every standard deviation fell. The count of oversized files stayed at 4 while the file count rose by 17, and the largest file is now 617 lines instead of 1,617; three of the four are over only because of their inline test modules. The duplication rise is two small groups found by a new whole-code window scan, not new copying: the previous run judged duplication on a sample only.

### Counts lie when the denominator moves

Fixing these findings adds functions and files, so a raw count of violations can rise
while the code gets better. When comparing with a later run, lead with the compliance
ratio, quote the worst value and the standard deviation, and say whether the
denominator grew.

## Findings (worst first)

### [!] 🔴 Q1: complexity — crates/cfproto/src/client.rs:33 `encode`

- Measured: complexity 20 (threshold 10)
- Recommendation: Extract the separate steps into named helper functions so each one does one job.
- Blocked: a flat match over the 17 message kinds is the clearest form. Splitting it under 10 needs dead `=> Vec::new()` arms that would send an empty packet for a misplaced kind. Kept the old code; added byte-exact golden tests for every kind instead.

### [ ] 🔴 Q2: file size — crates/crossfir3d/src/keys.rs

- Measured: 617 lines (threshold 500); the code is 332 lines, the inline `mod tests` from line 333 is 285
- Recommendation: Move the test module into `keys/tests.rs` (`#[cfg(test)] mod tests;`), as `buildings/` and `keys_ui/` already do.

### [ ] 🔴 Q3: file size — crates/crossfir3d/src/ui.rs

- Measured: 566 lines (threshold 500); the code is 420 lines, the inline `mod tests` from line 421 is 146
- Recommendation: Move the test module into `ui/tests.rs`, as Q2.

### [ ] 🔴 Q4: file size — crates/crossfir3d/src/back.rs

- Measured: 563 lines (threshold 500); the code is 406 lines, the inline `mod tests` from line 407 is 157
- Recommendation: Move the test module into `back/tests.rs`, as Q2.

### [ ] 🔴 Q5: file size — crates/crossfir3d/src/characters3d/mod.rs

- Measured: 504 lines (threshold 500); its tests are already in `characters3d/tests.rs`
- Recommendation: Move the preview systems (`setup_preview`, `update_preview`, `respawn_preview`, `clear_preview`, lines 422–493) into `characters3d/preview.rs`, beside `beasts.rs`, `dress.rs` and `worn.rs`.

### [!] 🔴 Q6: arguments — crates/crossfir3d/src/buildings/props.rs:71 `arc`

- Measured: arguments 6 (threshold 5)
- Recommendation: This is a Bevy system: group its queries and resources into a `#[derive(SystemParam)]` struct.
- Blocked: `arc` is a geometry helper with 6 clear, all-varying inputs at 6 call sites; a struct would make every call harder to read without making anything safer.

### [ ] 🔴 Q7: reusability — the "write a file atomically" tail is copied in 3 places, and a 4th save skips it: crates/crossfir3d/src/keys.rs:321 `save_bindings`, crates/crossfir3d/src/slots.rs:111 `save_slots`, crates/crossfir3d/src/settings.rs:265 `save_settings`, crates/crossfir3d/src/races.rs:207 `save_body`

- Measured: 5 identical lines × 3 (`create_dir_all` of the parent, write to a `.tmp`, `rename`); `slots::save_slots` and `races::save_body` also share the same 2-line "keep other characters' lines" filter. `races::save_body` writes the file directly, so a crash there can leave half a file — the thing the other three guard against.
- Recommendation: Add one `write_atomic(path, text) -> io::Result<()>` in `settings.rs` and call it from all four saves (`faces.rs:55` can use it too).

### [ ] 🔴 Q8: reusability — crates/crossfir3d/src/characters_ui.rs:14 re-creates `ui::GOLD` and `ui::overlay_window`

- Measured: its own `GOLD` constant, a `panel()` frame that matches `ui::overlay_window`'s frame except for the fill alpha, and the same 6-line `egui::Window` builder (`title_bar(false)`, `resizable(false)`, `movable(false)`, `fixed_rect`, `frame`) written twice, at lines 104 and 174
- Recommendation: Use `ui::GOLD`, and call `ui::overlay_window` for both character screens (give it the fill as a parameter, or accept the overlay's 238 alpha).

### [ ] 🔴 Q9: error handling — 19 `unwrap()`/`expect()` outside tests: 0.8 per 1,000 lines

- Measured: 6 in crates/cfproto/examples/dump_map2.rs (lines 3, 4, 4, 7, 7, 8); 13 `expect("why")` on built-in mesh data: back.rs:123 (×2), 295, 326, 328; robe.rs:153, 155, 159; sky.rs:108, 163; hat.rs:134; hat/wear.rs:45; buildings/village.rs:66. None is on server data or a game query.
- Recommendation: Make `dump_map2`'s `main` return `Result<(), Box<dyn Error>>` and use `?`; that alone gives 0.56 per 1k. To reach 🟢 (≤ 0.5), let `back.rs`'s merge helper (326–328) and `robe.rs`'s bead merge (153–159) return `Option` and skip the part on `None`; the remaining `expect`s state real invariants and can stay.

### [ ] 🔴 Q10: comment quality — 6 integration tests longer than 10 lines have no comment: crates/cfproto/tests/captured.rs:6 `captured_packets_parse`; crates/crossfir3d/tests/live_server.rs:33 `walk_one_step`, :116 `pick_up_and_drop`, :130 `creation_data_parses`, :203 `create_chosen_character`, :248 `read_a_sign_opens_the_box`

- Measured: 420 of 426 functions longer than 10 lines have a WHAT comment (98.6%); these 6 are the rest
- Recommendation: Give each a one-line `///` saying what server behaviour it checks (the names already say most of it, so this is low priority).

### [ ] 🔴 Q11: complexity, length, nesting — frozen old code kept as test oracles: crates/crossfir3d/src/modes.rs:243 `old_act`, crates/cfproto/src/map2.rs:190 `old_parse_map2`, crates/cfproto/src/items.rs:296 `old_parse_upditem`, crates/cfproto/src/stats.rs:262 `old_parse_stats`

- Measured: complexity 25, 24, 21 and 20 (threshold 10); `old_parse_map2` also length 51 (threshold 50) and nesting 7 (threshold 4). All four sit inside `#[cfg(test)]` modules, so they are not in the counts above.
- Recommendation: Nothing to refactor: these are deliberate copies of the code before its split, kept only so tests can compare old and new behaviour. Delete all four (and the tests that call them) once the new code is trusted; that also takes about 135 lines off map2.rs, items.rs, stats.rs and modes.rs.

## Marginal (🟡) — close to the limit, no action required

- Complexity 10 (limit 10) — crates/cfproto/examples/dump_map2.rs:2 `main`
- Nesting 4 (limit 4) — crates/cfproto/examples/dump_map2.rs:2 `main`
- Complexity 10 (limit 10) — crates/cfproto/src/items.rs:80 `read_item`
- Complexity 10 (limit 10) — crates/cfproto/src/items.rs:145 `apply_upd`
- Complexity 10 (limit 10) — crates/cfproto/src/newchar.rs:96 `read_field`
- Complexity 10 (limit 10) — crates/cfproto/src/server/commands.rs:110 `set_character_field`
- Complexity 10 (limit 10) — crates/cfproto/src/server/commands.rs:282 `parse_sound2`
- Complexity 10 (limit 10) — crates/cfproto/src/spells.rs:34 `read_spell`
- Arguments 5 (limit 5) — crates/cfproto/src/spells.rs:80 `update`
- Nesting 4 (limit 4) — crates/cfproto/src/view.rs:32 `apply`
- Arguments 5 (limit 5) — crates/crossfir3d/src/audio.rs:87 `toggle_mute`
- Complexity 10 (limit 10) — crates/crossfir3d/src/audio.rs:134 `play_queued`
- Arguments 5 (limit 5) — crates/crossfir3d/src/audio.rs:134 `play_queued`
- Complexity 10 (limit 10) — crates/crossfir3d/src/back.rs:237 `panel_point`
- Complexity 10 (limit 10) — crates/crossfir3d/src/buildings/castle.rs:7 `castle`
- Arguments 5 (limit 5) — crates/crossfir3d/src/buildings/castle.rs:42 `square_walls`
- Complexity 10 (limit 10) — crates/crossfir3d/src/buildings/dispatch.rs:30 `masked`
- Arguments 5 (limit 5) — crates/crossfir3d/src/buildings/items.rs:168 `hoop`
- Arguments 5 (limit 5) — crates/crossfir3d/src/buildings/mod.rs:130 `part`
- Nesting 4 (limit 4) — crates/crossfir3d/src/buildings/props.rs:7 `flames`
- Length 47 (limit 50) — crates/crossfir3d/src/buildings/props.rs:170 `machine`
- Length 48 (limit 50) — crates/crossfir3d/src/buildings/terrain.rs:74 `volcano`
- Arguments 5 (limit 5) — crates/crossfir3d/src/buildings/village.rs:17 `wall_file`
- Arguments 5 (limit 5) — crates/crossfir3d/src/camera.rs:50 `zoom`
- Arguments 5 (limit 5) — crates/crossfir3d/src/characters_ui.rs:70 `send_choice`
- Arguments 5 (limit 5) — crates/crossfir3d/src/characters_ui.rs:100 `character_list`
- Arguments 5 (limit 5) — crates/crossfir3d/src/characters_ui.rs:169 `new_character`
- Arguments 5 (limit 5) — crates/crossfir3d/src/characters_ui.rs:205 `pick_list`
- Nesting 4 (limit 4) — crates/crossfir3d/src/characters_ui.rs:287 `stats_grid`
- Arguments 5 (limit 5) — crates/crossfir3d/src/characters_ui.rs:310 `stats_column`
- Complexity 10 (limit 10) — crates/crossfir3d/src/characters_ui.rs:350 `dev_autofill`
- Complexity 10 (limit 10) — crates/crossfir3d/src/characters3d/beasts.rs:40 `find_clips`
- Arguments 5 (limit 5) — crates/crossfir3d/src/characters3d/beasts.rs:57 `animate_beasts`
- Arguments 5 (limit 5) — crates/crossfir3d/src/characters3d/beasts.rs:136 `head_mesh`
- Arguments 5 (limit 5) — crates/crossfir3d/src/characters3d/dress.rs:29 `put_skirt`
- Complexity 10 (limit 10) — crates/crossfir3d/src/characters3d/dress.rs:44 `reshape_body`
- Nesting 4 (limit 4) — crates/crossfir3d/src/characters3d/dress.rs:44 `reshape_body`
- Arguments 5 (limit 5) — crates/crossfir3d/src/characters3d/dress.rs:62 `put_headwear`
- Arguments 5 (limit 5) — crates/crossfir3d/src/characters3d/dress.rs:77 `put_broom`
- Arguments 5 (limit 5) — crates/crossfir3d/src/characters3d/dress.rs:93 `dress_people`
- Arguments 5 (limit 5) — crates/crossfir3d/src/characters3d/mod.rs:168 `spawn_character`
- Arguments 5 (limit 5) — crates/crossfir3d/src/characters3d/mod.rs:196 `spawn_monster`
- Arguments 5 (limit 5) — crates/crossfir3d/src/characters3d/mod.rs:363 `turn_own`
- Arguments 5 (limit 5) — crates/crossfir3d/src/characters3d/worn.rs:16 `hold_class_items`
- Arguments 5 (limit 5) — crates/crossfir3d/src/characters3d/worn.rs:118 `wear_helmet`
- Arguments 5 (limit 5) — crates/crossfir3d/src/characters3d/worn.rs:142 `gear_up`
- Arguments 5 (limit 5) — crates/crossfir3d/src/characters3d/worn.rs:218 `put_in_hand`
- Complexity 10 (limit 10) — crates/crossfir3d/src/clock.rs:17 `parse_time_reply`
- Arguments 5 (limit 5) — crates/crossfir3d/src/combat_fx.rs:77 `flash_on_hp_drop`
- Arguments 5 (limit 5) — crates/crossfir3d/src/combat_fx.rs:90 `show_attacks`
- Complexity 10 (limit 10) — crates/crossfir3d/src/combat_fx.rs:108 `tint_hit`
- Arguments 5 (limit 5) — crates/crossfir3d/src/graphics.rs:117 `apply_settings`
- Complexity 10 (limit 10) — crates/crossfir3d/src/graphics.rs:142 `keep_window_on_screen`
- Complexity 10 (limit 10) — crates/crossfir3d/src/graphics.rs:177 `remember_window`
- Arguments 5 (limit 5) — crates/crossfir3d/src/graphics.rs:177 `remember_window`
- Arguments 5 (limit 5) — crates/crossfir3d/src/graphics.rs:293 `fit_scene_image`
- Arguments 5 (limit 5) — crates/crossfir3d/src/hat/wear.rs:216 `put_on`
- Complexity 10 (limit 10) — crates/crossfir3d/src/held.rs:67 `worn`
- Complexity 10 (limit 10) — crates/crossfir3d/src/held.rs:201 `class_back`
- Arguments 5 (limit 5) — crates/crossfir3d/src/hud.rs:20 `bar`
- Nesting 4 (limit 4) — crates/crossfir3d/src/hud.rs:159 `draw_log`
- Arguments 5 (limit 5) — crates/crossfir3d/src/hud.rs:159 `draw_log`
- Arguments 5 (limit 5) — crates/crossfir3d/src/input.rs:272 `watch_ground`
- Complexity 10 (limit 10) — crates/crossfir3d/src/input/world_keys.rs:110 `screen_act`
- Complexity 10 (limit 10) — crates/crossfir3d/src/input/world_keys.rs:145 `world_act`
- Complexity 10 (limit 10) — crates/crossfir3d/src/inventory_ui.rs:136 `item_commands`
- Nesting 4 (limit 4) — crates/crossfir3d/src/inventory_ui.rs:136 `item_commands`
- Arguments 5 (limit 5) — crates/crossfir3d/src/inventory_ui.rs:193 `row`
- Arguments 5 (limit 5) — crates/crossfir3d/src/inventory_ui.rs:263 `draw_columns`
- Arguments 5 (limit 5) — crates/crossfir3d/src/inventory_ui.rs:272 `draw_window`
- Arguments 5 (limit 5) — crates/crossfir3d/src/keys_ui/keyboard.rs:67 `draw_cap`
- Arguments 5 (limit 5) — crates/crossfir3d/src/keys_ui/lists.rs:44 `scroll_list`
- Arguments 5 (limit 5) — crates/crossfir3d/src/keys_ui/press.rs:6 `apply_clicks`
- Arguments 5 (limit 5) — crates/crossfir3d/src/keys_ui/press.rs:26 `click_key`
- Arguments 5 (limit 5) — crates/crossfir3d/src/keys_ui/press.rs:56 `apply`
- Arguments 5 (limit 5) — crates/crossfir3d/src/keys_ui/press.rs:88 `bind_press`
- Complexity 10 (limit 10) — crates/crossfir3d/src/login.rs:95 `on_failure`
- Arguments 5 (limit 5) — crates/crossfir3d/src/messages.rs:39 `push`
- Complexity 10 (limit 10) — crates/crossfir3d/src/models/mod.rs:224 `parse_plain`
- Complexity 10 (limit 10) — crates/crossfir3d/src/models/mod.rs:258 `parse_person`
- Nesting 4 (limit 4) — crates/crossfir3d/src/net.rs:57 `read_loop`
- Arguments 5 (limit 5) — crates/crossfir3d/src/robe/dress.rs:95 `new`
- Nesting 4 (limit 4) — crates/crossfir3d/src/signs.rs:90 `draw_text`
- Arguments 5 (limit 5) — crates/crossfir3d/src/skeleton.rs:216 `spawn_piece`
- Arguments 5 (limit 5) — crates/crossfir3d/src/sky.rs:48 `sky_toward`
- Complexity 10 (limit 10) — crates/crossfir3d/src/slots.rs:111 `save_slots`
- Arguments 5 (limit 5) — crates/crossfir3d/src/ui.rs:69 `answer_pending`
- Arguments 5 (limit 5) — crates/crossfir3d/src/ui.rs:121 `run_typed`
- Arguments 5 (limit 5) — crates/crossfir3d/src/ui.rs:151 `typed_bind`
- Complexity 10 (limit 10) — crates/crossfir3d/src/ui.rs:358 `command_line`
- Complexity 10 (limit 10) — crates/crossfir3d/src/walls.rs:34 `piece_for`
- Arguments 5 (limit 5) — crates/crossfir3d/src/world/light.rs:36 `lighting`
- Nesting 4 (limit 4) — crates/crossfir3d/src/world/load.rs:22 `load_textures`
- Nesting 4 (limit 4) — crates/crossfir3d/src/world/load.rs:97 `reveal_assembled`
- Arguments 5 (limit 5) — crates/crossfir3d/src/world/load.rs:97 `reveal_assembled`
- Arguments 5 (limit 5) — crates/crossfir3d/src/world/parts.rs:6 `sign_material`
- Arguments 5 (limit 5) — crates/crossfir3d/src/world/parts.rs:43 `spawn_slab`
- Arguments 5 (limit 5) — crates/crossfir3d/src/world/rig.rs:15 `camera_eye`
- Arguments 5 (limit 5) — crates/crossfir3d/src/world/rig.rs:27 `camera_transform`
- Arguments 5 (limit 5) — crates/crossfir3d/src/world/rig.rs:101 `pulled_distance`
- Arguments 5 (limit 5) — crates/crossfir3d/src/world/rig.rs:158 `hide_entered`
- Arguments 5 (limit 5) — crates/crossfir3d/src/world/spawn.rs:55 `mark_owner_or_class`
- Complexity 10 (limit 10) — crates/crossfir3d/src/world/spawn.rs:65 `spawn_person`
- Arguments 5 (limit 5) — crates/crossfir3d/src/world/spawn.rs:141 `stand_in`
- Arguments 5 (limit 5) — crates/crossfir3d/src/world/spawn.rs:222 `add_ground_side`
- Nesting 4 (limit 4) — crates/crossfir3d/src/world/spawn.rs:272 `add_pending_model`
- Arguments 5 (limit 5) — crates/crossfir3d/src/world/spawn.rs:272 `add_pending_model`
- Arguments 5 (limit 5) — crates/crossfir3d/src/world/sync.rs:93 `new`
- Arguments 5 (limit 5) — crates/crossfir3d/src/world/sync.rs:162 `mark_solid`
- Arguments 5 (limit 5) — crates/crossfir3d/src/world/sync.rs:201 `remembered`
- Arguments 5 (limit 5) — crates/crossfir3d/src/world/sync.rs:232 `find_orphan`
- Arguments 5 (limit 5) — crates/crossfir3d/src/world/sync.rs:258 `reuse_orphan`
- Arguments 5 (limit 5) — crates/crossfir3d/src/world/sync.rs:299 `sync_visuals`
- Arguments 5 (limit 5) — crates/crossfir3d/tests/live_server.rs:89 `wait_item_at`
- Arguments 5 (limit 5) — experiments/sprite3d/ai3d.py:145 `convert`
- Arguments 5 (limit 5) — experiments/sprite3d/catalog.py:234 `rule_existing`
- Arguments 5 (limit 5) — experiments/sprite3d/catalog.py:246 `rule_icon_or_generator`
- Arguments 5 (limit 5) — experiments/sprite3d/catalog.py:256 `rule_folder_first`
- Arguments 5 (limit 5) — experiments/sprite3d/catalog.py:265 `rule_flags`
- Arguments 5 (limit 5) — experiments/sprite3d/catalog.py:274 `rule_type`
- Complexity 10 (limit 10) — experiments/sprite3d/catalog.py:284 `rule_fallback`
- Arguments 5 (limit 5) — experiments/sprite3d/catalog.py:284 `rule_fallback`
- Arguments 5 (limit 5) — experiments/sprite3d/catalog.py:300 `classify`
- Complexity 10 (limit 10) — experiments/sprite3d/catalog.py:310 `catalog`
- Complexity 10 (limit 10) — experiments/sprite3d/catalog.py:336 `main`
- Arguments 5 (limit 5) — experiments/sprite3d/voxel.py:87 `write_glb`
- Complexity 10 (limit 10) — scripts/obj2gltf.py:44 `read_obj`
- Arguments 5 (limit 5) — scripts/obj2gltf.py:94 `build_primitives`
- Arguments 5 (limit 5) — scripts/obj2gltf.py:108 `write_gltf`
- Nesting 4 (limit 4) — scripts/pick_models.py:132 `move_uvs`
- Complexity 10 (limit 10) — scripts/pick_models.py:158 `unpack_glb`
- Arguments 5 (limit 5) — scripts/pick_models.py:193 `write_glb_with_node`
- File size 495 (limit 500) — crates/crossfir3d/src/settings.rs
- File size 490 (limit 500) — crates/crossfir3d/src/held.rs
- File size 488 (limit 500) — crates/cfproto/src/items.rs
- File size 466 (limit 500) — crates/crossfir3d/src/login.rs

## Top recommendations

1. Move the inline test modules of `keys.rs`, `ui.rs` and `back.rs` into sibling `tests.rs` files, and the preview systems out of `characters3d/mod.rs`: that clears every oversized file (Q2–Q5) without touching code.
2. Add one `write_atomic` helper and use it in all four per-character saves; this also fixes `races::save_body`, the one save that can leave half a file after a crash (Q7).
3. Make `characters_ui.rs` use `ui::GOLD` and `ui::overlay_window` instead of its own copies (Q8).
4. Return `Result` from `dump_map2`'s `main` and make the two mesh-merge helpers return `Option` to bring error-handling smells under 0.5 per 1k lines (Q9).
5. When the split parsers are trusted, delete the four `old_*` test oracles (Q11).
