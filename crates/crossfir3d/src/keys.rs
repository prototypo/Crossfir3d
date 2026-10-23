//! Key bindings: key combinations that run a spell, a skill or typed commands.
//! Pure (key names are strings), so it tests without Bevy.

use std::io;
use std::path::{Path, PathBuf};

/// Modifier keys held with a key. `alt` is option on a Mac, `cmd` is command / Win.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Mods {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub cmd: bool,
}

impl Mods {
    pub fn none(self) -> bool {
        self == Mods::default()
    }

    /// The held modifiers' names in a fixed order: shift, control, option/Alt, command/Win.
    fn names(self, mac: bool) -> Vec<&'static str> {
        let names = if mac { ["shift", "ctrl", "option", "cmd"] } else { ["Shift", "Ctrl", "Alt", "Win"] };
        [self.shift, self.ctrl, self.alt, self.cmd].iter().zip(names).filter(|(on, _)| **on).map(|(_, n)| n).collect()
    }

    /// Reads the file form (`shift+cmd`, `-` for none).
    fn parse(text: &str) -> Option<Mods> {
        let mut m = Mods::default();
        if text == "-" {
            return Some(m);
        }
        for part in text.split('+') {
            match part {
                "shift" => m.shift = true,
                "ctrl" => m.ctrl = true,
                "option" => m.alt = true,
                "cmd" => m.cmd = true,
                _ => return None,
            }
        }
        Some(m)
    }
}

/// Modifiers plus one key, pressed together. `key` is a key name such as "G", "6",
/// "F3", "Up" or "/" (the app maps them to and from Bevy key codes).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Combo {
    pub mods: Mods,
    pub key: String,
}

impl Combo {
    /// "shift+cmd+g" on a Mac, "Shift+Win+g" elsewhere: letters in lower case, as on the keys.
    pub fn text(&self, mac: bool) -> String {
        let key = if self.key.len() == 1 { self.key.to_ascii_lowercase() } else { self.key.clone() };
        let mut parts = self.mods.names(mac);
        parts.push(&key);
        parts.join("+")
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Binding {
    /// For every character of the account (`bind -g`).
    pub all: bool,
    pub combo: Combo,
    /// Put the command into the command box instead of sending it (`bind -e`).
    pub edit: bool,
    pub command: String,
}

/// This character's bindings and the all-characters ones.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Bindings {
    pub list: Vec<Binding>,
}

impl Bindings {
    /// The character's binding for `combo`, else the all-characters one.
    pub fn lookup(&self, combo: &Combo) -> Option<&Binding> {
        let on = |all: bool| self.list.iter().find(|b| b.all == all && b.combo == *combo);
        on(false).or_else(|| on(true))
    }

    /// Replaces the binding with the same combo and scope, else adds it.
    pub fn bind(&mut self, b: Binding) {
        match self.list.iter_mut().find(|x| x.all == b.all && x.combo == b.combo) {
            Some(old) => *old = b,
            None => self.list.push(b),
        }
    }

    pub fn remove(&mut self, index: usize) -> Option<Binding> {
        (index < self.list.len()).then(|| self.list.remove(index))
    }

    /// Removes this character's bindings; the all-characters ones stay.
    pub fn reset(&mut self) {
        self.list.retain(|b| b.all);
    }

    /// The `unbind` list: "1. option+6  cast burning hands", "(all)" on shared ones.
    pub fn numbered(&self, mac: bool) -> Vec<String> {
        let line = |(i, b): (usize, &Binding)| {
            format!("{}. {}  {}{}", i + 1, b.combo.text(mac), b.command, if b.all { " (all)" } else { "" })
        };
        self.list.iter().enumerate().map(line).collect()
    }
}

/// What a binding's command does.
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    /// Cast this known spell in the facing direction.
    Spell(String),
    /// Use this known skill.
    Skill(String),
    /// Send the text as is (`;` separates commands).
    Command(String),
}

