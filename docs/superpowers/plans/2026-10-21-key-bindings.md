# Key Bindings Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Players bind key combinations to spells, skills and typed commands, with a keyboard-picture overlay (K) and the classic `bind` / `unbind` commands.

**Architecture:** A pure, Bevy-free model in the library (`crates/maccrossfire/src/keys.rs`): combos, bindings, the file, the rules (game keys, reserved keys, which action a command is, typed-command parsing). The app side maps Bevy `KeyCode`s to key names, runs bindings in the world, handles typed commands in the command box, and draws the egui overlay (`keys_ui.rs`). Skills come from `cfproto` (stats 140–189 and `replyinfo skill_info`).

**Tech Stack:** Rust, Bevy 0.20, bevy_egui, the `cfproto` crate. No new dependency.

**Spec:** `docs/superpowers/specs/2026-10-21-key-bindings-design.md`

## Global Constraints

- No new dependency. Bevy and egui as now.
- The library (`crates/maccrossfire/src/*.rs` listed in `lib.rs`) stays Bevy-free; key names there are strings.
- Bindings file: `~/Library/Application Support/MacCrossfire/keys.txt`, next to `slots.txt`; written whole to a temp file and renamed into place.
- Colours: bound key yellow; shift red, control purple, option/Alt blue, command/Win green.
- Never bindable alone: Escape, K, Enter. Never bindable on macOS: command-Q, command-W, command-H, command-M, command-Tab, command-space; elsewhere Alt-F4, Alt-Tab.
- Game keys (alone, no modifier): arrows, A, E, G, I, L, M, C, T, F, Space, Tab, 1–4 (plus Enter, Escape, K, which are reserved). D and X are free.
- Bindings run only in `Mode::World`.
- Quick slots 1–4 do not change.
- Copy: "Press a key for: <command>", "This removes it for all your characters.", "Not supported."

## Review Focus

