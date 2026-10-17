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

## Settings

Escape in game (or "Settings" on the login screen) opens the video settings:
window/full screen, render scale, frame rate, ambient occlusion, shadows,
anti-aliasing, brightness, contrast, gamma. They are saved to
`~/Library/Application Support/MacCrossfire/settings.txt` (override with
`MACCROSSFIRE_SETTINGS`). The sun follows the server's game clock
(`MACCROSSFIRE_HOUR=14` forces an hour, for screenshots).

## Tests

```sh
cargo test --workspace
# with the local server running:
cargo test -p maccrossfire --test live_server -- --ignored
```
