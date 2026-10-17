# M2 "Playable" — Design

Date: 2026-10-17
Status: draft, waiting for owner approval (reviewer questions answered)
Builds on: `docs/superpowers/specs/2026-10-17-maccrossfire-design.md` (M1, merged)

## Intent

M1 proved the 3D client can walk on a stock Crossfire server. M2 makes it a game
you can really play: read what the world tells you, see your health, fight, carry
and use things, cast spells, and hear it all. The feel stays close to *Ocarina of
Time*: little on screen, pop-ups for what matters, a full-screen inventory, and
quick-use slots like Ocarina's C-buttons.

The owner's own words for "playable": messages inside the game window, health and
mana bars, inventory, spells. Sound if easy (it is, see below).

## Goals

1. **Messages in the window, Zelda style** (owner chose layout C):
   - Sign, book, card, paper, monument and NPC talk (`MSG_TYPE` 1–6) open a
     parchment box in the middle of the screen, **but only when the player caused
     them**: the message arrives within 2 seconds after the player applied
     something (E, quick slot, inventory) or sent a command from the command box
     (for example `say hello` to an NPC). Other type 1–6 messages (magic mouths you
     walk past, NPCs answering other players) go to the log. (Claude's call: the
     server sends NPC talk to everyone on the map, and boxes must not pop up
     uninvited.)
   - While the box is open, walking still works and closes the box; E or Escape
     also closes it. The command box can be open at the same time, so the player
     can answer an NPC. (Claude's call.)
   - Attack and "victim" messages (`MSG_TYPE_ATTACK` 14, `MSG_TYPE_VICTIM` 19)
     show as short pop-ups that fade after about 3 seconds.
   - Everything else goes to a small log in the bottom-left corner. The last few
     lines show; L toggles a taller log you can scroll (Claude's call).
   - The terminal log stays as it is (useful for debugging).
2. **Bars** in the top-left: health (red), mana (blue), grace (white), from the
   server's `stats` command. Food shows only when low (Claude's call: below 20%
   of 999), because a character can starve.
3. **Quick slots** (owner: "like Ocarina's C-buttons") in the top-right: 4 slots on
   keys 1–4. A slot holds a spell, an item (potion, food, scroll, wand), or a
   weapon. Pressing the key uses it:
   - spell: cast it in the direction the player faces (owner);
   - wand, rod, horn, bow: fire it in the direction the player faces;
   - anything else: apply it.
   Slots are saved per character name in a small settings file. Spells are
   saved by name; items by name only, so a "potion of healing" slot uses any
   matching potion. If nothing matches, the slot stays assigned but greyed out.
   (Claude's call: item tags change every login.)
4. **Facing.** The direction of the player's last step or step attempt. A small
   arrow at the player's feet shows it (Claude's call). Walking into a monster is
   still the normal melee attack (Crossfire rule).
5. **Inventory, full screen, on I** (owner). I opens and closes it; Escape also
   closes it. The world does not pause (shared online world). It shows:
   - items carried, with icon, name, count, and markers for applied/worn,
     cursed, locked;
   - items on the ground under the player (so the player can pick up and see
     shop items);
   - **containers** (owner: in M2): bags, sacks, and quivers. Applying a container
     opens it; its contents show indented under it. Items can be moved into an
     open container and out of it to the main inventory or the ground. The
     inventory model is a tree keyed by item tag (location = player, ground, or a
     container tag). Arrows in an open quiver are used by a readied bow.
   - known spells (name, level, cost);
   - main stats (Str, Dex, Con, Wis, Cha, Int, Pow, level, experience).
   Actions on the selected entry: apply/wear (E), drop (D), pick up (G),
   put in quick slot (1–4), examine (X). Mouse click selects; arrow keys move the
   selection (Claude's call). Movement keys do not move the player while the
   inventory is open.
6. **Death.** Stock servers are not permadeath (`NOT_PERMADETH TRUE`): the
   character loses some experience and a stat and comes back to life, with no
   question asked. The client shows the death message in the parchment box.
   On a permadeath server the server sends `accountplayers` again after death;
   the client then goes back to the login form with "Your character died".
   No character picker in M2. (Claude's call, from the server source:
   `kill_player_not_permadeath`, `play_again`.)
7. **Sound and music** (owner: "if easy, add them now").
   - Sound effects from the server's `sound2` command, mapped to files with the
     `crossfire-sounds` repo's `sounds.conf` (41 sounds).
   - Music from the server's `music` command. The repo has 4 songs (scorn,
     scorncounty, navar, wilderness); other names play nothing.
   - M key mutes and unmutes (Claude's call). Volume is not adjustable in M2.
   - Sound files are not committed to this repo (per-file licences). The local
     server script clones `crossfire-sounds` into `upstream/` like the other repos,
     pinned, and the client loads sounds from there in M2 (Claude's call;
     bundling comes with packaging in M5/M7).

## Non-goals

- No new game rules; everything is done with existing server commands.
- No party, chat channels, or `tell` window beyond what the log shows.
  (`say`/`shout` still work from the Enter command box.)
- No character creation screen (still first race/class; M1 behaviour).
- No key rebinding screen. Fixed keys, listed below.
- No volume sliders or video settings (M3).
- No 3D models (M4), no animation beyond first frame unless cheap.
- No mouse-driven movement.

## Anti-goals

- Text the player must read appears only in the terminal.
- A screen that pauses or hides the world while monsters can attack, without the
  player choosing to open it.
- Keys that type into a box also moving or acting in the game (M1 bug class).
- Breaking play on stock servers 1.75.0+.

## Keys (fixed in M2)

| Key | In the world | In the inventory |
|---|---|---|
| Arrows / WASD | walk (tap = 1 step, hold repeats) | move selection |
| E | apply what you stand on; close parchment box | apply / wear selected |
| G | get what you stand on | pick up selected ground item |
| D | — (D is a movement key) | drop selected |
| X | — | examine selected |
| 1–4 | use quick slot | put selected in quick slot |
| I | open inventory | close inventory |
| L | toggle tall log | — |
| M | mute / unmute | — |
| Enter | command box | — |
| Escape | close box / inventory | close inventory |

Note: D is west in WASD, so "drop" exists only inside the inventory.

## Protocol work (all on stock servers)

New `setup` options: `spellmon 1`, `sound2 3`, `want_pickup 1` (optional),
`extended_stats 0`. New server commands to parse:
`stats`, `item2`, `upditem`, `delitem`, `delinv`, `player`, `addspell`,
`updspell`, `delspell`, `sound2`, `music`, `query` (log it; answer in the
command box), and `drawextinfo`'s type field
(kept, not thrown away as in M1). New client commands: `cast <spell>`,
`fire <dir>` / `fire_stop` (or the server's directional form — the builder checks
which the server expects for spells and ranged items), `apply <tag>`,
`move <to> <tag> <count>` (drop/pick up), `examine <tag>`, `reply <text>`,
`lookat`. Exact formats are in `doc/Developers/protocol.txt` and
`socket/request.cpp` in the pinned server.

## Constraints (unchanged from M1)

GPLv2-or-later; Rust + Bevy + bevy_egui; Mac first; no art or sound files
committed; unknown commands logged and ignored; must work on Crossfire 1.75.0+.

## Testing

- Unit tests for every new parse/encode, the message router (type → box / pop-up /
  log), the stats → bars mapping, quick-slot actions, inventory model updates
  (`item2`/`upditem`/`delitem`/`delinv`), and the sound name → file lookup.
- Live-server tests (ignored by default): log in, read a sign and see it routed to
  the box; pick up and drop an item; cast a known spell; death → play again flow
  if it can be forced safely on the local server.
- By hand, with `MACCROSSFIRE_LOGIN` and `MACCROSSFIRE_SCREENSHOT`: screenshots of
  the parchment box, bars, quick slots, and inventory.

## Left to the builder

- Exact look of the parchment box, pop-ups, bars, and inventory (egui is fine),
  within the layouts the owner picked.
- How the facing arrow is drawn.
- The settings file format and location for quick slots.
- How long pop-ups stay, how many log lines show.
- Module and file names.
