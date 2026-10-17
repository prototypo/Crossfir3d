# M4 "3D models and the Ocarina camera" — Design

Date: 2026-10-17
Status: draft, waiting for owner approval (reviewer questions answered)
Builds on: M1–M3.5 (merged).

## Intent

The owner's picture of M4: "The player should enter Scorn at the entrance gate.
The player character should be visible with the main street extending away from
them." That needs two things: a camera like *Ocarina of Time* (low, behind the
character, turning with them) and real 3D models — first the player character,
then buildings and trees, plus Crossfire's many stone gates, wells, and signs, and
the walls. The owner chose the Quaternius model family after seeing previews: it
looks closer to Ocarina than KayKit.

## Goals

### 1. Camera (owner: "like Ocarina")

- Low and behind the player character, looking the way they walk, so streets run
  away into the distance.
- **Movement is camera-relative** (owner chose option 2): arrows/WASD give a
  direction on screen; the client sends the nearest of Crossfire's 8 directions
  for the camera's heading **at the moment the key went down**; holding the key
  keeps that compass direction even while the camera swings (Claude's call:
  otherwise holding Left walks in circles).
- The camera **slowly swings behind** the player only while they walk forward or
  diagonally forward (away from the camera), about one 45° step per second; it
  never swings while standing still or walking sideways/back, like Ocarina
  (Claude's call).
- **C** snaps the camera straight behind the player (free in all key tables).
- Mouse wheel zooms (distance) as now (Claude's call).
- The camera never sits inside a wall: it moves closer when a wall or building is
  between it and the player (Claude's call).
- Fog hides the edge of the server's view (about 12 squares each way).
- Picture cards now always face the camera (they must, since the camera turns).
- The facing arrow from M2 stays.

### 2. Player character (owner: one body for all races first)

- One Quaternius **Universal Base Character** body, animated with the **Universal
  Animation Library** (same skeleton; CC0, glTF), dressed with a Quaternius outfit
  pack made for the same skeleton (simple tunic/trousers/boots), because the base
  bodies have no clothes (Claude's call).
- Per race: size (for example gnome small, troll big) and colours (skin, hair,
  clothes tint) from a small table in the client (Claude's call: the server sends
  no colours).
- Body type: the server does not know a character's sex. The new-character screen
  gets a "Body" choice (two body types), saved by the client per character name
  next to the quick slots; default is the regular build (Claude's call). Characters
  created before M4 use the default. Unusual races (fenx, dragon, wraith, serpentfolk, …) get special
  models later; for now they use the body with fitting colours.
- Animations: idle, walk, melee attack, spell cast, death. The server does not say
  what the player is doing, so the client works it out (Claude's call):
  - walk while a move is in progress (the step glide),
  - attack when an attack message (`MSG_TYPE_ATTACK`) arrives about the player,
  - cast when the player uses a spell slot or `cast`,
  - death on the death message,
  - idle otherwise.
- The body faces the way the player last moved.
- **Other players** use the same body, sized and coloured for their race when it can
  be told from their picture's name; otherwise a default human (Claude's call).
- **New-character screen**: the empty area from M3.5 shows the body, slowly
  turning, sized and coloured for the selected race (owner, M3.5).

### 3. World models

- **Buildings** (about 70 pictures in `construct/`: houses, shops, temples,
  castles, towers): the Quaternius **Medieval Village MegaKit** has modular pieces
  (walls, roofs, doors, windows), not finished houses. The client **assembles each
  building at run time** from a recipe in the mapping file: footprint (from the
  picture size), wall style, roof style, door side, and a few extras (chimney,
  tower top). A handful of recipes (small house, large house, shop, tavern,
  temple, tower, castle) cover the 70 pictures (Claude's call: no Blender work
  needed). If a CC0 Quaternius or Kenney pack with finished buildings that match
  is found while building, it may replace recipes for some pictures.
- **Trees** (about 50) and **mountains/hills** (about 21): Quaternius **Stylized
  Nature MegaKit**. Mountains/hills keep the M1 height guess for the ground and
  get rock models on top (Claude's call).
- **Stone gates and arches**: decorative arches (`exit/arch*`) and the openable
  gates and grates (`connect/Gates`, `Garden_gate`). Openable gates have separate
  pictures for open and closed, and the mapping is per picture, so an open gate
  shows an open model and a closed gate a closed one (Claude's call). Models from
  Kenney **Castle Kit** (CC0).
- **Wells and fountains** (7) and **signs** (7): from the packs where they exist;
  a simple wooden signpost and a well are made for the project if no pack has them
  (CC0 or GPL, our own).
- **Walls** (owner: in M4): 39 wall styles, each with up to 16 "joined" versions
  (`wall_0` … `wall_F` and similar, which the server already picks per square):
  modular wall pieces from the village kit, chosen and rotated from the version
  name so walls join. Wall styles map to a few materials (stone, timber, wood,
  hedge, …).
- **Many pictures share one model.** A mapping file in the repo lists, per picture
  name or name prefix: model file, scale, rotation, offset, and colour tint.
- **Anything without a model stays a picture card**, as now — including monsters,
  items, and floors.
- The darkness/lighting rules from M1–M3 apply to models as they do to cards.

### 4. Files

- Only the model files the game uses are committed, under `assets/models/`, with
  each pack's CC0 licence text next to them (Claude's call). Full packs are not
  committed.
- A script downloads the packs into `upstream/` (git-ignored) for picking more
  models later.
- Every model in the repo must be CC0 or our own work (GPLv2+-compatible).
- **Free tiers only** (owner): use the free "Standard" parts of the Quaternius
  packs. If an animation or piece is missing, reuse the nearest one and record
  it; then tell the owner exactly which paid pack would add it, so the owner can
  decide whether to buy it.

## Non-goals

- Special models per race (later), monster models (later), item models (later).
- Floor/terrain meshes beyond the M1 height guess.
- Changing the server or the protocol.
- Free (non-square) movement: steps stay Crossfire's 8 directions.

## Anti-goals

- A building or wall model that blocks the view of the player (the camera rule
  above must prevent it).
- A model at the wrong place or size relative to the server's squares (the player
  would walk "through" a model that looks solid, or bump into nothing).
- Losing the old picture fallback: a missing or broken model file must show the
  picture card, never nothing.
- Animations that make the player look like they are doing something the server
  does not agree with for long (for example walking in place).

## Constraints

Unchanged: GPLv2+, Rust + Bevy 0.20 (glTF loading; no Draco/meshopt), Mac first,
stock Crossfire 1.75.0+. Assets must be CC0 or own work. Models must not use glTF
extensions Bevy cannot read; re-export if needed.

## Testing

- Unit tests: camera-relative direction mapping (every screen direction × 8
  camera headings); camera swing rule; mapping-file parsing and lookup (exact
  name beats prefix; unknown → card); wall version → piece and rotation; race →
  size/colour table; animation state rules (walk/attack/cast/death/idle).
- Screenshots: Scorn from the gate looking up the main street; a house, a tree, a
  gate, a well, a sign, a wall corner; the player in each animation; the
  new-character preview for a small and a big race.
- By hand (owner): walk Scorn; the camera feels right; nothing blocks the view.

## Left to the builder

- Exact camera height, distance, pitch, and swing speed.
- Which Quaternius/Kenney pieces represent which Crossfire pictures (within the
  mapping-file approach), and the per-race size/colour values.
- Animation blending and timing.
- How the camera avoids walls (ray test or simple pull-in).
