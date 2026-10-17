# TODO

Known problems left for later. Each line says what goes wrong and where to look.

## Bugs

- [ ] Music keeps playing on the login screen after logout or death. `end_session` in `crates/maccrossfire/src/main.rs` does not despawn the `Music` entity (`audio.rs`).
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
- [ ] `MACCROSSFIRE_HOUR` is read from the environment every frame (`world.rs` `lighting`).

- [ ] A refused `accountplay` (e.g. "Character X is already playing" after a crash) ends the session instead of showing the reason on the character list; the screen is blank between Play and the first map (`login.rs`, `characters_ui.rs`).
- [ ] "Connecting..." is cleared as soon as a session exists, so a slow account login shows a blank screen (`characters_ui.rs`).
- [ ] A kept race/class choice is not re-checked against the newly offered options after changing race or class (`newchar.rs` `refresh_choices`); a choice with no options sends an empty value. Stock data has only one choice, so this cannot happen today.

## Missing

- [ ] Item pictures (icons) in the inventory.
- [ ] Server text markup such as `[b]` and `[fixed]` shows as plain text in the parchment box and log.
- [ ] Torches and other wall-mounted things are hidden inside the solid wall blocks (`world.rs`).
- [ ] Live tests for "read a sign → parchment box" and "cast a known spell".

## Not checked yet

- [ ] Character list with real characters (needs a saved character, i.e. one with experience), and a race/class choice drop-down (no stock race or class offered one on the test server).

- [ ] The "Settings" button on the login screen (only the in-game screen was screenshot-checked).

- [ ] Arrow keys in the inventory after a mouse click: egui's own focus movement may fight the custom selection.
- [ ] Whether the parchment box draws over the full-screen inventory.
- [ ] `spellmon 1` and `sound2 3` on a real Crossfire 1.75.0 server (only the pinned master server was tested).
- [ ] Sound and music by ear, and using a spell or wand quick slot on a monster.
