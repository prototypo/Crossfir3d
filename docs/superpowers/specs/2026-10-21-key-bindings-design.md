# Key bindings — Design

Date: 2026-10-21
Status: draft, waiting for owner approval (reviewer questions answered)
Part of: the UI polish before M5 (solo play). Other polish items come after this one.

## Intent

In Crossfire, players bind keys to spells and skills; that is how the game is meant
to be played (`lib/help/cast.en`: "It is helpful to bind strings like 'cast burning
hands' to keys."). Binding is a client feature: the server has no `bind` command, so
each client does it. MacCrossfire has none today. All keys are fixed in code, and only
4 quick slots (keys 1–4) hold a spell or an item.

This adds bindings two ways: a full-screen overlay with a picture of the keyboard
(owner's design, mockup approved), and the classic typed `bind` / `unbind` commands
"as it does in existing clients" (owner).

## Goals

1. **A hot key opens a bindings overlay**, full screen like the inventory. K opens it
   (Claude's call: K is free); K or Escape closes it. The world does not pause.
2. **A keyboard picture.**
   - US (ANSI) shape. Key names follow the system: on macOS `command`, `option`,
     `control`; on other systems `Ctrl`, `Alt`, `Win` (Windows builds come in M7).
   - Bindings are stored by key **position** (Bevy `KeyCode`), as games usually do.
     A key's label starts as the US letter and changes to the character it really
     types (Bevy's logical key) once the player presses it (Claude's call: no system
     tells a program the layout reliably).
   - **Bound keys are yellow.** Each modifier has a colour: shift red, control
     purple, option/Alt blue, command/Win green. A modifier key is drawn in its colour
     when any binding uses it.
   - **A key bound with modifiers is yellow with a ring in each modifier's colour**
     (shift-command-g: `g` yellow with a red ring and a green ring). A key bound both
     alone and with modifiers shows all of them.
   - **Keys the game already uses** (see "Game keys") are drawn dim.
   - Hovering any bound key shows what it does (Claude's call, so bindings made by
     typing, which may be any command, are readable too).
3. **Two lists beside the keyboard: Spells and Skills.** Only what the character
   knows: spells from the spell book the client already keeps (`SpellBook`); skills
   from the server (see "Skills"). Each row shows its current binding(s).
4. **Selecting and binding.**
   - Click an action to select it. Its keys (and their modifiers) light up brightly.
   - Pick keys by clicking them on the picture, or by pressing the real combination
     (for example option + 6) while the overlay is open.
   - A binding is **zero or more modifiers plus one key**, pressed together. Clicking a
     second non-modifier key replaces the first (Claude's call: the mockup works this
     way and the owner approved it).
   - Buttons: **Bind** (an unbound action is selected and a key is picked), **Change**
     (a bound action is selected and a key is picked; it replaces that action's
     binding(s)), **Unbind** (a bound action is selected), **Clear selection**
     (anything is selected). A button is greyed out until it can act.
   - Unbind clears the key colours at once.
   - After any button, nothing stays selected.
5. **Conflicts ask first.**
   - Picking a game key: a warning, and the game asks before the binding replaces
     that game key (owner). While bound, the key does the binding, not its old job.
     Unbinding gives the old job back.
   - Picking a combination already bound to another action: the same kind of warning
     and question, naming the other action (Claude's call).
   - The arrow keys are game keys like the others: they can be replaced after the
     question (owner's answer covers them). Movement (`input.rs` `send_moves`, which
     reads the arrows directly) must then skip an arrow that a binding took.
   - Never bindable when pressed alone: Escape (the way out of every screen), K (opens
     this overlay) and Enter (opens the command box, where `unbind` is typed), so the
     player can always get back (Claude's call). With a modifier they are free. Also
     never bindable: combinations the
     system takes: command-Q, command-W, command-H, command-M, command-Tab and
     command-space on macOS; Alt-F4 and Alt-Tab elsewhere (Claude's call). They look
     dim and say why on hover.
6. **Pressing a bound combination in the world runs it.**
   - A spell: cast it in the direction the player faces, the same way a quick slot
     does today (`slot_commands` with `Slot::Spell`).
   - A skill: `use_skill <name>` (Claude's call; the builder may switch a skill that
     needs a direction to `ready_skill` plus a fire in the facing direction, as the
     quick slots do for wands).
   - A typed binding: its command text, sent as is. Several commands joined with `;`
     are sent one after another.
   - Except: a typed binding whose whole command is `cast <spell>` or `invoke <spell>`
     for a known spell **is** that spell's binding: it casts in the facing direction
     and shows on the spell's row. Here `cast` alone only readies the spell, so sending
     it literally would seem to do nothing. The same for `use_skill <skill>` and the
     Skills list (Claude's call). The file keeps the command text as typed.
   - Only in the world, never while the command box, inventory or settings are open.
7. **Typed commands work as in the old clients** (`lib/help/bind.en`).
   - `bind [-flags] <command>` asks "Press a key for: <command>" and binds the next
     combination pressed (Escape cancels). The same conflict questions apply; the
     answer is typed `y` or `n` in the command box (Claude's call).
   - Flags: `-g` binds for all characters of the account (default: this character
     only); `-e` puts the command into the command box for editing instead of sending
     it. `-n`, `-f` and `-r` are accepted and ignored, since this client has no run or
     fire modes (Claude's call). The special binds (`commandkey`, `firekey1`, ...) answer
     "Not supported." in the log.
   - `unbind` lists the bindings with numbers in the log; `unbind <n>` removes one;
     `unbind reset` removes this character's bindings after a typed `y`.
   - Bindings made by typing show in the overlay (yellow keys, hover text) and can be
     unbound there (how they are selected: see "Left to the builder").
8. **This character, or all characters** (Claude's calls):
   - The overlay binds for this character only. Only typed `bind -g` makes a binding
     for all characters.
   - When both share a combination, this character's binding wins. Making one asks
     the same conflict question as any other.
   - Unbinding an all-characters binding in the overlay asks first: "This removes it
     for all your characters."
   - The `unbind` list shows both kinds, numbered together, all-characters ones marked
     `(all)`.
9. **Saved per character** (owner agreed), in
   `~/Library/Application Support/MacCrossfire/keys.txt`, next to `slots.txt`, same
   style: one line per binding, character name (or `*` for `-g`), modifiers, key,
   flags, command. Written whole to a new file and renamed into place (avoids the
   `slots.txt` bug in TODO.md).
10. **Quick slots 1–4 stay as they are** (owner agreed).

## Game keys

The fixed keys today (`modes.rs` `act`, `input.rs`): arrows (move), A and E (apply),
G (get), I (inventory), L (log), M (mute), C (camera behind), T (face nearest),
F (hold: look at face), Space (fire), Tab (top view), Enter (command box), 1–4 (quick
slots), Escape (settings / close). K joins them (this overlay). D and X act only in
the inventory screen, so in the world they are free (the mockup dimmed them by
mistake). These keys pressed
alone are game keys; the same key with a modifier (shift-G) is free.

## Skills

The client does not keep a skill list yet. The server sends each known skill's level
and experience as stats 140–189 (`cfproto/src/stats.rs` already parses them) and the
names through `requestinfo skill_info`. The builder adds a small skill list from
those, the way `SpellBook` holds spells.

## Non-goals

- Key **sequences** pressed one after another (Emacs style). A binding is one
  combination.
- Mouse-button bindings.
- A screen to move the game's own keys (only "replace this game key with a binding").
- Run and fire modes from the old clients.
- Reading an old client's `~/.crossfire/keys` file.
- Other keyboard shapes (ISO, JIS) drawn as they are. The picture is the Mac laptop
  layout of the mockup: esc and F1–F12, fn, and the arrow keys.
- Anything for Windows beyond the key names, until M7.

## Anti-goals

- A binding that silently takes a movement key or Escape.
- An overlay that looks or behaves unlike the inventory screen.
- Bindings that fire while the player is typing.

## Constraints

- Bevy and egui as now; no new dependency.
- Same look as the inventory overlay (egui, full screen, parchment-free).
- macOS Apple Silicon first.

## Approved mockup

`.superpowers/brainstorm/32163-1792551057/content/keys-overlay-v2.html` (local, not
committed). Layout: keyboard on the left, the two lists on the right, the buttons and
a status line under the keyboard, a colour legend above.

## Left to the builder

- The exact egui layout and sizes, and how the keyboard scales with the window.
- How a typed binding (not a spell or skill) is selected in the overlay for Unbind
  (for example, clicking a bound key selects its binding).
- Where the code lives (a new `keys.rs` / `keys_ui.rs` is fine).