/// `cast X` / `invoke X` of a known spell is that spell, `use_skill X` of a known skill
/// that skill (here `cast` alone only readies a spell). Anything else is a command.
pub fn action_of(command: &str, spells: &[String], skills: &[String]) -> Action {
    let known = |rest: &str, names: &[String]| names.iter().find(|n| n.eq_ignore_ascii_case(rest.trim())).cloned();
    let (word, rest) = command.trim().split_once(' ').unwrap_or((command.trim(), ""));
    let found = match word {
        "cast" | "invoke" => known(rest, spells).map(Action::Spell),
        "use_skill" => known(rest, skills).map(Action::Skill),
        _ => None,
    };
    found.unwrap_or_else(|| Action::Command(command.into()))
}

pub fn spell_command(name: &str) -> String {
    format!("cast {name}")
}

pub fn skill_command(name: &str) -> String {
    format!("use_skill {name}")
}

/// Each key the game uses by itself, with its job.
const GAME_KEYS: &[(&str, &str)] = &[
    ("Up", "move"),
    ("Down", "move"),
    ("Left", "move"),
    ("Right", "move"),
    ("A", "apply"),
    ("E", "apply"),
    ("G", "get"),
    ("I", "inventory"),
    ("B", "what is below"),
    ("L", "log"),
    ("M", "mute"),
    ("C", "camera behind"),
    ("T", "face nearest"),
    ("F", "look at face"),
    ("Space", "fire"),
    ("Tab", "top view"),
    ("1", "quick slot"),
    ("2", "quick slot"),
    ("3", "quick slot"),
    ("4", "quick slot"),
];

/// The game's own job for a key pressed alone (a binding may replace it, after asking).
pub fn game_key(combo: &Combo) -> Option<&'static str> {
    if !combo.mods.none() {
        return None;
    }
    GAME_KEYS.iter().find(|(k, _)| *k == combo.key).map(|(_, job)| *job)
}

/// Plain keys the game itself uses, with why they can't be bound.
const GAME_RESERVED: [(&str, &str); 3] =
    [("Escape", "Escape closes screens"), ("K", "K opens this screen"), ("Enter", "Enter opens the command box")];

/// Why an unmodified key is the game's own, if it is.
fn game_reserved(key: &str) -> Option<&'static str> {
    GAME_RESERVED.iter().find(|(k, _)| *k == key).map(|(_, why)| *why)
}

/// Why a key with these modifiers belongs to the system: Cmd+key on macOS, Alt+key elsewhere.
fn system_reserved(mods: Mods, key: &str, mac: bool) -> Option<&'static str> {
    let cmd = Mods { cmd: true, ..Default::default() };
    let alt = Mods { alt: true, ..Default::default() };
    match mac {
        true if mods == cmd && matches!(key, "Q" | "W" | "H" | "M" | "Tab" | "Space") => Some("used by macOS"),
        false if mods == alt && matches!(key, "F4" | "Tab") => Some("used by the system"),
        _ => None,
    }
}

/// Why a combination can never be bound: the way back out, or the system's own keys.
pub fn reserved(combo: &Combo, mac: bool) -> Option<&'static str> {
    let key = combo.key.as_str();
    if let (true, Some(why)) = (combo.mods.none(), game_reserved(key)) {
        return Some(why);
    }
    system_reserved(combo.mods, key, mac)
}

/// Before binding `command` to `combo`: Err(reason) if it can never be bound,
/// Ok(Some(question)) if it replaces a game key or another binding, Ok(None) to bind.
pub fn bind_question(b: &Bindings, combo: &Combo, command: &str, mac: bool) -> Result<Option<String>, &'static str> {
    if let Some(reason) = reserved(combo, mac) {
        return Err(reason);
    }
    let name = combo.text(mac);
    Ok(match (b.lookup(combo), game_key(combo)) {
        (Some(old), _) if old.command == command => None,
        (Some(old), _) => Some(format!("{name} runs {}. Replace it? Type y or n.", old.command)),
        (None, Some(job)) => Some(format!("{name} is the {job} key. Replace it? Type y or n.")),
        (None, None) => None,
    })
}

/// An all-characters binding that this character's own binding on the same keys hides.
pub fn shadowed(b: &Bindings, new: &Binding) -> bool {
    new.all && b.list.iter().any(|x| !x.all && x.combo == new.combo)
}

