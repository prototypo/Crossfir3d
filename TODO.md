# TODO

Known problems left for later. Each line says what goes wrong and where to look.

## Bugs



- [ ] Dragging a settings slider resets the TAA history (a brief blur while dragging).


## Missing

- [ ] Server text markup such as `[b]` and `[fixed]` shows as plain text in the parchment box and log.
- [ ] Torches and other wall-mounted things are hidden inside the solid wall blocks (`world.rs`).
- [ ] Live tests for "read a sign → parchment box" and "cast a known spell".


## Before making the repo public

- [ ] Ask GitHub Support to purge the old history: the closed PRs' refs (`refs/pull/*`, PRs #1-#53) still point at commits with the paid monster pack (`assets/models/monsters/`) and the old Hunyuan3D models of `assets/models/auto/`. Branches were rewritten and force-pushed on 2026-10-21; ask them to remove cached views and dereferenced commits of prototypo/Crossfir3d and run garbage collection.


- [ ] Shops drawn as map buildings (`mapbuilding/`, e.g. `store_gems`) stay picture cards (they show the shop name).
- [ ] Walking by typed direction words (`north` in the command line) does not turn the character; only movement keys set `Facing` (`input.rs`).
- [ ] Each tinted or darkened model instance gets its own material copies (`world.rs` `tint_model`); share copies per (material, tint) if draw calls hurt.
- [ ] Models are found in the source tree (`world.rs` `ASSET_DIR`); a packaged app needs them next to it (M7).
- [ ] Gates show only open or closed: frames between (a gate opening) show as open.
- [ ] The camera ignores trees and rocks when pulling in (`Solid` holds walls and buildings only), so it can sit inside a big tree.

- [ ] A darkness change on a square rebuilds a whole building or wall (new scenes and material copies) instead of re-tinting it (`world.rs` `sync_visuals` keep check).
- [ ] 3D characters ignore square darkness; only race colours tint them (`characters3d.rs`).
- [ ] Some committed model files are not used (Ranger outfits, Hair_Beard, Eyebrows, Floor_Brick, larger roofs, fences, some trees, several castle pieces): trim `PICK` in `scripts/pick_models.py` and re-run.
- [ ] `scripts/get-models.sh` still holds unused wizard helpers (`ask_secret`, `write_env`, `set_secret`, ...).
- [ ] `assets/models.txt` lists `guild_*` and `guild2*` twice (harmless).
- [ ] Clippy: `walls.rs:11` operator precedence, `anim.rs` collapsible if, `input.rs` complex `Local` tuple in `send_moves` (a small named struct would also make the key-down lock testable).
- [ ] Animation changes cut instead of blending (`player.stop_all()` then `play`).

## Not checked yet

- [ ] Key bindings by hand: typing `bind` in the command box, the Replace question on the K screen, and a bound spell cast in play (only the K screen picture was screenshot-checked).
- [ ] European (ISO) and Japanese keyboards are drawn as a US Mac keyboard; only key labels change once a key is pressed (`keys_ui.rs`).

- [ ] Wooden gates (`gate.glb`) and the garden gate in 3D: only portcullis gates were screenshot-checked.
- [ ] The attack, cast, and death animations in play (only idle and walk were screenshot-checked).
- [ ] Other players as 3D characters (only one player was online in tests).

- [ ] Character list with real characters (needs a saved character, i.e. one with experience), and a race/class choice drop-down (no stock race or class offered one on the test server).

- [ ] The "Settings" button on the login screen (only the in-game screen was screenshot-checked).

- [ ] Arrow keys in the inventory after a mouse click: egui's own focus movement may fight the custom selection.
- [ ] Whether the parchment box draws over the full-screen inventory.
- [ ] `spellmon 1` and `sound2 3` on a real Crossfire 1.75.0 server (only the pinned master server was tested).
- [ ] Sound and music by ear, and using a spell or wand quick slot on a monster.
