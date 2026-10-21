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
    /// "shift+cmd+G" on a Mac, "Shift+Win+G" elsewhere.
    pub fn text(&self, mac: bool) -> String {
        let mut parts = self.mods.names(mac);
        parts.push(&self.key);
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

pub fn default_keys_path() -> PathBuf {
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    home.join("Library/Application Support/MacCrossfire/keys.txt")
}

/// Lines: who<TAB>mods<TAB>key<TAB>flags<TAB>command; who is a character or `*` (all).
fn parse_line(line: &str, character: &str) -> Option<Binding> {
    let parts: Vec<&str> = line.splitn(5, '\t').collect();
    let [who, mods, key, flags, command] = parts[..] else { return None };
    if who != character && who != "*" || key.is_empty() || command.is_empty() {
        return None;
    }
    let combo = Combo { mods: Mods::parse(mods)?, key: key.into() };
    Some(Binding { all: who == "*", combo, edit: flags.contains('e'), command: command.into() })
}

pub fn load_bindings(path: &Path, character: &str) -> Bindings {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    Bindings { list: text.lines().filter_map(|l| parse_line(l, character)).collect() }
}

/// Rewrites this character's and the all-characters lines, keeps other characters'
/// lines, and replaces the file in one rename (a crash cannot leave half a file).
pub fn save_bindings(path: &Path, character: &str, b: &Bindings) -> io::Result<()> {
    let old = std::fs::read_to_string(path).unwrap_or_default();
    let mut lines: Vec<String> = old
        .lines()
        .filter(|l| !matches!(l.split('\t').next(), Some(w) if w == character || w == "*"))
        .map(String::from)
        .collect();
    for x in &b.list {
        let who = if x.all { "*" } else { character };
        let mods = if x.combo.mods.none() { "-".to_string() } else { x.combo.mods.names(true).join("+") };
        let command = x.command.replace(['\t', '\n', '\r'], " ");
        lines.push(format!("{who}\t{mods}\t{}\t{}\t{command}", x.combo.key, if x.edit { "e" } else { "-" }));
    }
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
        let p = std::env::temp_dir().join(format!("maccrossfire-keys-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_file(&p);
        p
    }

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
    fn missing_file_is_empty() {
        assert_eq!(load_bindings(&temp_file("missing"), "Ann"), Bindings::default());
    }
}
