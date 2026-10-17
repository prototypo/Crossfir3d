# M3.5 "Characters" Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** After account login, show the account's characters to pick from, and a new-character screen with race, class, rolled stats, choices, and starting place.

**Architecture:** `cfproto` gains parsers for the creation data (`newchar.rs`) and a richer `accountplayers`. A pure `maccrossfire::newchar` model holds the creation form (roll, adjust, final stats, validation, the `createplayer` message). `login.rs` stops auto-creating and exposes the character list. Two egui screens draw it all on the login screen.

**Tech Stack:** Rust stable, Bevy 0.20, bevy_egui 0.43; no new crates (the dice use a small xorshift).

**Spec:** `docs/superpowers/specs/2026-10-17-characters-design.md`

## Global Constraints

- Stock Crossfire 1.75.0+, `loginmethod 2`. Formats from the pinned server: `socket/requestinfo.cpp` (`send_arch_info`, `send_map_info`, `send_new_char_info`), `socket/request.cpp` (`create_player_cmd`, `send_account_players`).
- `replyinfo <kind>` is followed by a space **or a newline** before the data.
- Stat order on the wire and in the UI: Str, Dex, Con, Wis, Cha, Int, Pow. Stat codes in race/class info: Str 5, Int 6, Wis 7, Dex 8, Con 9, Cha 10, Pow 22 (int16 values).
- Fallback limits when `newcharinfo` is missing: min 3, max 18, points 84.
- Every final stat (rolled + race + class) must be ≥ 1, or the server refuses (`check_race_and_class`).
- The character's password is the account password.
- No pictures in the character list (spec).

## Review Focus

1. **A race or class info reply arrives after the player already picked another one (slow server).** Expected: each info is stored by its arch name; the screen shows the info of what is selected now. Test in Task 2 (infos keyed by name).
2. **The server refuses `createplayer` (name taken, bad stats).** Expected: the reason shows on the new-character screen, all choices stay. Test in Task 3.
3. **An account with zero characters (just created).** Expected: the character list shows "No characters yet" and the New character button. Test in Task 3.
4. **Unknown fields in race/class info, startingmap, or newcharinfo (newer server).** Expected: skipped; the rest parses. Tests in Task 1.
5. **Roll or ± producing a total over budget or a stat outside min..max.** Expected: never. Test in Task 2 (1000 rolls).

---

### Task 1: Protocol: creation data and character records (`cfproto`)

**Files:** create `crates/cfproto/src/newchar.rs`; modify `server.rs`, `client.rs`, `lib.rs`; update callers of `AccountPlayers` and `CreatePlayer` in `maccrossfire` (login.rs tests, live tests).

**Interfaces — Produces:**
- `ServerMsg::ReplyInfo { kind, data }`: `kind` ends at the first space **or newline**.
- `#[derive(Clone, Debug, Default, PartialEq)] pub struct CharacterInfo { pub name: String, pub race: String, pub class: String, pub level: u16, pub face: Option<u16> }`; `ServerMsg::AccountPlayers(Vec<CharacterInfo>)` (ACL 1 name, 2 class, 3 race, 4 level i16, 8 face_num i16; others skipped by length).
- `#[derive(Clone, Debug, PartialEq)] pub struct Choice { pub key: String, pub description: String, pub options: Vec<(String, String)> /* (arch, name) */ }`
- `#[derive(Clone, Debug, Default, PartialEq)] pub struct ArchInfo { pub arch: String, pub name: String, pub message: String, pub stats: Vec<(u8, i16)>, pub choices: Vec<Choice> }`; `pub fn parse_arch_info(data: &[u8]) -> Result<ArchInfo, ParseError>` — data is `<arch>\n` then fields `name <lstring>`, `msg <l2string>`, `stats <code i16>... 0`, `choice <lstring key><lstring desc>(<lstring arch><lstring name>)* 0`; unknown word → stop parsing, keep what was read.
- `#[derive(Clone, Debug, PartialEq)] pub struct StartMap { pub arch: String, pub name: String, pub description: String }`; `pub fn parse_starting_maps(data: &[u8]) -> Result<Vec<StartMap>, ParseError>` — entries of `<type u8><l2string>`: 1 arch (starts a new map), 2 name, 3 description; unknown types skipped.
- `#[derive(Clone, Copy, Debug, PartialEq)] pub struct StatLimits { pub points: u16, pub min: u8, pub max: u8 }` with `impl Default` (84, 3, 18); `pub fn parse_newcharinfo(data: &[u8]) -> StatLimits` — lstring lines (with a trailing NUL) `V points N`, `V statrange A B`; anything else ignored; missing values keep defaults.
- `ClientMsg::CreatePlayer` gains `starting_map: Option<String>, choices: Vec<(String, String)>`; extra lines `starting_map <arch>` and `choice <key> <arch>` after the stats.

