//! Facing direction and the 4 quick slots (like Ocarina's C-buttons).

use std::io;
use std::path::{Path, PathBuf};

use cfproto::{Inventory, SpellBook};

/// Item client types that are fired, not just applied: bows, crossbows, and
/// rods/wands/staffs/horns (from `client_type` in crossfire-arch).
fn is_fired(kind: u16) -> bool {
    matches!(kind, 150 | 151 | 161 | 700..=799)
}

/// "north" -> 1 ... "northwest" -> 8 (the server's direction numbers).
pub fn dir_number(command: &str) -> Option<u8> {
    const DIRS: [&str; 8] = ["north", "northeast", "east", "southeast", "south", "southwest", "west", "northwest"];
    DIRS.iter().position(|d| *d == command).map(|i| i as u8 + 1)
}

#[derive(Clone, Debug, PartialEq)]
pub enum Slot {
    Spell(String),
    Item(String),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct QuickSlots(pub [Option<Slot>; 4]);

/// ncom command strings that use the slot, facing direction `facing`. Empty if the
/// character no longer has the spell or item.
/// `range` is the server's "Range: ..." stat: which item or spell `fire` will use.
pub fn slot_commands(slot: &Slot, inv: &Inventory, spells: &SpellBook, facing: u8, range: &str) -> Vec<String> {
    match slot {
        Slot::Spell(name) => match spells.by_name(name) {
            Some(spell) => [vec![format!("cast {}", spell.tag)], fire(facing)].concat(),
            None => vec![],
        },
        Slot::Item(name) => match inv.find_by_name(name) {
            Some(item) if is_fired(item.kind) => fire_item(item, facing, range),
            Some(item) => vec![format!("apply {}", item.tag)],
            None => vec![],
        },
    }
}

fn fire(facing: u8) -> Vec<String> {
    vec![format!("fire {facing}"), "fire_stop".to_string()]
}

/// Ready `item` (a bow, wand, ...) for `fire` if it is not, then fire it.
fn fire_item(item: &cfproto::Item, facing: u8, range: &str) -> Vec<String> {
    let apply = format!("apply {}", item.tag);
    // Applied is not enough: `fire` uses whatever is in range (maybe the last spell, or
    // nothing at all just after login).
    let in_range = range == format!("Range: {}", item.name) || range.starts_with(&format!("Range: {} (", item.name));
    let ready = match (item.applied(), in_range) {
        (true, true) => vec![],
        (true, false) => vec![apply.clone(), apply], // unready, then ready again
        (false, _) => vec![apply],
    };
    [ready, fire(facing)].concat()
}

/// Space: fire the readied bow, wand, rod, or horn the way the player faces. Empty if
/// none is readied.
pub fn fire_commands(inv: &Inventory, facing: u8, range: &str) -> Vec<String> {
    inv.carried().into_iter().find(|i| i.applied() && is_fired(i.kind)).map_or_else(Vec::new, |i| fire_item(i, facing, range))
}

pub fn usable(slot: &Slot, inv: &Inventory, spells: &SpellBook) -> bool {
    match slot {
        Slot::Spell(name) => spells.by_name(name).is_some(),
        Slot::Item(name) => inv.find_by_name(name).is_some(),
    }
}

/// ponytail: Mac path; the `dirs` crate in M7.
pub fn default_slots_path() -> PathBuf {
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    home.join("Library/Application Support/Crossfir3d/slots.txt")
}

/// Lines: character<TAB>index<TAB>spell|item<TAB>name.
pub fn load_slots(path: &Path, character: &str) -> QuickSlots {
    let mut slots = QuickSlots::default();
    let text = std::fs::read_to_string(path).unwrap_or_default();
    for line in text.lines() {
        let parts: Vec<&str> = line.splitn(4, '\t').collect();
        let [who, index, kind, name] = parts[..] else { continue };
        let Ok(i @ 0..=3) = index.parse::<usize>() else { continue };
        if who != character {
            continue;
        }
        slots.0[i] = match kind {
            "spell" => Some(Slot::Spell(name.into())),
            "item" => Some(Slot::Item(name.into())),
            _ => None,
        };
    }
    slots
}

/// Rewrites this character's lines and keeps other characters' lines.
pub fn save_slots(path: &Path, character: &str, slots: &QuickSlots) -> io::Result<()> {
    let old = std::fs::read_to_string(path).unwrap_or_default();
    let mut lines: Vec<String> =
        old.lines().filter(|l| l.split('\t').next() != Some(character)).map(String::from).collect();
    for (i, slot) in slots.0.iter().enumerate() {
        match slot {
            Some(Slot::Spell(n)) => lines.push(format!("{character}\t{i}\tspell\t{n}")),
            Some(Slot::Item(n)) => lines.push(format!("{character}\t{i}\titem\t{n}")),
            None => {}
        }
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, lines.join("\n") + "\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use cfproto::{Item, Spell};

    fn item(tag: u32, name: &str, kind: u16, flags: u32) -> Item {
        Item {
            tag,
            location: 1,
            flags,
            weight: 0,
            face: 0,
            name: name.into(),
            plural: name.into(),
            anim: 0,
            anim_speed: 0,
            nrof: 1,
            kind,
        }
    }

    fn world() -> (Inventory, SpellBook) {
        let mut inv = Inventory::default();
        inv.player = 1;
        inv.apply_item2(1, vec![item(10, "wand of fire", 711, 0), item(11, "bow", 151, 0x0001), item(12, "potion of healing", 60, 0)]);
        let mut book = SpellBook::default();
        book.add(vec![Spell {
            tag: 77,
            level: 1,
            casting_time: 0,
            mana: 3,
            grace: 0,
            damage: 4,
            skill: 0,
            path: 0,
            face: 0,
            name: "small fireball".into(),
            message: String::new(),
        }]);
        (inv, book)
    }

    #[test]
    fn dir_numbers() {
        let all = ["north", "northeast", "east", "southeast", "south", "southwest", "west", "northwest"];
        for (i, d) in all.iter().enumerate() {
            assert_eq!(dir_number(d), Some(i as u8 + 1));
        }
        assert_eq!(dir_number("up"), None);
    }

    #[test]
    fn space_fires_the_readied_bow() {
        let (inv, _) = world();
        assert_eq!(fire_commands(&inv, 3, "Range: nothing"), ["apply 11", "apply 11", "fire 3", "fire_stop"]);
        assert_eq!(fire_commands(&inv, 3, "Range: bow (arrows)"), ["fire 3", "fire_stop"]);
        let mut empty = Inventory::default();
        empty.player = 1;
        assert!(fire_commands(&empty, 3, "Range: nothing").is_empty());
    }

    #[test]
    fn spell_slot_casts_then_fires() {
        let (inv, book) = world();
        assert_eq!(slot_commands(&Slot::Spell("small fireball".into()), &inv, &book, 3, ""), ["cast 77", "fire 3", "fire_stop"]);
    }

    #[test]
    fn wand_slot_applies_once_then_fires() {
        let (inv, book) = world();
        assert_eq!(slot_commands(&Slot::Item("wand of fire".into()), &inv, &book, 5, ""), ["apply 10", "fire 5", "fire_stop"]);
        let ready = "Range: bow (arrow)";
        assert_eq!(slot_commands(&Slot::Item("bow".into()), &inv, &book, 5, ready), ["fire 5", "fire_stop"]); // already applied
    }

    #[test]
    fn applied_but_not_readied_reapplies() {
        // The last cast spell is in range: re-ready the bow (unapply, apply) before firing.
        let (inv, book) = world();
        let range = "Range: spell (small fireball)";
        assert_eq!(slot_commands(&Slot::Item("bow".into()), &inv, &book, 5, range), ["apply 11", "apply 11", "fire 5", "fire_stop"]);
    }

    #[test]
    fn potion_slot_applies() {
        let (inv, book) = world();
        assert_eq!(slot_commands(&Slot::Item("potion of healing".into()), &inv, &book, 1, ""), ["apply 12"]);
    }

    #[test]
    fn missing_item_sends_nothing() {
        let (inv, book) = world();
        let gone = Slot::Item("scroll of light".into());
        assert!(slot_commands(&gone, &inv, &book, 1, "").is_empty());
        assert!(!usable(&gone, &inv, &book));
        assert!(usable(&Slot::Spell("small fireball".into()), &inv, &book));
    }

    fn temp_file(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("crossfir3d-slots-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_file(&p);
        p
    }

    #[test]
    fn slots_roundtrip_keeps_other_characters() {
        let path = temp_file("roundtrip");
        let a = QuickSlots([Some(Slot::Spell("small fireball".into())), None, Some(Slot::Item("potion of healing".into())), None]);
        let b = QuickSlots([None, Some(Slot::Item("bow".into())), None, None]);
        save_slots(&path, "Ann", &a).unwrap();
        save_slots(&path, "Bob", &b).unwrap();
        save_slots(&path, "Ann", &a).unwrap();
        assert_eq!(load_slots(&path, "Ann"), a);
        assert_eq!(load_slots(&path, "Bob"), b);
    }

    #[test]
    fn missing_slot_file_is_empty() {
        assert_eq!(load_slots(&temp_file("missing"), "Ann"), QuickSlots::default());
    }
}