- A bound combination pressed while the command box, inventory, settings or overlay is open must do nothing (Task 4 test `bindings_only_in_world`).
- Shift + a game key (shift-G) is free and needs no warning; plain G, once bound, runs the binding and not Get (Task 2 test `game_key_only_without_mods`, Task 4 test `binding_beats_game_key`).
- A binding to a spell this character does not know (forgotten, or a `-g` binding on another character) sends its text as is and never panics (Task 2 test `unknown_spell_is_plain_command`).
- A hand-edited or damaged `keys.txt` (bad lines, tabs, other characters' lines) loads what it can and keeps other characters' lines on save (Task 1 test `file_skips_bad_lines_keeps_others`).
- A character binding and an all-characters binding on the same combo: the character's wins; unbinding it shows the all-characters one again (Task 1 test `character_beats_all`).

---

### Task 1: Binding model and file (library)

**Files:**
- Create: `crates/maccrossfire/src/keys.rs`
- Modify: `crates/maccrossfire/src/lib.rs` (add `pub mod keys;`)
- Test: `crates/maccrossfire/src/keys.rs` (`#[cfg(test)] mod tests`, as in `slots.rs`)

**Interfaces:**
- Produces:
  - `#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)] pub struct Mods { pub shift: bool, pub ctrl: bool, pub alt: bool, pub cmd: bool }` with `pub fn none(self) -> bool`.
  - `#[derive(Clone, Debug, PartialEq, Eq, Hash)] pub struct Combo { pub mods: Mods, pub key: String }`. `key` is a key name: `"A"`..`"Z"`, `"0"`..`"9"`, `"F1"`..`"F12"`, `"Up"`, `"Down"`, `"Left"`, `"Right"`, `"Space"`, `"Tab"`, `"Enter"`, `"Escape"`, `"Backspace"`, and the punctuation names `` "`" "-" "=" "[" "]" "\\" ";" "'" "," "." "/" ``.
  - `impl Combo { pub fn text(&self, mac: bool) -> String }`: `"shift+cmd+G"` on mac, `"Shift+Win+G"` elsewhere; order shift, ctrl, alt, cmd; mac names `shift ctrl option cmd`, other names `Shift Ctrl Alt Win`.
  - `#[derive(Clone, Debug, PartialEq)] pub struct Binding { pub all: bool, pub combo: Combo, pub edit: bool, pub command: String }` (`all`: for every character, from `-g`; `edit`: from `-e`).
  - `#[derive(Clone, Debug, Default, PartialEq)] pub struct Bindings { pub list: Vec<Binding> }` holding this character's and the all-characters bindings.
  - `impl Bindings`:
    - `pub fn lookup(&self, combo: &Combo) -> Option<&Binding>`: the character's binding first, else the all-characters one.
    - `pub fn bind(&mut self, b: Binding)`: replaces a binding with the same `combo` and `all`, else adds.
    - `pub fn remove(&mut self, index: usize) -> Option<Binding>` (index into `list`, the `unbind <n>` numbering minus 1).
    - `pub fn reset(&mut self)`: removes this character's bindings, keeps the all-characters ones.
    - `pub fn numbered(&self, mac: bool) -> Vec<String>`: `"1. option+6  cast burning hands"`, with `" (all)"` appended for all-characters ones.
  - `pub fn default_keys_path() -> PathBuf` (like `slots::default_slots_path`, file `keys.txt`).
  - `pub fn load_bindings(path: &Path, character: &str) -> Bindings`.
  - `pub fn save_bindings(path: &Path, character: &str, b: &Bindings) -> io::Result<()>`.
  - File line: `who<TAB>mods<TAB>key<TAB>flags<TAB>command`; `who` is the character or `*`; `mods` like `shift+cmd` or `-`; `flags` `e` or `-`. Tabs and newlines in a command are written as spaces.

- [ ] **Step 1: Write the failing tests**

```rust
fn combo(mods: Mods, key: &str) -> Combo { Combo { mods, key: key.into() } }
fn opt() -> Mods { Mods { alt: true, ..Default::default() } }

#[test]
fn combo_text_by_system() {
    let c = combo(Mods { shift: true, cmd: true, ..Default::default() }, "G");
    assert_eq!(c.text(true), "shift+cmd+G");
    assert_eq!(c.text(false), "Shift+Win+G");
}

#[test]
fn character_beats_all() {
    let mut b = Bindings::default();
    b.bind(Binding { all: true, combo: combo(opt(), "6"), edit: false, command: "say all".into() });
    b.bind(Binding { all: false, combo: combo(opt(), "6"), edit: false, command: "say me".into() });
    assert_eq!(b.lookup(&combo(opt(), "6")).unwrap().command, "say me");
    let mine = b.list.iter().position(|x| !x.all).unwrap();
    b.remove(mine);
    assert_eq!(b.lookup(&combo(opt(), "6")).unwrap().command, "say all");
}

#[test]
fn bind_replaces_same_combo() {
    let mut b = Bindings::default();
    b.bind(Binding { all: false, combo: combo(opt(), "6"), edit: false, command: "a".into() });
    b.bind(Binding { all: false, combo: combo(opt(), "6"), edit: false, command: "b".into() });
    assert_eq!(b.list.len(), 1);
    assert_eq!(b.numbered(true), ["1. option+6  b"]);
}

#[test]
fn file_roundtrip_and_reset() { /* save for "Ann" (one own, one all), load for "Ann" == same; reset() keeps only the all one */ }

#[test]
fn file_skips_bad_lines_keeps_others() {
    // File holds: a line for "Bob", a line with 3 fields, a line with mods "nonsense", and one good "Ann" line.
    // load_bindings(.., "Ann") has 1 binding; save_bindings(.., "Ann", ..) leaves Bob's line unchanged.
    // A command "a\tb\nc" saves and loads back as "a b c".
}
```

The two tests with comments: write them with the values given in the comment (use the `temp_file` helper pattern from `slots.rs` tests).

- [ ] **Step 2: Run them and see them fail**

Run: `cargo test -p maccrossfire --lib keys::`
Expected: FAIL (module `keys` not found)

- [ ] **Step 3: Implement the types and functions above in `crates/maccrossfire/src/keys.rs`**

Save: read the old file, keep lines whose `who` is neither `character` nor `*`, append this `Bindings`' lines, write to `keys.txt.tmp`, then `std::fs::rename`. Create the parent directory as `save_slots` does.

- [ ] **Step 4: Run the tests and see them pass**

Run: `cargo test -p maccrossfire --lib keys::`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/maccrossfire/src/keys.rs crates/maccrossfire/src/lib.rs
git commit -m "feat: key binding model and keys.txt"
```

---

### Task 2: Binding rules and typed commands (library)

**Files:**
- Modify: `crates/maccrossfire/src/keys.rs`

**Interfaces:**
- Consumes: Task 1 `Combo`, `Mods`.
- Produces:
  - `#[derive(Clone, Debug, PartialEq)] pub enum Action { Spell(String), Skill(String), Command(String) }`
  - `pub fn action_of(command: &str, spells: &[String], skills: &[String]) -> Action`: a whole command `cast X` or `invoke X` with X a known spell (case-insensitive, trimmed) is `Spell(X as known)`; `use_skill X` with X a known skill is `Skill(X)`; anything else `Command(command)`.
  - `pub fn spell_command(name: &str) -> String` = `"cast <name>"`; `pub fn skill_command(name: &str) -> String` = `"use_skill <name>"` (what the overlay saves).
  - `pub fn game_key(combo: &Combo) -> Option<&'static str>`: for no modifiers only, the job: Up/Down/Left/Right `"move"`, A and E `"apply"`, G `"get"`, I `"inventory"`, L `"log"`, M `"mute"`, C `"camera behind"`, T `"face nearest"`, F `"look at face"`, Space `"fire"`, Tab `"top view"`, 1–4 `"quick slot"`.
  - `pub fn reserved(combo: &Combo, mac: bool) -> Option<&'static str>`: the reason, for example Escape `"Escape closes screens"`, K `"K opens this screen"`, Enter `"Enter opens the command box"` (no modifiers); on mac cmd+Q/W/H/M/Tab/Space `"used by macOS"`; else alt+F4, alt+Tab `"used by the system"`.
  - `#[derive(Debug, PartialEq)] pub enum Typed { Bind { all: bool, edit: bool, command: String }, Special(String), List, Remove(usize), Reset, Usage }`
  - `pub fn parse_typed(line: &str) -> Option<Typed>`: `None` when the line is not `bind`/`unbind`. Flags are one leading word starting with `-` holding any of `nfregc` (`g` sets `all`, `e` sets `edit`, the rest ignored); an unknown flag letter or an empty command gives `Usage`. `bind commandkey|firekey1|firekey2|runkey1|runkey2|prevkey|nextkey` gives `Special(word)`. `unbind` → `List`, `unbind 3` → `Remove(3)`, `unbind reset` → `Reset`, other `unbind` text → `Usage`.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn typed_spell_is_the_spell() {
    let spells = vec!["burning hands".to_string()];
    assert_eq!(action_of("cast Burning Hands", &spells, &[]), Action::Spell("burning hands".into()));
    assert_eq!(action_of("invoke burning hands", &spells, &[]), Action::Spell("burning hands".into()));
    assert_eq!(action_of("use_skill praying", &[], &["praying".into()]), Action::Skill("praying".into()));
}

#[test]
fn unknown_spell_is_plain_command() {
    assert_eq!(action_of("cast fireball", &[], &[]), Action::Command("cast fireball".into()));
    assert_eq!(action_of("cast burning hands; say hi", &["burning hands".into()], &[]), Action::Command("cast burning hands; say hi".into()));
}

#[test]
fn game_key_only_without_mods() {
    assert_eq!(game_key(&Combo { mods: Mods::default(), key: "G".into() }), Some("get"));
    assert_eq!(game_key(&Combo { mods: Mods { shift: true, ..Default::default() }, key: "G".into() }), None);
    assert_eq!(game_key(&Combo { mods: Mods::default(), key: "D".into() }), None);
    assert_eq!(game_key(&Combo { mods: Mods::default(), key: "Up".into() }), Some("move"));
}

#[test]
fn reserved_keys() {
    let plain = |k: &str| Combo { mods: Mods::default(), key: k.into() };
    let cmd = |k: &str| Combo { mods: Mods { cmd: true, ..Default::default() }, key: k.into() };
    assert!(reserved(&plain("Escape"), true).is_some());
    assert!(reserved(&plain("K"), true).is_some());
    assert!(reserved(&Combo { mods: Mods { shift: true, ..Default::default() }, key: "K".into() }, true).is_none());
    assert!(reserved(&cmd("Q"), true).is_some());
    assert!(reserved(&cmd("Q"), false).is_none());
    assert!(reserved(&Combo { mods: Mods { alt: true, ..Default::default() }, key: "F4".into() }, false).is_some());
}

#[test]
fn typed_commands() {
    assert_eq!(parse_typed("say hi"), None);
    assert_eq!(parse_typed("bind -ge cast paralyze"), Some(Typed::Bind { all: true, edit: true, command: "cast paralyze".into() }));
    assert_eq!(parse_typed("bind use_skill sense curse; use_skill sense magic"), Some(Typed::Bind { all: false, edit: false, command: "use_skill sense curse; use_skill sense magic".into() }));
    assert_eq!(parse_typed("bind -f cast paralyze"), Some(Typed::Bind { all: false, edit: false, command: "cast paralyze".into() }));
    assert_eq!(parse_typed("bind firekey1"), Some(Typed::Special("firekey1".into())));
    assert_eq!(parse_typed("bind -z x"), Some(Typed::Usage));
    assert_eq!(parse_typed("bind"), Some(Typed::Usage));
    assert_eq!(parse_typed("unbind"), Some(Typed::List));
    assert_eq!(parse_typed("unbind 3"), Some(Typed::Remove(3)));
    assert_eq!(parse_typed("unbind reset"), Some(Typed::Reset));
}
```

- [ ] **Step 2: Run them and see them fail**

Run: `cargo test -p maccrossfire --lib keys::`
Expected: FAIL (`action_of` etc. not found)

- [ ] **Step 3: Implement the functions above in `keys.rs`**

- [ ] **Step 4: Run the tests and see them pass**

Run: `cargo test -p maccrossfire --lib keys::`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/maccrossfire/src/keys.rs
git commit -m "feat: key binding rules and typed bind/unbind parsing"
```

---

### Task 3: Skills from the server

**Files:**
- Modify: `crates/cfproto/src/stats.rs` (keep skill stats)
- Create or modify: a `parse_skill_info` function in `crates/cfproto/src/newchar.rs` (where the other `requestinfo` parsers live) and export it from `crates/cfproto/src/lib.rs`
- Modify: `crates/maccrossfire/src/main.rs` (a `SkillNames` resource; send `requestinfo skill_info` on entering the game; store the reply)

**Interfaces:**
- Produces:
  - `Stats.skills: std::collections::BTreeMap<u8, i64>`: stat code (140–189) to experience, filled by `Stats::apply` for codes `140..=189`.
  - `pub fn parse_skill_info(data: &[u8]) -> HashMap<u8, String>`: lines `"141:alchemy\n"`; a line may have a third `:face` field (ignored); bad lines skipped.
  - In the app: `#[derive(Resource, Default)] pub struct SkillNames(pub HashMap<u8, String>)` in `main.rs`, and `pub fn known_skills(stats: &Stats, names: &SkillNames) -> Vec<String>` (sorted names of codes in `stats.skills`).

- [ ] **Step 1: Write the failing tests** (in each file's test module)

```rust
#[test]
fn skill_stats_are_kept() {
    let mut s = Stats::default();
    s.apply(&[(141, StatValue::Int(1200))]);
    assert_eq!(s.skills.get(&141), Some(&1200));
}

#[test]
fn skill_info_lines() {
    let m = parse_skill_info(b"140:alchemy\n141:praying:77\nbad\n");
    assert_eq!(m.get(&140).map(String::as_str), Some("alchemy"));
    assert_eq!(m.get(&141).map(String::as_str), Some("praying"));
    assert_eq!(m.len(), 2);
}
```

- [ ] **Step 2: Run them and see them fail**

Run: `cargo test -p cfproto`
Expected: FAIL (no field `skills`, no `parse_skill_info`)

- [ ] **Step 3: Implement**, then in `main.rs`: `init_resource::<SkillNames>()`, send `ClientMsg::RequestInfo("skill_info".into())` in the existing `OnEnter(AppState::InGame)` systems, and on `ServerMsg::ReplyInfo { kind: "skill_info", data }` in game set `SkillNames(parse_skill_info(&data))`. Clear `SkillNames` where the session ends (beside the other per-session resources).

- [ ] **Step 4: Run all tests**

Run: `cargo test --release`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/cfproto crates/maccrossfire/src/main.rs
git commit -m "feat: keep the character's skills and their names"
```

---

### Task 4: Run bindings in the world

**Files:**
- Modify: `crates/maccrossfire/src/input.rs` (key names, mods, running bindings, arrows taken by a binding)
- Modify: `crates/maccrossfire/src/modes.rs` (`Mode::Keys`, `bindings_active`)
- Modify: `crates/maccrossfire/src/main.rs` (`KeyBindings` resource, load on entering the game)

**Interfaces:**
- Consumes: Task 1 `Bindings`, `load_bindings`, `default_keys_path`; Task 2 `action_of`, `Action`; Task 3 `known_skills`; `slots::slot_commands`.
- Produces:
  - `modes.rs`: new `Mode::Keys`; `pub fn bindings_active(mode: Mode) -> bool` (true only for `Mode::World`).
  - `input.rs`: `pub fn key_name(key: KeyCode) -> Option<&'static str>` (the names from Task 1; rename the existing private `key_name` to `act_key`), `pub fn key_code(name: &str) -> Option<KeyCode>` (the reverse), `pub fn held_mods(keys: &ButtonInput<KeyCode>) -> Mods` (left or right Shift, Control, Alt, Super).
  - `main.rs`: `#[derive(Resource, Default)] pub struct KeyBindings(pub Bindings)`, loaded in `load_character_slots` (rename to `load_character_keys`) from `default_keys_path()`.
  - `input.rs`: `pub fn binding_commands(b: &Binding, inv: &Inventory, spells: &SpellBook, skills: &[String], facing: u8, range: &str) -> Vec<String>`: `Spell(n)` → `slot_commands(&Slot::Spell(n), ..)`; `Skill(n)` → `[skill_command(n)]`; `Command(c)` → `c` split on `;`, trimmed, empty parts dropped.
- Behaviour in `world_keys`: for each just-pressed key with a `key_name`, when `bindings_active(mode)` and `KeyBindings.0.lookup(&Combo { mods: held_mods(..), key })` finds a binding: if `edit`, open the command box with the command text (`CommandLine { open: true, text }`, `Mode::CommandLine`); else send `binding_commands(..)` with the same `command` closure; then skip `act` for that key. Modifier keys themselves are never looked up. `send_moves` ignores an arrow whose plain combo has a binding.

- [ ] **Step 1: Write the failing tests**

```rust
// modes.rs
#[test]
fn bindings_only_in_world() {
    assert!(bindings_active(Mode::World));
    for m in [Mode::CommandLine, Mode::Inventory, Mode::Settings, Mode::Keys] {
        assert!(!bindings_active(m));
    }
}

// input.rs
#[test]
fn key_names_roundtrip() {
    for k in [KeyCode::KeyG, KeyCode::Digit6, KeyCode::F3, KeyCode::ArrowUp, KeyCode::Slash] {
        assert_eq!(key_code(key_name(k).unwrap()), Some(k));
    }
    assert_eq!(key_name(KeyCode::ShiftLeft), None);
}

#[test]
fn binding_beats_game_key() {
    // A plain "G" binding to "say hi": binding_commands gives ["say hi"].
    // "use_skill sense curse; use_skill sense magic" gives both commands in order.
}
```

Write `binding_beats_game_key` with the values in its comment.

- [ ] **Step 2: Run them and see them fail**

Run: `cargo test --release -p maccrossfire`
Expected: FAIL (`bindings_active`, `key_code`, `binding_commands` not found)

- [ ] **Step 3: Implement as described in Interfaces**

- [ ] **Step 4: Run all tests and clippy**

Run: `cargo test --release && cargo clippy --release 2>&1 | grep -c "^warning"`
Expected: tests PASS; no new clippy warnings (today's count is the baseline).

- [ ] **Step 5: Commit**

```bash
git add crates/maccrossfire/src
git commit -m "feat: bound keys run their spell, skill or command"
```

---

### Task 5: Typed bind and unbind

**Files:**
- Modify: `crates/maccrossfire/src/keys.rs` (`bind_question`)
- Modify: `crates/maccrossfire/src/ui.rs` (`command_line`, `PendingBind`)
- Modify: `crates/maccrossfire/src/input.rs` (`world_keys` skips a key press taken by `PendingBind`)

**Interfaces:**
- Consumes: Task 2 `parse_typed`, `Typed`, `game_key`, `reserved`; Task 1 `Bindings`, `save_bindings`; Task 4 `KeyBindings`, `key_name`, `held_mods`.
- Produces: `#[derive(Resource, Default)] pub struct PendingBind(pub Option<BindStep>)` in `ui.rs` with
  `pub enum BindStep { WaitKey { all: bool, edit: bool, command: String }, Confirm { binding: Binding, question: String }, ConfirmReset }`.
- Behaviour, on Submit in `command_line`:
  - `parse_typed(text)` is `None` → send to the server as now.
  - `Bind{..}` → log "Press a key for: <command>", set `WaitKey`. The next just-pressed non-modifier key (any mode while waiting; Escape cancels and logs "Cancelled.") makes the combo. `reserved` → log the reason, stop. `game_key` or an existing `lookup` on another command → log the question ("G is the get key. Replace it? Type y or n." / "option+6 runs <command>. Replace it? Type y or n.") and set `Confirm`; else bind and save.
  - While `Confirm` or `ConfirmReset` is set, the next submitted line `y` acts, anything else logs "Not changed." Neither goes to the server.
  - `Special(w)` → log "Not supported." `Usage` → log the syntax line from `bind.en` ("bind [-nfregc] <command>" or "unbind [<number>|reset]"). `List` → log `numbered(mac)` lines, or "No key bindings." `Remove(n)` → remove and save, or log "No binding <n>." `Reset` → log "Remove all key bindings of this character? Type y or n." and set `ConfirmReset`.
  - While `WaitKey` is set, `world_keys` must not act on that key press (check `PendingBind` there).

- [ ] **Step 1: Write the failing test** for the pure decision, as `pub fn bind_question(b: &Bindings, combo: &Combo, command: &str, mac: bool) -> Result<Option<String>, &'static str>` in `keys.rs` (Err = reserved reason; Ok(Some(q)) = ask; Ok(None) = bind at once):

```rust
#[test]
fn bind_questions() {
    let plain = |k: &str| Combo { mods: Mods::default(), key: k.into() };
    let b = Bindings::default();
    assert!(bind_question(&b, &plain("Escape"), "x", true).is_err());
    assert_eq!(bind_question(&b, &plain("G"), "x", true), Ok(Some("G is the get key. Replace it? Type y or n.".into())));
    assert_eq!(bind_question(&b, &plain("D"), "x", true), Ok(None));
    let mut b2 = Bindings::default();
    b2.bind(Binding { all: false, combo: plain("D"), edit: false, command: "say a".into() });
    assert_eq!(bind_question(&b2, &plain("D"), "x", true), Ok(Some("D runs say a. Replace it? Type y or n.".into())));
    assert_eq!(bind_question(&b2, &plain("D"), "say a", true), Ok(None));
}
```

- [ ] **Step 2: Run it and see it fail**

Run: `cargo test -p maccrossfire --lib keys::bind_questions`
Expected: FAIL

- [ ] **Step 3: Implement `bind_question`, then the `command_line` behaviour above**

- [ ] **Step 4: Live check**

Run the client against the local server (`MACCROSSFIRE_LOGIN=sprite3d:sprite3d:Showcase:new target/release/maccrossfire`), type `bind say hello`, press shift+D, then press shift+D in the world. Expected: the log says "Press a key for: say hello", then the character says hello. `unbind` lists `1. shift+D  say hello`; `unbind 1` removes it; `~/Library/Application Support/MacCrossfire/keys.txt` has and then loses the line.

- [ ] **Step 5: Commit**

```bash
git add crates/maccrossfire/src
git commit -m "feat: typed bind and unbind commands"
```

---

### Task 6: The bindings overlay (K)

**Files:**
- Create: `crates/maccrossfire/src/keys_ui.rs`
- Modify: `crates/maccrossfire/src/modes.rs` (K opens/closes: `KeyName::K`, `Act::OpenKeys`, `Act::CloseKeys`; in `Mode::Keys` only K and Escape act, like the inventory)
- Modify: `crates/maccrossfire/src/input.rs` (`act_key` maps `KeyCode::KeyK`; handle the two new acts)
- Modify: `crates/maccrossfire/src/main.rs` (`mod keys_ui;`, add `keys_ui::keys_screen` to the `EguiPrimaryContextPass` chain beside `inventory_screen`, `init_resource::<keys_ui::KeysSel>()`)

**Interfaces:**
- Consumes: everything above; `Spells`, `PlayerStats`, `SkillNames`, `KeyBindings`.
- Produces:
  - `#[derive(Resource, Default)] pub struct KeysSel { pub action: Option<Action>, pub picked: Option<Combo>, pub ask: Option<(String, Binding)>, pub labels: HashMap<&'static str, String>, pub status: String }`.
  - `pub fn keys_screen(..) -> Result` drawing only when `mode.0 == Mode::Keys`.
  - `pub fn buttons(sel: &KeysSel, bound: bool) -> [bool; 4]`: enabled state of Bind, Change, Unbind, Clear selection: Bind = action and picked and not bound; Change = action and picked and bound; Unbind = bound; Clear = action or picked.
- Layout (the approved mockup `.superpowers/brainstorm/32163-1792551057/content/keys-overlay-v2.html`): full screen; a colour legend on top; the keyboard left (Mac laptop rows: esc F1–F12 / ` 1–0 - = delete / tab q–\ / caps a–' return / shift z–/ shift / fn control option command space command option arrows; on other systems Ctrl, Win, Alt in place of control, option, command); Bind, Change, Unbind, Clear selection and a status line under it; the Spells and Skills lists right, each row with its binding text.
- Drawing rules: bound key yellow (`#e8c840`); modifier key in its colour when any binding uses it; a key bound with modifiers gets one ring per modifier colour (nested strokes); game keys and reserved keys dim, with hover text giving the job or reason; hovering a bound key shows `Combo::text` and the command; the selected action's keys and modifiers bright; picked keys dashed outline.
- Picking: click a key (modifier keys toggle; a second normal key replaces the first), or press a real combo (its held mods plus the key). Clicking a bound key with no action selected selects that binding's action (`action_of`), so typed bindings can be unbound here.
- Buttons: as `buttons`. Bind/Change save `spell_command`/`skill_command` (or the selected binding's command) for this character (`all: false`), first through `bind_question`; a question shows as a small egui modal with Replace / Cancel. Unbind of an `all` binding asks "This removes it for all your characters." first. After any button: `action` and `picked` cleared. Every change saves `keys.txt`.
- Labels: on each `KeyboardInput` with a printable `logical_key`, store `labels[key_name] = char`; draw `labels` over the US letter.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn buttons_follow_input() {
    let mut s = KeysSel::default();
    assert_eq!(buttons(&s, false), [false, false, false, false]);
    s.action = Some(Action::Spell("burning hands".into()));
    assert_eq!(buttons(&s, false), [false, false, false, true]);
    s.picked = Some(Combo { mods: Mods { alt: true, ..Default::default() }, key: "6".into() });
    assert_eq!(buttons(&s, false), [true, false, false, true]);
    assert_eq!(buttons(&s, true), [false, true, true, true]);
}

// modes.rs
#[test]
fn k_opens_and_closes_keys() {
    assert_eq!(act(Mode::World, KeyName::K, false), Act::OpenKeys);
    assert_eq!(act(Mode::Keys, KeyName::K, false), Act::CloseKeys);
    assert_eq!(act(Mode::Keys, KeyName::Escape, false), Act::CloseKeys);
    assert_eq!(act(Mode::Keys, KeyName::A, false), Act::Nothing);
}
```

- [ ] **Step 2: Run them and see them fail**

Run: `cargo test --release -p maccrossfire`
Expected: FAIL

- [ ] **Step 3: Implement as described**

- [ ] **Step 4: Run tests, then a screenshot check**

Run: `cargo test --release`, then bind two keys by typing (`bind cast <a known spell>` on option+6, `bind say hi` on shift+cmd+G), open K by a dev command or by hand, and screenshot (`MACCROSSFIRE_SCREENSHOT=$HOME/Downloads/keys.png`). Expected: 6 yellow with a blue ring, G yellow with red and green rings, option blue, shift red, command green, the spell's row shows `option+6`.

- [ ] **Step 5: Commit**

```bash
git add crates/maccrossfire/src
git commit -m "feat: key bindings overlay on K"
```

---

### Task 7: Docs

**Files:**
- Modify: `README.md` (key table: K opens key bindings; `bind` / `unbind` in the command box)
- Modify: `TODO.md` (remove nothing; add any limits found, for example ISO keyboards drawn as ANSI)

- [ ] **Step 1: Update both files**
- [ ] **Step 2: Commit**

```bash
git add README.md TODO.md
git commit -m "docs: key bindings"
```