- [ ] **Step 1: Failing tests:** `replyinfo_kind_ends_at_newline`; `accountplayers_full_records` (name, race, class, level, face, unknown ACL skipped, zero characters); `arch_info_name_msg_stats` (real bytes shape from `send_arch_info`); `arch_info_with_choice`; `arch_info_unknown_field_stops_cleanly`; `starting_maps_four_entries`; `newcharinfo_points_and_range`; `newcharinfo_missing_uses_defaults`; `createplayer_with_map_and_choice` (exact bytes).
- [ ] **Step 2:** `cargo test --workspace` → FAIL.
- [ ] **Step 3: Implement**; fix callers (login flow uses `CharacterInfo.name`).
- [ ] **Step 4:** `cargo test --workspace` → PASS. Also run the probe idea live: `cargo test -p maccrossfire --test live_server -- --ignored` still passes.
- [ ] **Step 5: Commit** `feat(cfproto): character creation data`.

### Task 2: New-character model (`maccrossfire/src/newchar.rs`, pure)

**Interfaces — Consumes:** `ArchInfo`, `StartMap`, `StatLimits`, `ClientMsg`. **Produces:**
- `pub const STAT_NAMES: [&str; 7] = ["Str", "Dex", "Con", "Wis", "Cha", "Int", "Pow"];` and `pub fn stat_code(i: usize) -> u8` (5, 8, 9, 7, 10, 6, 22).
- `pub struct NewChar { pub races: Vec<String>, pub classes: Vec<String>, pub infos: HashMap<String, ArchInfo>, pub maps: Vec<StartMap>, pub limits: StatLimits, pub race: Option<String>, pub class: Option<String>, pub stats: [u8; 7], pub choices: HashMap<String, String>, pub map: Option<String>, pub name: String, pub error: Option<String> }` with `Default` (stats all = limits.min).
- `impl NewChar`:
  - `pub fn requests() -> Vec<ClientMsg>` — `newcharinfo`, `race_list`, `class_list`, `startingmap`.
  - `pub fn on_reply(&mut self, kind: &str, data: &[u8]) -> Vec<ClientMsg>` — stores lists/infos/maps/limits; when a list arrives, asks `race_info <r>` / `class_info <c>` for every entry; preselects the first race and class and, when maps arrive, the map whose arch is `map_scorn`, else the first; when an info with choices arrives for the selected race/class, preselects each choice's first option.
  - `pub fn roll(&mut self, seed: u64)` — every stat starts at `min`; add points one at a time to random stats below `max` until the budget is used or all are at max (xorshift64 from `seed`).
  - `pub fn adjust(&mut self, i: usize, delta: i8)` — within min..max and the budget.
  - `pub fn used(&self) -> u16`
  - `pub fn final_stats(&self) -> [i16; 7]` — stats + race + class changes.
  - `pub fn problem(&self) -> Option<String>` — empty name, no race/class, over budget, any final stat < 1 ("Str would be below 1 with this race and class").
  - `pub fn create_msg(&self, password: &str) -> Option<ClientMsg>` — None if `problem()`; choices sent as `(key, arch)` for the selected race's and class's choices only.

