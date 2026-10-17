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
saves a character once it has some experience.

## Playing in 3D

The camera sits low behind your character, like *Ocarina of Time*. Arrow keys (or
WASD) move relative to the camera: Up walks the way the camera looks. Walking forward
slowly swings the camera behind you; **C** puts it straight behind you at once. The
mouse wheel zooms. When a wall or house is in the way, the camera moves closer, or
looks down from above.

Houses, walls, gates, trees, rocks, wells, fountains, and signs are 3D. Players are
one 3D body (with a "Broad" or "Slim" build, chosen on the new-character screen),
sized and coloured for their race. Everything else is still a picture card.

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
  the character in Scorn if it does not exist).
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
