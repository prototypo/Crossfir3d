# MacCrossfire

A 3D client for [Crossfire](https://crossfire.real-time.com/), written in Rust.
It plays on normal Crossfire servers (1.75.0 and newer). Licence: GPLv2 or later.

Design: `docs/superpowers/specs/2026-10-17-maccrossfire-design.md`

## Run a local server (macOS, needs Homebrew)

```sh
scripts/server.sh build   # once: clones server, arch, maps into upstream/ and builds
scripts/server.sh run     # listens on localhost:13327
```

## Run the client

```sh
cargo run --release -p maccrossfire
```

## Tests

```sh
cargo test --workspace
# with the local server running:
cargo test -p maccrossfire --test live_server -- --ignored
```