- [ ] **Step 1: Failing tests:** `requests_ask_four_things`; `lists_trigger_info_requests`; `scorn_preselected`; `first_map_when_no_scorn`; `infos_keyed_by_name` (info for a race not selected does not change the selected one); `roll_obeys_limits_1000_times` (each in min..max, used ≤ points, varies across seeds); `adjust_respects_limits`; `final_stats_add_race_and_class`; `stat_below_one_is_a_problem`; `create_msg_carries_everything` (race, class, 7 stats, map, choices).
- [ ] **Step 2:** `cargo test -p maccrossfire newchar` → FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4:** → PASS.
- [ ] **Step 5: Commit** `feat: new-character model`.

### Task 3: Login flow with a character list (`login.rs`)

**Interfaces — Produces:**
- `Creds { account, password, new_account, quick_character: Option<String> }` (`quick_character` only from `MACCROSSFIRE_LOGIN`).
- `LoginState { Handshake, Account, Characters, Creating, Playing, Failed(String) }`.
- `LoginFlow::characters(&self) -> &[CharacterInfo]`; `play(&mut self, name: &str) -> Vec<ClientMsg>` (→ Playing); `start_creating(&mut self) -> Vec<ClientMsg>` (→ Creating, returns `NewChar::requests()`); `create(&mut self, msg: ClientMsg) -> Vec<ClientMsg>`; `back_to_list(&mut self)`; `password(&self) -> &str`; `character(&self) -> &str` (the one being played, for quick slots).
- Rules: `AccountPlayers` in Account or Playing → store list, state Characters (unless `quick_character`: play it if present, else the old quick creation with the first race/class and stats 12, as today). `failure createplayer …` in Creating → stay in Creating with the reason (not `Failed`). Any other `failure` → `Failed`.

- [ ] **Step 1: Failing tests:** `account_login_shows_list`; `zero_characters_list_is_empty`; `play_sends_accountplay`; `creating_failure_keeps_creating` (reason kept); `quick_character_still_auto_plays`; `quick_character_still_auto_creates`; `accountplayers_while_playing_returns_to_list`; update existing login tests.
- [ ] **Step 2–4:** `cargo test -p maccrossfire login` → FAIL → implement → PASS.
- [ ] **Step 5: Commit** `feat: login flow with character list`.

### Task 4: Screens and wiring

**Files:** `ui.rs` (login form without character field), new `characters_ui.rs` (list + new-character screens), `main.rs` (route `ReplyInfo` to `NewChar` while Creating; `AccountPlayers` while InGame → back to Login state with the session kept and the list showing; quick slots load with `login.character()` once playing).

- List screen: account name, characters as rows "Name — race class, level N" (selectable; double-click/Enter plays), "No characters yet" when empty, buttons Play, New character, Back (disconnect → login form).
- New-character screen (inventory style): left column Race list, Class list; middle: selected race and class descriptions and their stat changes, choice drop-downs; right: 7 stats with − / + and the rolled value, final value, points used/budget, **Roll** button; Where to start (radio list with description of the selected one); Name field; one note line "Characters are saved once they have some experience."; empty preview area labelled for M4; **Create** (disabled with the `problem()` text shown) and **Back**. Server failure text shows above Create.
- Roll seed: current time in nanoseconds.
- [ ] **Step 1: Implement.**
- [ ] **Step 2: Verify by screenshots** (extend `dev_screenshot` to also run on the login screen when `MACCROSSFIRE_SCREENSHOT` is set and the env `MACCROSSFIRE_SCREEN=characters|newchar` drives to that screen): character list for `tester`; new-character screen after one roll.
- [ ] **Step 3: Commit** `feat: character list and new-character screens`.

### Task 5: Live test and final checks

- [ ] **Step 1:** Live test `create_chosen_character` (ignored): new account; request the creation data; choose the second race and second class, roll, Scorn; `createplayer`; expect `Map2` (in game). Then reconnect and expect `accountplayers` to list it only if the server saved it (it will not, 0 experience — assert only the creation).
- [ ] **Step 2:** `cargo test --workspace`, live tests, `cargo build --release` (0 warnings).
- [ ] **Step 3:** README (login → character list), TODO (deferred items).
- [ ] **Step 4: Owner check:** log in, create a character with your choices, start in Scorn; log in again and pick a saved character.
- [ ] **Step 5: Commit** `docs: characters notes; M3.5 complete`.