/// A typed `bind` or `unbind` command (`lib/help/bind.en`).
#[derive(Debug, PartialEq)]
pub enum Typed {
    Bind { all: bool, edit: bool, command: String },
    /// An old client's special bind (`bind firekey1`, ...): not supported here.
    Special(String),
    List,
    Remove(usize),
    Reset,
    Usage,
}

const SPECIAL_BINDS: [&str; 7] = ["commandkey", "firekey1", "firekey2", "runkey1", "runkey2", "prevkey", "nextkey"];

/// None when the line is not `bind` or `unbind`.
pub fn parse_typed(line: &str) -> Option<Typed> {
    let (word, rest) = line.trim().split_once(' ').unwrap_or((line.trim(), ""));
    let rest = rest.trim();
    match word {
        "bind" => Some(parse_bind(rest)),
        "unbind" => Some(parse_unbind(rest)),
        _ => None,
    }
}

/// The arguments of `bind`: optional `-nfregc` flags, then the command.
fn parse_bind(rest: &str) -> Typed {
    let (mut all, mut edit, mut command) = (false, false, rest);
    if let Some(flags) = rest.strip_prefix('-') {
        let (flags, after) = flags.split_once(' ').unwrap_or((flags, ""));
        if !flags.chars().all(|c| "nfregc".contains(c)) {
            return Typed::Usage;
        }
        (all, edit, command) = (flags.contains('g'), flags.contains('e'), after.trim());
    }
    match command {
        "" => Typed::Usage,
        c if SPECIAL_BINDS.contains(&c) => Typed::Special(c.into()),
        c => Typed::Bind { all, edit, command: c.into() },
    }
}

/// The arguments of `unbind`: nothing (list), `reset`, or a binding number.
fn parse_unbind(rest: &str) -> Typed {
    match rest {
        "" => Typed::List,
        "reset" => Typed::Reset,
        n => n.parse().map_or(Typed::Usage, Typed::Remove),
    }
}

/// CROSSFIR3D_KEYS overrides it (screenshots).
pub fn default_keys_path() -> PathBuf {
    if let Some(p) = std::env::var_os("CROSSFIR3D_KEYS") {
        return PathBuf::from(p);
    }
    crate::settings::data_dir().join("keys.txt")
}

/// Lines: who<TAB>mods<TAB>key<TAB>flags<TAB>command; who is a character or `*` (all).
fn parse_line(line: &str, character: &str) -> Option<Binding> {
    let parts: Vec<&str> = line.splitn(5, '\t').collect();
    let [who, mods, key, flags, command] = parts[..] else { return None };
    if who != character && who != "*" || key.is_empty() || command.is_empty() {
        return None;
    }
    // Key names spell letters in capitals; a hand-written "h" means the same key.
    let key = if key.len() == 1 { key.to_ascii_uppercase() } else { key.into() };
    let combo = Combo { mods: Mods::parse(mods)?, key };
    Some(Binding { all: who == "*", combo, edit: flags.contains('e'), command: command.into() })
}

pub fn load_bindings(path: &Path, character: &str) -> Bindings {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    Bindings { list: text.lines().filter_map(|l| parse_line(l, character)).collect() }
}

/// Old file lines to keep: other characters', plus our own that do not load.
fn kept_lines(old: &str, character: &str) -> Vec<String> {
    // Other characters' lines stay, and so do this character's (and `*`) lines that do
    // not load (a typo in a hand edit): they are not ours to drop.
    let ours = |l: &str| matches!(l.split('\t').next(), Some(w) if w == character || w == "*");
    old.lines().filter(|l| !ours(l) || parse_line(l, character).is_none()).map(String::from).collect()
}

/// One file line for a binding, with tabs and newlines in the command turned into spaces.
fn binding_line(x: &Binding, character: &str) -> String {
    let who = if x.all { "*" } else { character };
    let mods = if x.combo.mods.none() { "-".to_string() } else { x.combo.mods.names(true).join("+") };
    let command = x.command.replace(['\t', '\n', '\r'], " ");
    format!("{who}\t{mods}\t{}\t{}\t{command}", x.combo.key, if x.edit { "e" } else { "-" })
}

