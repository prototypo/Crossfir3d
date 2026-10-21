# Crossfir3d

A 3D client for [Crossfire](https://crossfire.real-time.com/), written in Rust.
It plays on normal Crossfire servers (1.75.0 and newer). Licence: GPLv2 or later.

Design: `docs/superpowers/specs/2026-10-17-crossfir3d-design.md`

## Run a local server (macOS, needs Homebrew)

```sh
scripts/server.sh build   # once: clones server, arch, maps into upstream/ and builds
scripts/server.sh run     # listens on localhost:13327
```

`build` also clones the Crossfire sound pack to `upstream/crossfire-sounds`; the client
plays sounds and music from there (or from `$CROSSFIR3D_SOUNDS`).

## Run the client

```sh
cargo run --release -p crossfir3d
```

## Characters

Log in with an account, then pick a character from the list or press "New character"
to choose race, class, rolled stats, and where to start (Scorn by default). The server
(as patched by `scripts/server.sh`) saves new characters too.

## Playing in 3D

The camera sits over your character's shoulder and turns to stay behind them. Arrow
keys work relative to the way you face, in every view: Up takes a step forward; Left
and Right turn you 90° and Down turns you round, without a step. Hold an arrow to turn,
then walk that way. **Tab**
turns a top view (north up) on and off; **C** goes back to the view behind you. Hold
**F** to look at your character's face. **A** or **E** applies. The mouse wheel, a
trackpad pinch, and two-finger scroll zoom (the top view has its own zoom). When a
wall or house is in the way, the camera moves closer.

Walk into a monster to attack it (hold the arrow to keep attacking); **T** turns you to
face the nearest one. **Space** fires your readied bow or wand the way you face (ready
it from the inventory: **I**, select it, **E**); arrows and bolts fly as 3D arrows. When you are hit, the screen flashes red and the HP you lost
floats above you; "hit", "miss", and "killed!" float over the monster you attack, and
it flashes red when hit. (The server sends no monster health or damage numbers.)

The log in the bottom-left corner can be made easier to read in the Settings screen
(Escape): "Log text size" (the box grows with the text) and "Log background" (how
dark, in per cent).

When you step onto things, the log names them ("You are standing on: a sack, a long
sword, ..."). **B** shows the full list in the parchment box; the Settings screen's
"Show what is under me" opens it on every new square.

### Key bindings

**K** opens the key bindings screen: a keyboard with your bound keys in yellow, and
your spells and skills beside it. Pick a spell or skill, then pick keys (click them, or
press the real combination, such as option + 6), and press **Bind**. A key bound with
modifiers has a ring in each modifier's colour. Picking a key the game already uses
asks first. A bound spell casts the way you face; a bound skill is used.

The old clients' commands also work in the command box (**Enter**): `bind cast burning
hands` (then press the key), `bind -g ...` for all your characters, `bind -e ...` to put
the command in the box instead of sending it, `unbind` to list them, `unbind 2` to
remove one, `unbind reset` to remove all. Bindings are saved in
`~/Library/Application Support/Crossfir3d/keys.txt`. Escape, K and Enter alone can
never be bound.

Shops are one-storey houses with a painted sign and icon (JEWELS, BANK, MAGIC, ...).
Standing on a house's squares (a shop's entrance row, say) shows you on its
doorstep; apply goes in. Leaving a shop, or stepping south off its squares, puts you
on the square in front of the door. With your back to a wall the camera comes in
close, first-person. Houses, walls, gates, trees, rocks, wells,
fountains, and signs are 3D. Players are one 3D body (with a "Broad" or "Slim"
build, chosen on the new-character screen), sized and coloured for their race. Your
character holds the weapon, bow, and shield in use. Everything else is still a
picture card.

The models are CC0 (Quaternius, Kenney), free tiers, in `assets/models/` with their
licences. `assets/models.txt` says which picture becomes which model.
`assets/models/MISSING.md` lists what the free tiers lack. To change the picked
models: `scripts/get-models.sh` (download, needs a browser for itch.io), then
`python3 -I scripts/pick_models.py`.

The skeleton, imp, werewolf and other monsters in `assets/models/monsters/` come from the
paid Quaternius Dungeon Monsters Kit (Source tier), whose licence forbids sharing the
files themselves, so they are not in git. With the kit unpacked in
`upstream/packs/dungeon-monsters-kit-source/`, `scripts/pick_models.py` makes the folder;
without it those monsters show as picture cards.

## Settings

Escape in game (or "Settings" on the login screen) opens the settings:
window/full screen, render scale, frame rate, ambient occlusion, shadows,
anti-aliasing, brightness, contrast, gamma, and music on/off (M still mutes
all sound). They are saved to
`~/Library/Application Support/Crossfir3d/settings.txt` (override with
`CROSSFIR3D_SETTINGS`). The sun follows the server's game clock
(`CROSSFIR3D_HOUR=14` forces an hour, for screenshots): a blue sky with drifting
clouds by day, a warm glow round the low sun at dawn and dusk, and stars at night.
Wells and other one-square things stay in view when you stand on them.

## Developer switches

- `CROSSFIR3D_LOGIN=account:password:character[:new]` logs in at start (creates
  the character in Scorn if it does not exist; `:new` creates the account if it
  does not exist).
- `CROSSFIR3D_COMMANDS="cmd1;cmd2"` sends game commands on entering the game.
- `CROSSFIR3D_SCREEN=newchar[:race_arch]` opens the new-character screen.
- `CROSSFIR3D_SCREENSHOT=file.png` (with `CROSSFIR3D_SCREENSHOT_AT=seconds`)
  saves one screenshot and quits.
- `CROSSFIR3D_COMPARE=file` (lines `picture_name model1.glb model2.glb ...`, files
  under `assets/models`) keeps those pictures as cards with the models in a row
  beside them (`experiments/sprite3d/`).

## Tests

```sh
cargo test --workspace
# with the local server running:
cargo test -p crossfir3d --test live_server -- --ignored
```
