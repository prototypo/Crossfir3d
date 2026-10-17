# M3.5 "Characters" — Design

Date: 2026-10-17
Status: draft, waiting for owner approval (reviewer questions answered)
Builds on: M1–M3 (merged).

## Intent

Today the login form asks for one character name: if the account has it, the
client plays it; if not, the client silently creates it with the first race, the
first class, and every stat at 12. There is no way to see your characters or to
choose race, class, stats, or where to start. The owner asked for real character
selection and creation. How this looks is a client decision; the server only
checks the rules (each client does it differently).

Found while checking: the server does not save a character until it has some
experience (`save_player`: "no experience, no save"), so a brand-new character
that logs out at once is gone. That is server behaviour and stays as it is; the
new-character screen says so in one line (Claude's call).

## Goals

1. **Login form**: account name, password, "New account" checkbox. No character
   name field any more.
2. **Character list** (new screen after the account login):
   - Every character on the account from the server's `accountplayers` reply:
     name, race, class, level — text only. The face number is read but no
     picture is drawn: pictures are only downloaded once the game is running, so
     a list picture is real work for a flat image M4 replaces anyway (Claude's
     call; corrects an earlier "comes for free").
   - Pick one → play (`accountplay`). Double-click or Enter also plays.
   - "New character" button → the new-character screen.
   - "Back" → logs out to the login form.
3. **New-character screen** (owner: race, class, and stats with their
   descriptions; roll the dice; where to start):
   - **Race**: the server's race list (`requestinfo race_list`), each with its
     description and stat changes (`requestinfo race_info <race>`).
   - **Class**: the same for classes (`class_list`, `class_info <class>`).
   - **Choices**: if a race or class offers choices (for example a starting
     skill or god — sent as `choice` entries in race/class info), show each as a
     drop-down with its first option preselected; they are sent with the
     creation (the server must accept the choices it offers).
   - **Stats**: the 7 stats (Str, Dex, Con, Wis, Cha, Int, Pow).
     - **Roll** (owner: "roll the dice"): random stats that obey the server's
       limits — each between the minimum and maximum, total at most the point
       budget. Roll as often as you like.
     - **− / +** buttons on each stat to adjust by hand within the same limits
       (Claude's call, owner agreed in conversation).
     - Shows points used / point budget.
     - The server also refuses a character if any final stat (after race and
       class changes) is below 1 (`check_race_and_class`); the client blocks
       Create and says why instead of sending it.
     - The limits come from the server (`requestinfo newcharinfo`, if the
       server sends it); otherwise min 3, max 18, budget 84 — this server's
       settings (Claude's call).
   - **Final stats**: each stat after race and class changes, next to the rolled
     value, so the player sees the end result.
   - **Where to start** (owner): the server's starting places
     (`requestinfo startingmap`) with their descriptions; Scorn is preselected
     (owner). If a server has no Scorn entry, the first entry is preselected.
   - **Name** of the new character. The character's password is the account
     password (Claude's call: one less field; the old per-character password is
     legacy).
   - **Create** → `createplayer` with name, password, race, class, stats,
     starting map, and choices; on success the game starts. On `failure` the
     server's reason shows on the screen and nothing is lost.
   - An empty space on the screen is kept for the 3D preview that comes in M4
     (owner).
   - "Back" → the character list.
4. **Death on a permadeath server** returns to the character list (the server
   sends `accountplayers` again), not the login form as in M2.
5. **Developer login** `MACCROSSFIRE_LOGIN=account:password:character[:new]`
   keeps working: it picks that character if it exists, otherwise creates it the
   old quick way (first race and class, stats 12). It is for tests only.

## Non-goals

- No 3D or picture preview of races (M4).
- No deleting characters, renaming, or changing passwords.
- No adding an existing (pre-account) character to the account.
- No change to how the server saves characters.

## Anti-goals

- Sending stats the server will refuse because the client broke its own limits.
- Losing what the player chose (race, class, rolled stats, name) when the server
  says no (for example "name already in use").
- Keys typed into the name field also acting in the game (M2 key rules apply; the
  game is not running on these screens).

## Constraints

Unchanged: GPLv2+, Rust + Bevy 0.20 + bevy_egui, stock Crossfire 1.75.0+,
`loginmethod 2`. Exact `replyinfo` formats for `race_info`, `class_info`,
`startingmap`, and `newcharinfo` come from the pinned server
(`socket/requestinfo.cpp`) and must be parsed safely (unknown fields skipped).

## Testing

- Unit tests: parsing of `accountplayers` with race/class/level/face, `race_info`
  / `class_info` (description, stat changes, choices), `startingmap`,
  `newcharinfo`; the dice roll always obeys the limits (many rolls); − / + stay
  within limits; final stats = rolled + race + class; Scorn preselection; the
  `createplayer` message carries starting map and choices; login flow states
  (list → play, list → create → play, failure keeps the form).
- Live test (ignored by default): create a character with a chosen race, class,
  rolled stats, and Scorn; check the game starts.
- Screenshots of the character list and the new-character screen.

## Left to the builder

- Screen layout and widgets (in the inventory's dark-and-gold style).
- How the random roll is drawn (any method that obeys the limits and does not
  always give the same shape).
- Small text such as "Characters are saved once they have some experience".
