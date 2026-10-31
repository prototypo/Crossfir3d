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

const BROWN: [f32; 3] = [0.45, 0.3, 0.2];
const CLOTH: [f32; 3] = [0.45, 0.4, 0.3];

/// The look of each race arch except human, which is the fallback.
const LOOKS: [(&str, Look); 13] = [
    ("elf_player", look(1.02, [0.95, 0.85, 0.75], [0.9, 0.85, 0.6], [0.3, 0.5, 0.3])),
    ("dwarf_player", look(0.78, [0.85, 0.65, 0.5], [0.6, 0.3, 0.15], [0.5, 0.35, 0.25])),
    ("gnome_player", look(0.7, [0.9, 0.72, 0.6], [0.8, 0.8, 0.8], [0.6, 0.25, 0.25])),
    ("halfling_player", look(0.72, [0.9, 0.7, 0.55], [0.5, 0.3, 0.15], [0.4, 0.5, 0.25])),
    ("northman_player", look(1.12, [0.95, 0.8, 0.7], [0.85, 0.7, 0.4], [0.35, 0.35, 0.5])),
    ("fenx_player", look(0.92, [0.85, 0.5, 0.2], [0.95, 0.9, 0.85], [0.35, 0.45, 0.25])),
    ("fireborn_player", look(1.0, [0.95, 0.4, 0.15], [1.0, 0.75, 0.2], [0.6, 0.2, 0.1])),
    ("quetzalcoatl_player", look(1.1, [0.3, 0.6, 0.35], [0.9, 0.3, 0.2], [0.8, 0.7, 0.2])),
    ("serpentman_player", look(1.0, [0.45, 0.6, 0.3], [0.2, 0.3, 0.15], [0.5, 0.45, 0.3])),
    ("troll_player", look(1.3, [0.45, 0.55, 0.45], [0.2, 0.2, 0.2], [0.4, 0.3, 0.2])),
    ("wraith_player", look(1.0, [0.8, 0.82, 0.88], [0.9, 0.9, 0.95], [0.25, 0.25, 0.3])),
    ("pl_dragon", look(0.95, [0.75, 0.15, 0.1], [0.3, 0.1, 0.05], [0.9, 0.6, 0.1])),
    ("pl_half_orc", look(1.06, [0.5, 0.58, 0.4], [0.15, 0.15, 0.15], BROWN)),
];

/// A body that is not human-shaped: its model file, scale and tint.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BeastBody {
    pub file: &'static str,
    pub scale: f32,
    pub tint: [f32; 3],
    /// How solid it is: 1 is solid, 0 cannot be seen.
    pub alpha: f32,
}

/// The races that are not human-shaped, by arch. Other races use the people's body.
const BEAST_BODIES: [(&str, BeastBody); 3] = [
    ("pl_dragon", BeastBody { file: "beasts/cute/YellowDragon.gltf", scale: 0.5, tint: [1.0, 0.5, 0.4], alpha: 1.0 }),
    // GhostShell is Ghost without the rear of its eyes and teeth, which would show through.
    ("wraith_player", BeastBody { file: "beasts/cute/GhostShell.gltf", scale: 0.4, tint: [0.8, 0.82, 0.88], alpha: 0.5 }),
    ("fenx_player", BeastBody { file: "beasts/animals/Fox.gltf", scale: 0.3, tint: [1.0, 1.0, 1.0], alpha: 1.0 }),
];

/// The arch of a race arch ("dwarf_player") or display name ("dwarf"). Unknown: human.
fn race_arch(race: &str) -> &'static str {
    RACES.iter().find(|(a, n)| *a == race || *n == race).map_or("human_player", |(a, _)| *a)
}

/// The race's body if it is not human-shaped; takes an arch or a display name.
pub fn beast_body(race: &str) -> Option<BeastBody> {
    let arch = race_arch(race);
    BEAST_BODIES.iter().find(|(a, _)| *a == arch).map(|(_, b)| *b)
}

