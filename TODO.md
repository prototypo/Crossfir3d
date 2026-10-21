# TODO

Known problems left for later. Each line says what goes wrong and where to look.

## Bugs

- [ ] Music keeps playing on the login screen after logout or death. `end_session` in `crates/crossfir3d/src/main.rs` does not despawn the `Music` entity (`audio.rs`).
- [ ] A corrupt (not missing) sound file can crash the app: Bevy 0.20's `AudioSource::decoder()` unwraps. Check the file decodes before spawning it (`audio.rs`).
- [ ] Connect blocks the window for up to ~75 s on an unreachable host: `TcpStream::connect` runs on the main thread (`net.rs`). Use `connect_timeout` or connect on the net thread.
- [ ] Face textures from one session stay in `Textures.images` for the next (`world.rs`). After changing server, reused face numbers show old pictures until new ones arrive.
- [ ] The bag chosen for G (`InvSel.container`, `inventory_ui.rs`) can go stale if it is closed or deleted while another column is selected. The server answers with an error; nothing breaks.
- [ ] The server's "next/previous group" paging entries (tag `0x80000000 | n`) show as items in the inventory. E pages correctly; G, D and X on them send useless moves.
- [ ] The quick-slot file is not written atomically, so a crash while saving can lose every character's slots; an item name containing a newline splits a line (`slots.rs` `save_slots`).
- [ ] "Your character died" also shows when `accountplayers` arrives for other reasons, for example after deleting a character (`messages.rs` `in_game_reset`).
- [ ] An unknown stat code is logged once per `stats` packet (`cfproto/src/server.rs`). A newer server could fill the log; log it once.
- [ ] Unknown server commands are ignored silently (`ServerMsg::Unknown` in `main.rs`). The design says "logged and ignored"; add a `debug!`.


- [ ] The last settings change can be lost if the game quits within 1 s of the previous save (`graphics.rs` `save_settings_later`); save on exit too.
- [ ] `NaN` (or 0-size window) in a hand-edited settings file is not rejected (`settings.rs` `from_text` / `parse_window`).
- [ ] The scene image keeps a full-size zeroed copy in RAM (~59 MB at 5K) and reallocates while the window is drag-resized (`graphics.rs`); set `image.data = None`.
- [ ] macOS green-button full screen is saved as a screen-sized window place (`graphics.rs` `remember_window`).
- [ ] Turning TAA off leaves `MipBias`/`TemporalJitter` on the camera (sharper, shimmering textures); slider drags reset TAA history.
- [ ] The settings screen has no scroll area; in a short window the bottom buttons can be cut off (Escape still works).
- [ ] A player's own `time` command within 10 s of login is hidden (the clock request stays open 10 s).
- [ ] `CROSSFIR3D_HOUR` is read from the environment every frame (`world.rs` `lighting`).

- [ ] A refused `accountplay` (e.g. "Character X is already playing" after a crash) ends the session instead of showing the reason on the character list; the screen is blank between Play and the first map (`login.rs`, `characters_ui.rs`).
- [ ] "Connecting..." is cleared as soon as a session exists, so a slow account login shows a blank screen (`characters_ui.rs`).
- [ ] A kept race/class choice is not re-checked against the newly offered options after changing race or class (`newchar.rs` `refresh_choices`); a choice with no options sends an empty value. Stock data has only one choice, so this cannot happen today.

## Missing

- [ ] Item pictures (icons) in the inventory.
- [ ] Server text markup such as `[b]` and `[fixed]` shows as plain text in the parchment box and log.
- [ ] Torches and other wall-mounted things are hidden inside the solid wall blocks (`world.rs`).
- [ ] Live tests for "read a sign → parchment box" and "cast a known spell".

- [ ] Monsters the Bestiary kit does not cover (orcs, animals, dragons, undead other than skeletons, ...) are still picture cards or tinted 3D people (`person` in `assets/models.txt`).

## Before making the repo public


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
