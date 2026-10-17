//! How each race looks on the shared 3D body, and what a player picture says.
//!
//! In Crossfire a player's picture usually shows their class (wizard, warrior, ...);
//! only some races keep a race picture. Names come from crossfire-arch `player/`.

use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Look {
    pub scale: f32,
    pub skin: [f32; 3],
    pub hair: [f32; 3],
    pub clothes: [f32; 3],
}

/// Race arch names and display names (arch, name).
pub const RACES: [(&str, &str); 14] = [
    ("human_player", "human"),
    ("elf_player", "elf"),
    ("dwarf_player", "dwarf"),
    ("gnome_player", "gnome"),
    ("halfling_player", "halfling"),
    ("northman_player", "northman"),
    ("fenx_player", "fenx"),
    ("fireborn_player", "fireborn"),
    ("quetzalcoatl_player", "quetzalcoatl"),
    ("serpentman_player", "serpentfolk"),
    ("troll_player", "troll"),
    ("wraith_player", "wraith"),
    ("pl_dragon", "fire hatchling"),
    ("pl_half_orc", "half orc"),
];

/// Accepts a race arch ("dwarf_player") or display name ("dwarf"). Unknown: human.
pub fn look_for_race(race: &str) -> Look {
    let arch = RACES.iter().find(|(a, n)| *a == race || *n == race).map_or("human_player", |(a, _)| *a);
    let look = |scale, skin, hair, clothes| Look { scale, skin, hair, clothes };
    let brown = [0.45, 0.3, 0.2];
    let cloth = [0.45, 0.4, 0.3];
    match arch {
        "elf_player" => look(1.02, [0.95, 0.85, 0.75], [0.9, 0.85, 0.6], [0.3, 0.5, 0.3]),
        "dwarf_player" => look(0.78, [0.85, 0.65, 0.5], [0.6, 0.3, 0.15], [0.5, 0.35, 0.25]),
        "gnome_player" => look(0.7, [0.9, 0.72, 0.6], [0.8, 0.8, 0.8], [0.6, 0.25, 0.25]),
        "halfling_player" => look(0.72, [0.9, 0.7, 0.55], [0.5, 0.3, 0.15], [0.4, 0.5, 0.25]),
        "northman_player" => look(1.12, [0.95, 0.8, 0.7], [0.85, 0.7, 0.4], [0.35, 0.35, 0.5]),
        "fenx_player" => look(0.92, [0.85, 0.5, 0.2], [0.95, 0.9, 0.85], [0.35, 0.45, 0.25]),
        "fireborn_player" => look(1.0, [0.95, 0.4, 0.15], [1.0, 0.75, 0.2], [0.6, 0.2, 0.1]),
        "quetzalcoatl_player" => look(1.1, [0.3, 0.6, 0.35], [0.9, 0.3, 0.2], [0.8, 0.7, 0.2]),
        "serpentman_player" => look(1.0, [0.45, 0.6, 0.3], [0.2, 0.3, 0.15], [0.5, 0.45, 0.3]),
        "troll_player" => look(1.3, [0.45, 0.55, 0.45], [0.2, 0.2, 0.2], [0.4, 0.3, 0.2]),
        "wraith_player" => look(1.0, [0.8, 0.82, 0.88], [0.9, 0.9, 0.95], [0.25, 0.25, 0.3]),
        "pl_dragon" => look(0.95, [0.75, 0.15, 0.1], [0.3, 0.1, 0.05], [0.9, 0.6, 0.1]),
        "pl_half_orc" => look(1.06, [0.5, 0.58, 0.4], [0.15, 0.15, 0.15], brown),
        _ => look(1.0, [0.9, 0.72, 0.6], brown, cloth),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerFace {
    /// A race picture: the race arch.
    Race(&'static str),
    /// A class picture: the class arch.
    Class(&'static str),
}

/// What a map picture name ("wizard.171", "dwarf_p.151") says about a player.
pub fn classify_player_face(face_name: &str) -> Option<PlayerFace> {
    const RACE_FACES: [(&str, &str); 14] = [
        ("dwarf_p", "dwarf_player"),
        ("elf", "elf_player"),
        ("fenx", "fenx_player"),
        ("fireborn", "fireborn_player"),
        ("gnome_p", "gnome_player"),
        ("halfling", "halfling_player"),
        ("human", "human_player"),
        ("viking", "northman_player"),
        ("pl_dragon_r", "pl_dragon"),
        ("pl_half_orc", "pl_half_orc"),
        ("quetzalcoatl", "quetzalcoatl_player"),
        ("serpentman", "serpentman_player"),
        ("troll_p", "troll_player"),
        ("wraithp", "wraith_player"),
    ];
    const CLASS_FACES: [(&str, &str); 16] = [
        ("alchemist", "alchemist_class"),
        ("barbarian", "barbarian_class"),
        ("wizard_white", "devotee_class"),
        ("evoker", "evoker_class"),
        ("monk", "monk_class"),
        ("ninja", "ninja_class"),
        ("paladin", "paladin_class"),
        ("priestnew", "priest_class"),
        ("ranger", "ranger_class"),
        ("sorcerer", "sorcerer_class"),
        ("wizard_yellow", "summoner_class"),
        ("swashbuckl", "swashbuckler_class"),
        ("thief", "thief_class"),
        ("mage", "warlock_class"),
        ("warrior", "warrior_class"),
        ("wizard", "wizard_class"),
    ];
    let base = face_name.split('.').next().unwrap_or(face_name);
    let find = |table: &[(&str, &'static str)]| table.iter().find(|(f, _)| *f == base).map(|(_, a)| *a);
    find(&RACE_FACES).map(PlayerFace::Race).or_else(|| find(&CLASS_FACES).map(PlayerFace::Class))
}

/// A clothes colour per class, so other players look different by class.
pub fn clothes_for_class(class_arch: &str) -> [f32; 3] {
    match class_arch {
        "warrior_class" | "barbarian_class" => [0.5, 0.35, 0.25],
        "paladin_class" => [0.8, 0.8, 0.85],
        "wizard_class" | "evoker_class" | "sorcerer_class" => [0.25, 0.3, 0.6],
        "devotee_class" | "priest_class" | "monk_class" => [0.85, 0.82, 0.7],
        "summoner_class" | "alchemist_class" => [0.7, 0.6, 0.2],
        "warlock_class" => [0.35, 0.15, 0.4],
        "thief_class" | "ninja_class" => [0.15, 0.15, 0.18],
        "ranger_class" => [0.25, 0.45, 0.25],
        "swashbuckler_class" => [0.6, 0.2, 0.2],
        _ => [0.45, 0.4, 0.3],
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Body {
    Regular,
    Other,
}

/// Lines `character<TAB>regular|other`, kept next to the quick slots.
pub fn load_body(path: &Path, character: &str) -> Body {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let line = text.lines().find_map(|l| l.split_once('\t').filter(|(who, _)| *who == character));
    match line {
        Some((_, "other")) => Body::Other,
        _ => Body::Regular,
    }
}

/// Rewrites this character's line and keeps the others.
pub fn save_body(path: &Path, character: &str, body: Body) -> std::io::Result<()> {
    let old = std::fs::read_to_string(path).unwrap_or_default();
    let mut lines: Vec<String> =
        old.lines().filter(|l| l.split('\t').next() != Some(character)).map(String::from).collect();
    lines.push(format!("{character}\t{}", if body == Body::Other { "other" } else { "regular" }));
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, lines.join("\n") + "\n")
}

/// Next to the quick slots and settings. ponytail: Mac path; `dirs` crate in M7.
pub fn default_body_path() -> std::path::PathBuf {
    let home = std::env::var_os("HOME").map(std::path::PathBuf::from).unwrap_or_default();
    home.join("Library/Application Support/MacCrossfire/bodies.txt")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_stock_race_has_a_look() {
        let human = look_for_race("human_player");
        for (arch, name) in RACES {
            assert_eq!(look_for_race(arch), look_for_race(name), "{arch}");
            if arch != "human_player" {
                assert_ne!(look_for_race(arch), human, "{arch} looks like a human");
            }
        }
    }

    #[test]
    fn unknown_race_is_human() {
        assert_eq!(look_for_race("martian"), look_for_race("human"));
    }

    #[test]
    fn gnome_small_troll_big() {
        assert!(look_for_race("gnome").scale < 0.85);
        assert!(look_for_race("troll").scale > 1.15);
    }

    #[test]
    fn player_faces_classified() {
        assert_eq!(classify_player_face("dwarf_p.151"), Some(PlayerFace::Race("dwarf_player")));
        assert_eq!(classify_player_face("pl_dragon_r.151"), Some(PlayerFace::Race("pl_dragon")));
        assert_eq!(classify_player_face("wizard.171"), Some(PlayerFace::Class("wizard_class")));
        assert_eq!(classify_player_face("wizard_white.171"), Some(PlayerFace::Class("devotee_class")));
        assert_eq!(classify_player_face("swashbuckl.151"), Some(PlayerFace::Class("swashbuckler_class")));
        assert_eq!(classify_player_face("goblin.111"), None);
    }

    #[test]
    fn classes_have_distinct_clothes() {
        assert_ne!(clothes_for_class("wizard_class"), clothes_for_class("warrior_class"));
        let _ = clothes_for_class("unknown_class");
    }

    #[test]
    fn body_round_trip_keeps_others() {
        let p = std::env::temp_dir().join(format!("mcf-body-{}", std::process::id()));
        let _ = std::fs::remove_file(&p);
        assert_eq!(load_body(&p, "Ann"), Body::Regular);
        save_body(&p, "Ann", Body::Other).unwrap();
        save_body(&p, "Bob", Body::Regular).unwrap();
        assert_eq!(load_body(&p, "Ann"), Body::Other);
        assert_eq!(load_body(&p, "Bob"), Body::Regular);
    }
}
