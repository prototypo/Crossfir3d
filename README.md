# MacCrossfire

A 3D client for [Crossfire](https://crossfire.real-time.com/), written in Rust.
It plays on normal Crossfire servers (1.75.0 and newer). Licence: GPLv2 or later.

Design: `docs/superpowers/specs/2026-10-17-maccrossfire-design.md`

## Run a local server (macOS, needs Homebrew)

```sh
scripts/server.sh build   # once: clones server, arch, maps into upstream/ and builds
scripts/server.sh run     # listens on localhost:13327
```

`build` also clones the Crossfire sound pack to `upstream/crossfire-sounds`; the client
plays sounds and music from there (or from `$MACCROSSFIRE_SOUNDS`).

## Run the client

```sh
cargo run --release -p maccrossfire
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

## Settings

Escape in game (or "Settings" on the login screen) opens the video settings:
window/full screen, render scale, frame rate, ambient occlusion, shadows,
anti-aliasing, brightness, contrast, gamma. They are saved to
`~/Library/Application Support/MacCrossfire/settings.txt` (override with
`MACCROSSFIRE_SETTINGS`). The sun follows the server's game clock
(`MACCROSSFIRE_HOUR=14` forces an hour, for screenshots).

## Developer switches

- `MACCROSSFIRE_LOGIN=account:password:character[:new]` logs in at start (creates
  the character in Scorn if it does not exist; `:new` creates the account if it
  does not exist).
- `MACCROSSFIRE_COMMANDS="cmd1;cmd2"` sends game commands on entering the game.
- `MACCROSSFIRE_SCREEN=newchar[:race_arch]` opens the new-character screen.
- `MACCROSSFIRE_SCREENSHOT=file.png` (with `MACCROSSFIRE_SCREENSHOT_AT=seconds`)
  saves one screenshot and quits.

## Tests

```sh
cargo test --workspace
# with the local server running:
cargo test -p maccrossfire --test live_server -- --ignored
```
