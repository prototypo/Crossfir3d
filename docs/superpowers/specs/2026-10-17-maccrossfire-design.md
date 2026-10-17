# MacCrossfire — Design

Date: 2026-10-17
Status: draft, waiting for owner approval

## Intent

Crossfire (https://crossfire.real-time.com/) is an old open source client-server
adventure game. It still has a small active team. Its server and main client are C.
The look is flat 32x32 tiles.

This project makes a new version of Crossfire with a modern 3D look, in the style
of *Ocarina of Time*: pseudo-realistic, with modern light and shadow. It stays
compatible with old Crossfire, so the large body of existing maps, art, and
servers keeps working.

It is a separate open source project (owner decision). It is not run by the
Crossfire team. We may talk to them later, after the first goal works.

## Goals

- A new client, written in Rust, compiled to a native app. Mac first.
- The client plays on existing Crossfire servers (the current C server).
- The game works for one player alone and for many players on a hosted server.
- 3D world with modern lighting and shadows.
- Modern video settings: resolution, window/full screen, frame rate limit, vsync,
  shadow quality, contrast, brightness, gamma. (Full list: see Milestone 3.)
- Later: a Rust server that replaces the C server, and that old clients can join.
- Later: Windows and Linux builds.

## Non-goals

- No new game rules, classes, or content. The game plays like Crossfire.
- No free (non-square) movement. Crossfire rules move in squares. The client
  only makes each step look smooth.
- No web or mobile version.
- No change to the old map or art files.

## Anti-goals (a failure even if it "works")

- It cannot join a normal Crossfire server.
- It needs new art before it can show a map. Old art must always work as fallback.
- It uses art or models with a licence that does not agree with GPLv2+.
- Two programming languages with a bridge between them in the client.

## Constraints

- **Licence:** GPLv2 or later (same as the Crossfire server and client).
- **Protocol:** Crossfire TCP protocol, port 13327, as documented in
  `doc/Developers/protocol.txt` in the server repo. Target the latest versions:
  client-to-server 1023, server-to-client 1030. Minimum supported server:
  release 1.75.0 (2021, the last tagged release) plus current master. The build
  script pins one known-good master commit. Older servers are not supported
  (Claude's call). Packets are a 2-byte big-endian
  length, then an ASCII command word, a space, then text or binary data.
- **Old data:** maps from `crossfire-maps`, art from `crossfire-arch`
  (PNG, 32x32 base faceset, some multi-tile images). Art licences are per file
  (`*.LICENSE` sidecar files), so check before copying art into this repo.
  Downloading faces from the server at run time is always fine.
- **New 3D models:** glTF format. Licence must be GPLv2+-compatible. CC0 packs
  (for example Kenney, Quaternius) are preferred.
- **Server data:** the server sends a view around the player (default 11x11,
  servers allow more), a face list per square, and a darkness value per square
  (0 = dark, 255 = full light). It does **not** send height.
- **Platform:** macOS on Apple Silicon first. Must compile, not interpret.

## Decided

| Topic | Decision | Who |
|---|---|---|
| Language | Rust for client and (later) server | Owner agreed |
| Graphics | Rust too, no second language | Owner agreed |
| Engine | Bevy (Rust game engine) | Claude's call |
| Order | Client first on the old C server; port the server later | Owner agreed |
| Compatibility | Old maps, art, and servers must work | Owner |
| 3D art | Free CC0 model packs first, own models later as needed | Owner |
| Missing models | Show the old 2D picture as an upright card ("billboard") in the 3D world | Claude's call |
| SVG | Not for game art. Maybe for menu icons later | Claude's call |
| Project | Separate from the Crossfire team | Owner |
| Solo play | The client starts a local server in the background and connects to it | Claude's call |
| Camera | Fixed angle, tilted down about 50 degrees, follows the player, zoom in/out. No free rotation in M1 | Claude's call |
| Height | Client guesses land height from face names (water low, hills/mountains high, rest flat) | Claude's call |
| Walls | Faces whose name says wall become textured blocks one square high, not cards | Claude's call |
| Indoor light | The per-square darkness value is the main light. The sun and its shadows only show where squares are fully lit, so dark dungeons are not sunlit | Claude's call |
| First test map | Scorn (town with walls, buildings, open ground) | Claude's call |

## Milestone 1: "Walk in 3D" (the first proof)

Success: the owner starts the app on a Mac, logs in to a local C server,
and walks a character around an old map, seen in 3D.

1. **Local C server.** A script that clones `crossfire-server`, `crossfire-arch`,
   and `crossfire-maps` (from `https://git.code.sf.net/p/crossfire/…`), builds the
   server on macOS (autotools or the CMake file in the tree; Homebrew deps), and
   runs it on port 13327. Python plugin on, because many maps need it.
   The upstream code stays outside this repo (a git-ignored folder).
2. **Protocol (Rust crate).** Packet read/write. Commands needed for M1:
   `version`, `setup` (mapsize, facecache, loginmethod 2, darkness, tick, beat),
   `accountlogin` / `accountnew` / `accountplayers` / `accountplay` /
   `createplayer`, `newmap`, `map2`, `face2`, `askface`, `image2`, `smooth`,
   `ncom`, `comc`, `tick`, `drawinfo`/`drawextinfo` (log to console), `failure`.
   Unknown commands are logged and ignored, never fatal.
3. **Face cache.** Store faces by checksum in the user cache folder so later
   runs do not download them again.
4. **World view (Bevy).**
   - One ground tile per map square, textured with the floor face, raised or
     lowered by the height guess.
   - Wall faces become blocks (see Decided).
   - Every other face layer is an upright card using the old picture, sorted so
     near cards draw over far cards.
   - A sun light with shadows, faded out where squares are dark. The
     per-square darkness value dims each square.
   - Fog at the edge of the view, so the edge of the server's view is hidden.
   - Smooth movement: when the map scrolls one square, slide the camera and the
     player card over about the step time (from `comc`).
5. **Input.** Arrow keys / WASD send `ncom` move commands (`north`, `east`, …,
   and diagonals with two keys). Hold a key to keep walking.
6. **Login screen.** Minimal: server address (default `localhost`), account,
   password, pick or create a character.

Not in M1: combat, spells, inventory, chat window, stats panel, sound,
video settings menu, 3D models, solo-play auto-start.

## Later milestones (roughly in order; each gets its own design)

- **M2 — Playable client.** Stats, inventory, chat, combat, spells, skills,
  pick up/drop, apply, shops, sound. Goal: a full game is possible.
- **M3 — Video settings.** Resolution, window / borderless / full screen,
  render scale, vsync, frame rate limit (30/60/120/unlimited), shadow quality
  (off/low/medium/high), anti-aliasing, contrast, brightness, gamma, field of
  view/zoom. Saved to a settings file.
- **M3.5 — Characters.** Pick a character after login; create one with race,
  class, rolled stats, and starting place. See `2026-10-17-characters-design.md`.
- **M4 — 3D models.** A mapping file from face/arch name to a glTF model.
  Use a model when the map has one, a card when it does not. Start with CC0 packs.
  Includes a 3D player character in the world and a turning 3D preview on the
  new-character screen (owner, 2026-10-17).
- **M5 — Solo play.** The app starts a bundled server and connects to it.
  (Uses the C server until M6 is done.)
- **M6 — Rust server.** Port the C server to Rust. Old clients and this client
  must both work with it. Height and other extras may be added as optional
  `setup` options, so old clients still work. Strategy (rewrite vs. step-by-step
  replacement inside the C build) is decided in its own design.
- **M7 — Windows and Linux builds.**

## Testing

- Protocol: unit tests that read and write real packets. Capture sample packets
  from the local C server and keep them as test data.
- One integration test: connect to the local server, log in, move one square,
  check that the map changed.
- Visual check by hand for the 3D view.

## Left to the builder

- Crate layout and module names.
- Bevy version and plugins (pick a current stable version).
- Exact height values and the list of face-name words for the height guess.
- Async vs. thread for the network code.
- Camera tilt, zoom limits, and step-slide timing, within the decisions above.
- How to build the C server on macOS, as long as one script does it.