const fn look(scale: f32, skin: [f32; 3], hair: [f32; 3], clothes: [f32; 3]) -> Look {
    Look { scale, skin, hair, clothes }
}

/// Accepts a race arch ("dwarf_player") or display name ("dwarf"). Unknown: human.
pub fn look_for_race(race: &str) -> Look {
    let arch = race_arch(race);
    LOOKS.iter().find(|(a, _)| *a == arch).map_or(look(1.0, [0.9, 0.72, 0.6], BROWN, CLOTH), |(_, l)| *l)
}

/// The look as multipliers for the model's own colours. Skin and clothes textures are a
/// human's, so a human gets ones; the hair texture is grey, so hair is the colour itself.
/// Scale is kept.
pub fn tints(look: &Look) -> Look {
    let human = look_for_race("human");
    let rel = |c: [f32; 3], h: [f32; 3]| [c[0] / h[0], c[1] / h[1], c[2] / h[2]];
    Look {
        scale: look.scale,
        skin: rel(look.skin, human.skin),
        hair: look.hair,
        clothes: rel(look.clothes, human.clothes),
    }
}

/// The triangles of a skinned mesh whose corners all follow head bones most. The free
/// body is a whole body; the outfits carry their own arms and legs, so only the head
/// of the body is shown. `is_head` is per skin joint.
pub fn head_triangles(indices: &[u32], joints: &[[u16; 4]], weights: &[[f32; 4]], is_head: &[bool]) -> Vec<u32> {
    let head_vertex = |v: u32| {
        let (j, w) = (joints[v as usize], weights[v as usize]);
        let strongest = (0..4).max_by(|&a, &b| w[a].total_cmp(&w[b])).unwrap_or(0);
        is_head.get(j[strongest] as usize).copied().unwrap_or(false)
    };
    indices.as_chunks::<3>().0.iter().filter(|t| t.iter().all(|&v| head_vertex(v))).flatten().copied().collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerFace {
    /// A race picture: the race arch.
    Race(&'static str),
    /// A class picture: the class arch.
    Class(&'static str),
}

/// Race picture names and the race arch each shows.
const RACE_FACES: [(&str, &str); 15] = [
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
    ("mountain_dwarf", "dwarf_player"),
];

/// Race picture name starts. Action and race+class pictures: dwarf_player_class_barbarian,
/// fenx_sword, pl_dragon_bl_spellcasting, wraith_player_feeding.
const RACE_PREFIXES: [(&str, &str); 5] = [
    ("dwarf_player_", "dwarf_player"),
    ("elf_player_", "elf_player"),
    ("fenx_", "fenx_player"),
    ("pl_dragon_", "pl_dragon"),
    ("wraith_player_", "wraith_player"),
];

/// Class picture names and the class arch each shows.
const CLASS_FACES: [(&str, &str); 22] = [
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
    ("cleric", "priest_class"),
    ("priest", "priest_class"),
    ("sorcerer_hat", "sorcerer_class"),
    ("wizard_blue", "wizard_class"),
    ("wizard_green", "wizard_class"),
    ("wizard_purple", "wizard_class"),
];

/// What a map picture name ("wizard.171", "dwarf_p.151") says about a player.
pub fn classify_player_face(face_name: &str) -> Option<PlayerFace> {
    let base = face_name.split('.').next().unwrap_or(face_name);
    let find = |table: &[(&str, &'static str)]| table.iter().find(|(f, _)| *f == base).map(|(_, a)| *a);
    let prefixed = || RACE_PREFIXES.iter().find(|(p, _)| base.starts_with(p)).map(|(_, a)| *a);
    find(&RACE_FACES)
        .or_else(prefixed)
        .map(PlayerFace::Race)
        .or_else(|| find(&CLASS_FACES).map(PlayerFace::Class))
}

/// The clothes colour of each class arch; classes may share a colour.
const CLASS_CLOTHES: [(&str, [f32; 3]); 16] = [
    ("warrior_class", [0.5, 0.35, 0.25]),
    ("barbarian_class", [0.5, 0.35, 0.25]),
    ("paladin_class", [0.8, 0.8, 0.85]),
    ("wizard_class", [0.25, 0.3, 0.6]),
    ("evoker_class", [0.25, 0.3, 0.6]),
    ("sorcerer_class", [0.25, 0.3, 0.6]),
    ("devotee_class", [0.85, 0.82, 0.7]),
    ("priest_class", [0.85, 0.82, 0.7]),
    ("monk_class", [0.85, 0.82, 0.7]),
    ("summoner_class", [0.7, 0.6, 0.2]),
    ("alchemist_class", [0.7, 0.6, 0.2]),
    ("warlock_class", [0.35, 0.15, 0.4]),
    ("thief_class", [0.15, 0.15, 0.18]),
    ("ninja_class", [0.15, 0.15, 0.18]),
    ("ranger_class", [0.25, 0.45, 0.25]),
    ("swashbuckler_class", [0.6, 0.2, 0.2]),
];

/// A clothes colour per class, so other players look different by class.
pub fn clothes_for_class(class_arch: &str) -> [f32; 3] {
    CLASS_CLOTHES.iter().find(|(c, _)| *c == class_arch).map_or(CLOTH, |(_, c)| *c)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Body {
    #[default]
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

/// Next to the quick slots and settings.
pub fn default_body_path() -> std::path::PathBuf {
    crate::settings::data_dir().join("bodies.txt")
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
    fn tints_are_relative_to_a_human() {
        let human = tints(&look_for_race("human"));
        assert_eq!(human.skin, [1.0; 3]);
        // The hair texture is grey, so hair takes the colour itself.
        assert_eq!(human.hair, look_for_race("human").hair);
        assert_eq!(human.clothes, [1.0; 3]);
        let troll = tints(&look_for_race("troll"));
        assert!(troll.skin[0] < troll.skin[1], "troll skin is greener than human");
        assert_eq!(troll.scale, look_for_race("troll").scale);
    }

    #[test]
    fn only_head_triangles_kept() {
        // Joints: 0 = pelvis, 1 = Head. Vertices 0..3 follow the head, 3 follows the pelvis.
        let joints = [[1, 0, 0, 0], [1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 0, 0]];
        let weights = [[1.0, 0.0, 0.0, 0.0], [0.9, 0.1, 0.0, 0.0], [0.3, 0.7, 0.0, 0.0], [1.0, 0.0, 0.0, 0.0]];
        let head = [false, true];
        assert_eq!(head_triangles(&[0, 1, 2, 0, 1, 3], &joints, &weights, &head), vec![0, 1, 2]);
    }

    #[test]
    fn every_race_look_unchanged() {
        let brown = [0.45, 0.3, 0.2];
        let cloth = [0.45, 0.4, 0.3];
        let rows: [(&str, &str, Look); 14] = [
            ("human_player", "human", Look { scale: 1.0, skin: [0.9, 0.72, 0.6], hair: brown, clothes: cloth }),
            ("elf_player", "elf", Look { scale: 1.02, skin: [0.95, 0.85, 0.75], hair: [0.9, 0.85, 0.6], clothes: [0.3, 0.5, 0.3] }),
            ("dwarf_player", "dwarf", Look { scale: 0.78, skin: [0.85, 0.65, 0.5], hair: [0.6, 0.3, 0.15], clothes: [0.5, 0.35, 0.25] }),
            ("gnome_player", "gnome", Look { scale: 0.7, skin: [0.9, 0.72, 0.6], hair: [0.8, 0.8, 0.8], clothes: [0.6, 0.25, 0.25] }),
            ("halfling_player", "halfling", Look { scale: 0.72, skin: [0.9, 0.7, 0.55], hair: [0.5, 0.3, 0.15], clothes: [0.4, 0.5, 0.25] }),
            ("northman_player", "northman", Look { scale: 1.12, skin: [0.95, 0.8, 0.7], hair: [0.85, 0.7, 0.4], clothes: [0.35, 0.35, 0.5] }),
            ("fenx_player", "fenx", Look { scale: 0.92, skin: [0.85, 0.5, 0.2], hair: [0.95, 0.9, 0.85], clothes: [0.35, 0.45, 0.25] }),
            ("fireborn_player", "fireborn", Look { scale: 1.0, skin: [0.95, 0.4, 0.15], hair: [1.0, 0.75, 0.2], clothes: [0.6, 0.2, 0.1] }),
            ("quetzalcoatl_player", "quetzalcoatl", Look { scale: 1.1, skin: [0.3, 0.6, 0.35], hair: [0.9, 0.3, 0.2], clothes: [0.8, 0.7, 0.2] }),
            ("serpentman_player", "serpentfolk", Look { scale: 1.0, skin: [0.45, 0.6, 0.3], hair: [0.2, 0.3, 0.15], clothes: [0.5, 0.45, 0.3] }),
            ("troll_player", "troll", Look { scale: 1.3, skin: [0.45, 0.55, 0.45], hair: [0.2, 0.2, 0.2], clothes: [0.4, 0.3, 0.2] }),
            ("wraith_player", "wraith", Look { scale: 1.0, skin: [0.8, 0.82, 0.88], hair: [0.9, 0.9, 0.95], clothes: [0.25, 0.25, 0.3] }),
            ("pl_dragon", "fire hatchling", Look { scale: 0.95, skin: [0.75, 0.15, 0.1], hair: [0.3, 0.1, 0.05], clothes: [0.9, 0.6, 0.1] }),
            ("pl_half_orc", "half orc", Look { scale: 1.06, skin: [0.5, 0.58, 0.4], hair: [0.15, 0.15, 0.15], clothes: brown }),
        ];
        for (arch, name, look) in rows {
            assert_eq!(look_for_race(arch), look, "{arch}");
            assert_eq!(look_for_race(name), look, "{name}");
        }
        let human = rows[0].2;
        assert_eq!(look_for_race(""), human);
        assert_eq!(look_for_race("martian"), human);
    }

    #[test]
    fn beast_bodies_by_arch_or_name() {
        assert_eq!(beast_body("fire hatchling"), beast_body("pl_dragon"));
        assert!(beast_body("fire hatchling").is_some() && beast_body("wraith").is_some() && beast_body("fenx").is_some());
        assert_eq!(beast_body("troll"), None);
        assert_eq!(beast_body("human"), None);
        assert_eq!(beast_body("no such race"), None);
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
    fn every_player_picture_is_a_3d_player() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../upstream/crossfire-arch/player");
        let Ok(walk) = std::fs::read_dir(dir) else { return }; // no server data checked out
        let mut stack: Vec<std::path::PathBuf> = walk.flatten().map(|e| e.path()).collect();
        let mut missed = Vec::new();
        while let Some(p) = stack.pop() {
            // A player turned to stone is a statue, not a player (assets/models.txt).
            if p.ends_with("stoned") {
                continue;
            }
            if p.is_dir() {
                stack.extend(std::fs::read_dir(&p).unwrap().flatten().map(|e| e.path()));
            } else if let Some(name) = p.file_name().and_then(|n| n.to_str()).filter(|n| n.ends_with(".111.png")) {
                let face = format!("{}.111", name.split('.').next().unwrap());
                if classify_player_face(&face).is_none() {
                    missed.push(face);
                }
            }
        }
        missed.sort();
        missed.dedup();
        assert!(missed.is_empty(), "would show as 2D pictures: {missed:?}");
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
    fn player_faces_from_every_table() {
        let f = classify_player_face;
        assert_eq!(f("viking.111"), Some(PlayerFace::Race("northman_player")));
        assert_eq!(f("mountain_dwarf.151"), Some(PlayerFace::Race("dwarf_player")));
        assert_eq!(f("wraithp.111"), Some(PlayerFace::Race("wraith_player")));
        assert_eq!(f("fenx_sword.111"), Some(PlayerFace::Race("fenx_player")));
        assert_eq!(f("dwarf_player_class_barbarian.111"), Some(PlayerFace::Race("dwarf_player")));
        assert_eq!(f("pl_dragon_bl_spellcasting.111"), Some(PlayerFace::Race("pl_dragon")));
        assert_eq!(f("mage.111"), Some(PlayerFace::Class("warlock_class")));
        assert_eq!(f("cleric.111"), Some(PlayerFace::Class("priest_class")));
        assert_eq!(f("wizard_purple.111"), Some(PlayerFace::Class("wizard_class")));
        assert_eq!(f("x"), None);
    }

    #[test]
    fn every_stock_player_picture_is_known() {
        let race = |f| classify_player_face(f);
        assert_eq!(race("dwarf_player_class_barbarian.151"), Some(PlayerFace::Race("dwarf_player")));
        assert_eq!(race("elf_player_class_ranger.151"), Some(PlayerFace::Race("elf_player")));
        assert_eq!(race("fenx_spellcasting.151"), Some(PlayerFace::Race("fenx_player")));
        assert_eq!(race("fenx_class_warrior.151"), Some(PlayerFace::Race("fenx_player")));
        assert_eq!(race("pl_dragon_bl_spellcasting.151"), Some(PlayerFace::Race("pl_dragon")));
        assert_eq!(race("pl_dragon_g.151"), Some(PlayerFace::Race("pl_dragon")));
        assert_eq!(race("wraith_player_feeding.151"), Some(PlayerFace::Race("wraith_player")));
        assert_eq!(race("mountain_dwarf.151"), Some(PlayerFace::Race("dwarf_player")));
        assert_eq!(race("cleric.151"), Some(PlayerFace::Class("priest_class")));
        assert_eq!(race("priest.151"), Some(PlayerFace::Class("priest_class")));
        assert_eq!(race("sorcerer_hat.151"), Some(PlayerFace::Class("sorcerer_class")));
        assert_eq!(race("wizard_purple.151"), Some(PlayerFace::Class("wizard_class")));
        // Statues of players stay pictures.
        assert_eq!(race("dwarf_stoned.151"), None);
        assert_eq!(race("dragon_stoned.151"), None);
    }

    #[test]
    fn classes_have_distinct_clothes() {
        assert_ne!(clothes_for_class("wizard_class"), clothes_for_class("warrior_class"));
        let _ = clothes_for_class("unknown_class");
    }

    #[test]
    fn every_class_has_its_clothes() {
        let rows: [(&str, [f32; 3]); 16] = [
            ("warrior", [0.5, 0.35, 0.25]), ("barbarian", [0.5, 0.35, 0.25]),
            ("paladin", [0.8, 0.8, 0.85]),
            ("wizard", [0.25, 0.3, 0.6]), ("evoker", [0.25, 0.3, 0.6]), ("sorcerer", [0.25, 0.3, 0.6]),
            ("devotee", [0.85, 0.82, 0.7]), ("priest", [0.85, 0.82, 0.7]), ("monk", [0.85, 0.82, 0.7]),
            ("summoner", [0.7, 0.6, 0.2]), ("alchemist", [0.7, 0.6, 0.2]),
            ("warlock", [0.35, 0.15, 0.4]),
            ("thief", [0.15, 0.15, 0.18]), ("ninja", [0.15, 0.15, 0.18]),
            ("ranger", [0.25, 0.45, 0.25]),
            ("swashbuckler", [0.6, 0.2, 0.2]),
        ];
        for (name, colour) in rows {
            assert_eq!(clothes_for_class(&format!("{name}_class")), colour, "{name}");
        }
        assert_eq!(clothes_for_class(""), [0.45, 0.4, 0.3]);
        assert_eq!(clothes_for_class("x_class"), [0.45, 0.4, 0.3]);
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