/// Rewrites this character's and the all-characters lines, keeps other characters'
/// lines, and replaces the file in one rename (a crash cannot leave half a file).
pub fn save_bindings(path: &Path, character: &str, b: &Bindings) -> io::Result<()> {
    let old = std::fs::read_to_string(path).unwrap_or_default();
    let mut lines = kept_lines(&old, character);
    lines.extend(b.list.iter().map(|x| binding_line(x, character)));
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("txt.tmp");
    std::fs::write(&tmp, lines.join("\n") + "\n")?;
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn combo(mods: Mods, key: &str) -> Combo {
        Combo { mods, key: key.into() }
    }
    fn opt() -> Mods {
        Mods { alt: true, ..Default::default() }
    }
    fn temp_file(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("crossfir3d-keys-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_file(&p);
        p
    }

    #[test]
    fn combo_text_by_system() {
        let c = combo(Mods { shift: true, cmd: true, ..Default::default() }, "G");
        assert_eq!(c.text(true), "shift+cmd+g");
        assert_eq!(c.text(false), "Shift+Win+g");
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
    fn file_roundtrip_and_reset() {
        let path = temp_file("roundtrip");
        let mut b = Bindings::default();
        b.bind(Binding { all: false, combo: combo(opt(), "6"), edit: true, command: "cast burning hands".into() });
        b.bind(Binding { all: true, combo: combo(Mods::default(), "F3"), edit: false, command: "say hi".into() });
        save_bindings(&path, "Ann", &b).unwrap();
        assert_eq!(load_bindings(&path, "Ann"), b);
        b.reset();
        assert_eq!(b.list.len(), 1);
        assert!(b.list[0].all);
        assert_eq!(b.numbered(true), ["1. F3  say hi (all)"]);
    }

    #[test]
    fn file_skips_bad_lines_keeps_others() {
        let path = temp_file("bad");
        let bob = "Bob\tshift\tD\t-\tsay bob";
        std::fs::write(&path, format!("{bob}\nAnn\t-\tG\nAnn\tnonsense\tG\t-\tsay x\nAnn\toption\t6\t-\tsay ann\n")).unwrap();
        let mut b = load_bindings(&path, "Ann");
        assert_eq!(b.list.len(), 1);
        assert_eq!(b.list[0].command, "say ann");
        b.bind(Binding { all: false, combo: combo(Mods::default(), "Q"), edit: false, command: "a\tb\nc".into() });
        save_bindings(&path, "Ann", &b).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.lines().any(|l| l == bob));
        let again = load_bindings(&path, "Ann");
        assert_eq!(again.lookup(&combo(Mods::default(), "Q")).unwrap().command, "a b c");
    }

    #[test]
    fn typed_spell_is_the_spell() {
        let spells = vec!["burning hands".to_string()];
        assert_eq!(action_of("cast Burning Hands", &spells, &[]), Action::Spell("burning hands".into()));
        assert_eq!(action_of("invoke burning hands", &spells, &[]), Action::Spell("burning hands".into()));
        assert_eq!(action_of("use_skill praying", &[], &["praying".into()]), Action::Skill("praying".into()));
        assert_eq!(spell_command("burning hands"), "cast burning hands");
        assert_eq!(skill_command("praying"), "use_skill praying");
    }

    #[test]
    fn unknown_spell_is_plain_command() {
        assert_eq!(action_of("cast fireball", &[], &[]), Action::Command("cast fireball".into()));
        assert_eq!(
            action_of("cast burning hands; say hi", &["burning hands".into()], &[]),
            Action::Command("cast burning hands; say hi".into())
        );
    }

    #[test]
    fn game_key_only_without_mods() {
        assert_eq!(game_key(&Combo { mods: Mods::default(), key: "G".into() }), Some("get"));
        assert_eq!(game_key(&Combo { mods: Mods { shift: true, ..Default::default() }, key: "G".into() }), None);
        assert_eq!(game_key(&Combo { mods: Mods::default(), key: "D".into() }), None);
        assert_eq!(game_key(&Combo { mods: Mods::default(), key: "B".into() }), Some("what is below"));
        assert_eq!(game_key(&Combo { mods: Mods::default(), key: "Up".into() }), Some("move"));
    }

    #[test]
    fn game_key_every_key() {
        let job = |k: &str| game_key(&combo(Mods::default(), k));
        for k in ["Up", "Down", "Left", "Right"] {
            assert_eq!(job(k), Some("move"));
        }
        for k in ["1", "2", "3", "4"] {
            assert_eq!(job(k), Some("quick slot"));
        }
        let rows = [
            ("A", "apply"),
            ("E", "apply"),
            ("G", "get"),
            ("I", "inventory"),
            ("B", "what is below"),
            ("L", "log"),
            ("M", "mute"),
            ("C", "camera behind"),
            ("T", "face nearest"),
            ("F", "look at face"),
            ("Space", "fire"),
            ("Tab", "top view"),
        ];
        for (k, want) in rows {
            assert_eq!(job(k), Some(want));
        }
        for k in ["D", "K", "Escape", "5", "g"] {
            assert_eq!(job(k), None);
        }
        let shift = Mods { shift: true, ..Default::default() };
        let cmd = Mods { cmd: true, ..Default::default() };
        let ctrl = Mods { ctrl: true, ..Default::default() };
        assert_eq!(game_key(&combo(shift, "G")), None);
        assert_eq!(game_key(&combo(cmd, "Up")), None);
        assert_eq!(game_key(&combo(ctrl, "1")), None);
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
    fn reserved_table() {
        let m = |shift, alt, cmd| Mods { shift, alt, cmd, ..Default::default() };
        let (none, cmd, alt, cmd_shift) = (m(false, false, false), m(false, false, true), m(false, true, false), m(true, false, true));
        let esc = Some("Escape closes screens");
        let k = Some("K opens this screen");
        let enter = Some("Enter opens the command box");
        let mac = Some("used by macOS");
        let sys = Some("used by the system");
        // (mods, key, expected on mac, expected elsewhere)
        let rows = [
            (none, "Escape", esc, esc),
            (none, "K", k, k),
            (none, "Enter", enter, enter),
            (none, "Q", None, None),
            (none, "Tab", None, None),
            (none, "F4", None, None),
            (none, "A", None, None),
            (cmd, "Escape", None, None),
            (cmd, "K", None, None),
            (cmd, "Enter", None, None),
            (cmd, "Q", mac, None),
            (cmd, "W", mac, None),
            (cmd, "H", mac, None),
            (cmd, "M", mac, None),
            (cmd, "Tab", mac, None),
            (cmd, "Space", mac, None),
            (cmd, "F4", None, None),
            (cmd, "A", None, None),
            (alt, "F4", None, sys),
            (alt, "Tab", None, sys),
            (alt, "Q", None, None),
            (alt, "F5", None, None),
            (alt, "Up", None, None),
            (cmd_shift, "Q", None, None),
            (cmd_shift, "Tab", None, None),
            (cmd_shift, "F4", None, None),
            (cmd_shift, "K", None, None),
        ];
        for (mods, key, on_mac, elsewhere) in rows {
            let c = Combo { mods, key: key.into() };
            assert_eq!(reserved(&c, true), on_mac, "{mods:?} {key} mac");
            assert_eq!(reserved(&c, false), elsewhere, "{mods:?} {key} other");
        }
    }

    #[test]
    fn typed_commands() {
        assert_eq!(parse_typed("say hi"), None);
        assert_eq!(parse_typed("binder x"), None);
        assert_eq!(
            parse_typed("bind -ge cast paralyze"),
            Some(Typed::Bind { all: true, edit: true, command: "cast paralyze".into() })
        );
        assert_eq!(
            parse_typed("bind use_skill sense curse; use_skill sense magic"),
            Some(Typed::Bind { all: false, edit: false, command: "use_skill sense curse; use_skill sense magic".into() })
        );
        assert_eq!(
            parse_typed("bind -f cast paralyze"),
            Some(Typed::Bind { all: false, edit: false, command: "cast paralyze".into() })
        );
        assert_eq!(parse_typed("bind firekey1"), Some(Typed::Special("firekey1".into())));
        assert_eq!(parse_typed("bind -z x"), Some(Typed::Usage));
        assert_eq!(parse_typed("bind"), Some(Typed::Usage));
        assert_eq!(parse_typed("bind -g"), Some(Typed::Usage));
        assert_eq!(parse_typed("unbind"), Some(Typed::List));
        assert_eq!(parse_typed("unbind 3"), Some(Typed::Remove(3)));
        assert_eq!(parse_typed("unbind reset"), Some(Typed::Reset));
        assert_eq!(parse_typed("unbind x"), Some(Typed::Usage));
        assert_eq!(parse_typed("bind -"), Some(Typed::Usage));
        assert_eq!(parse_typed("bind -gx cmd"), Some(Typed::Usage));
        assert_eq!(
            parse_typed("bind  -g   spaced  cmd"),
            Some(Typed::Bind { all: true, edit: false, command: "spaced  cmd".into() })
        );
        assert_eq!(parse_typed("unbind 0"), Some(Typed::Remove(0)));
        assert_eq!(parse_typed("unbind -1"), Some(Typed::Usage));
        assert_eq!(parse_typed("unbind 99999999999"), Some(Typed::Remove(99999999999)));
        assert_eq!(
            parse_typed("  bind x  "),
            Some(Typed::Bind { all: false, edit: false, command: "x".into() })
        );
    }

    #[test]
    fn bind_questions() {
        let plain = |k: &str| Combo { mods: Mods::default(), key: k.into() };
        let b = Bindings::default();
        assert!(bind_question(&b, &plain("Escape"), "x", true).is_err());
        assert_eq!(bind_question(&b, &plain("G"), "x", true), Ok(Some("g is the get key. Replace it? Type y or n.".into())));
        assert_eq!(bind_question(&b, &plain("D"), "x", true), Ok(None));
        let mut b2 = Bindings::default();
        b2.bind(Binding { all: false, combo: plain("D"), edit: false, command: "say a".into() });
        assert_eq!(bind_question(&b2, &plain("D"), "x", true), Ok(Some("d runs say a. Replace it? Type y or n.".into())));
        assert_eq!(bind_question(&b2, &plain("D"), "say a", true), Ok(None));
    }

    #[test]
    fn hand_written_lower_case_letter_matches() {
        let path = temp_file("lower");
        std::fs::write(&path, "Ann\t-\th\t-\tsay hello\n").unwrap();
        let b = load_bindings(&path, "Ann");
        assert_eq!(b.lookup(&combo(Mods::default(), "H")).unwrap().command, "say hello");
    }

    #[test]
    fn save_keeps_own_lines_it_could_not_read() {
        let path = temp_file("keepbad");
        let bad = "*\tshfit\tG\t-\tsay typo";
        std::fs::write(&path, format!("{bad}\nAnn\t-\tQ\t-\tsay q\n")).unwrap();
        let b = load_bindings(&path, "Ann");
        save_bindings(&path, "Ann", &b).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.lines().any(|l| l == bad));
        assert_eq!(text.lines().filter(|l| l.ends_with("say q")).count(), 1);
    }

    #[test]
    fn all_binding_under_own_binding_is_shadowed() {
        let g = Combo { mods: Mods::default(), key: "G".into() };
        let mut b = Bindings::default();
        b.bind(Binding { all: false, combo: g.clone(), edit: false, command: "say me".into() });
        let all = Binding { all: true, combo: g.clone(), edit: false, command: "say all".into() };
        assert!(shadowed(&b, &all));
        assert!(!shadowed(&b, &Binding { all: false, ..all.clone() }));
        assert!(!shadowed(&Bindings::default(), &all));
    }

    #[test]
    fn missing_file_is_empty() {
        assert_eq!(load_bindings(&temp_file("missing"), "Ann"), Bindings::default());
    }
}
