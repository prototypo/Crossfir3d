//! Which picture becomes which 3D visual: `assets/models.txt`.
//!
//! Lines: `pattern kind args...`; `#` starts a comment; a pattern ending in `*` is a
//! prefix. Kinds:
//! - `model <file> <scale> <rot_deg> <dy> [r,g,b]`
//! - `building <recipe> [SIGN_TEXT [icon]]` (`_` in the sign is a space; icons in signs.rs)
//! - `wall <style>`
//! - `person <regular|other> <scale> <skin> <hair> <clothes> [dress r,g,b] [broom]` (colours
//!   as r,g,b: a 3D character, like a player, for a monster or townsperson)
//! - `monster <file> <scale> [r,g,b]` (a rigged creature model, moved by the same clips
//!   as people; scale 1 = the size a person's body is drawn at)
//! - `flat` (a floor decoration lying on the ground)
//! - `card` (keep the picture card)

use crate::held::{class_look, ClassLook};
use crate::races::{Body, Look};

#[derive(Clone, Debug, PartialEq)]
pub enum Visual {
    Model { file: String, scale: f32, rot_deg: f32, dy: f32, tint: Option<[f32; 3]> },
    Building { recipe: String, sign: Option<String>, icon: Option<String> },
    Wall { style: String },
    /// A person in stone on a plinth: a half-size 3D character, maybe tinted (bronze).
    Statue { tint: Option<[f32; 3]> },
    /// A creature drawn as a 3D character with this body and look, maybe in a long
    /// skirt of this colour, maybe with a broom in the right hand, maybe in metal armour,
    /// `wide` times as broad and with its head `head` times as big, maybe dressed as a class.
    /// A creature (not a human) also has a `build`, rags of a colour over its clothes, and
    /// horns or pointed ears (and no hair).
    Person {
        body: Body,
        look: Look,
        dress: Option<[f32; 3]>,
        broom: bool,
        armour: bool,
        wide: f32,
        head: f32,
        class: Option<ClassLook>,
        build: Option<Build>,
        rags: Option<[f32; 3]>,
        horns: bool,
        ears: bool,
    },
    /// A rigged creature model (one file) moved like a person.
    Monster { file: String, scale: f32, tint: Option<[f32; 3]> },
    /// A walking skeleton: a person's bones, built in code, `scale` times a person's size.
    Skeleton { scale: f32, tint: Option<[f32; 3]> },
    /// An animal or monster model with its own skeleton and animations (idle, walk).
    Beast { file: String, scale: f32, tint: Option<[f32; 3]> },
    Flat,
    Card,
}

/// A creature's body shape, made from a person's by resizing and bending its bones.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Build {
    Goblin,
    Troll,
    Ogre,
    Golem,
    Demon,
    Gnoll,
}

impl Build {
    fn parse(word: &str) -> Option<Build> {
        Some(match word {
            "goblin" => Build::Goblin,
            "troll" => Build::Troll,
            "ogre" => Build::Ogre,
            "golem" => Build::Golem,
            "demon" => Build::Demon,
            "gnoll" => Build::Gnoll,
            _ => return None,
        })
    }

    /// Each reshaped bone: its name, its scale (on the head: across, up, front; elsewhere
    /// the same every way, as a child bone takes its parent's scale and a stretched parent
    /// would shear it), and a bend forward in radians. A bone's scale carries on to the
    /// bones beyond it: a bigger collarbone makes the whole arm bigger.
    pub fn bones(self) -> Vec<(&'static str, [f32; 3], f32)> {
        let both = |name: &'static str, k: f32| -> [(&'static str, [f32; 3], f32); 2] {
            let side = |s: &str| -> &'static str { Box::leak(format!("{name}_{s}").into_boxed_str()) };
            [(side("l"), [k; 3], 0.0), (side("r"), [k; 3], 0.0)]
        };
        // (head, collarbone: the whole arm, forearm, hand, thigh: the whole leg, hunch)
        let (head, arm, forearm, hand, leg, hunch) = match self {
            Build::Goblin => ([1.4, 1.15, 1.3], 0.95, 1.15, 1.25, 0.8, 0.35),
            Build::Troll => ([1.25, 0.9, 1.3], 1.35, 1.1, 1.25, 0.88, 0.4),
            Build::Ogre => ([1.4, 1.1, 1.25], 1.25, 1.0, 1.2, 0.92, 0.25),
            Build::Golem => ([0.7, 0.65, 0.75], 1.45, 1.15, 1.4, 1.1, 0.0),
            Build::Demon => ([1.0, 1.25, 1.2], 1.2, 1.2, 1.3, 1.05, 0.1),
            Build::Gnoll => ([0.95, 1.0, 1.55], 1.05, 1.15, 1.2, 0.95, 0.4),
        };
        let mut out = vec![("Head", head, -hunch * 0.6), ("spine_03", [1.0; 3], hunch), ("neck_01", [1.0; 3], -hunch * 0.5)];
        out.extend(both("clavicle", arm));
        out.extend(both("lowerarm", forearm));
        out.extend(both("hand", hand));
        out.extend(both("thigh", leg));
        out
    }
}

/// Whether a floor picture is indoors (Some(true): dungeon stone, cave, boards, marble),
/// outdoors (Some(false): grass, roads, sand, water), or not a floor (None).
fn indoor_floor(face_name: &str) -> Option<bool> {
    let name = face_name.split('.').next().unwrap_or(face_name);
    let has = |words: &[&str]| words.iter().any(|w| name.contains(w));
    if has(&["flagstone", "dung_floor", "dun_floor", "cave", "woodfloor", "wood_floor", "dirtfloor", "marble", "checkered", "stonefloor", "tile"]) {
        Some(true)
    } else if has(&["grass", "cobblesto", "road", "sand", "desert", "water", "sea", "beach", "swamp", "snow", "dirt", "earth", "hills", "steppe", "brush", "plains"]) {
        Some(false)
    } else {
        None
    }
}

/// Indoors (a dungeon, cave or house: a ceiling, no sky) when most floors in view are
/// indoor floors. The server does not send a map's outdoor flag.
pub fn is_indoors<'a>(floors: impl IntoIterator<Item = &'a str>) -> bool {
    let (mut inside, mut out) = (0, 0);
    for f in floors {
        match indoor_floor(f) {
            Some(true) => inside += 1,
            Some(false) => out += 1,
            None => {}
        }
    }
    inside > out
}

#[derive(Default)]
pub struct ModelMap {
    exact: Vec<(String, Visual)>,
    prefixes: Vec<(String, Visual)>,
}

impl ModelMap {
    pub fn parse(text: &str) -> ModelMap {
        let mut map = ModelMap::default();
        for line in text.lines() {
            let line = line.split('#').next().unwrap_or("").trim();
            let words: Vec<&str> = line.split_whitespace().collect();
            let Some((&pattern, rest)) = words.split_first() else { continue };
            let Some(visual) = parse_visual(rest) else { continue };
            match pattern.strip_suffix('*') {
                Some(prefix) => map.prefixes.push((prefix.to_string(), visual)),
                None => map.exact.push((pattern.to_string(), visual)),
            }
        }
        map
    }

    /// `face_name` may carry the `.111` suffix. An exact full name ("grate_1.118", one
    /// frame) beats an exact base name, which beats the longest prefix.
    pub fn lookup(&self, face_name: &str) -> &Visual {
        let name = face_name.split('.').next().unwrap_or(face_name);
        for want in [face_name, name] {
            if let Some((_, v)) = self.exact.iter().find(|(p, _)| p == want) {
                return v;
            }
        }
        self.prefixes
            .iter()
            .filter(|(p, _)| name.starts_with(p.as_str()))
            .max_by_key(|(p, _)| p.len())
            .map_or(&Visual::Card, |(_, v)| v)
    }
}

/// An optional `r,g,b` word.
fn tint(words: &[&str]) -> Option<Option<[f32; 3]>> {
    match words {
        [] => Some(None),
        [t] => Some(Some(rgb(t)?)),
        _ => None,
    }
}

fn rgb(word: &str) -> Option<[f32; 3]> {
    let v: Vec<f32> = word.split(',').map(|c| c.parse().ok()).collect::<Option<_>>()?;
    <[f32; 3]>::try_from(v).ok()
}

fn parse_visual(words: &[&str]) -> Option<Visual> {
    Some(match *words {
        ["model", file, scale, rot, dy, ref rest @ ..] => Visual::Model {
            file: file.to_string(),
            scale: scale.parse().ok()?,
            rot_deg: rot.parse().ok()?,
            dy: dy.parse().ok()?,
            tint: tint(rest)?,
        },
        ["person", body, scale, skin, hair, clothes, ref rest @ ..] => {
            let (mut dress, mut broom, mut armour, mut wide, mut head, mut class) = (None, false, false, 1.0, 1.0, None);
            let (mut build, mut rags, mut horns, mut ears) = (None, None, false, false);
            let mut words = rest.iter();
            while let Some(&w) = words.next() {
                match w {
                    "dress" => dress = Some(rgb(words.next()?)?),
                    "broom" => broom = true,
                    "armour" => armour = true,
                    "wide" => wide = words.next()?.parse().ok()?,
                    "head" => head = words.next()?.parse().ok()?,
                    "class" => class = Some(class_look(words.next()?)?),
                    "build" => build = Some(Build::parse(words.next()?)?),
                    "rags" => rags = Some(rgb(words.next()?)?),
                    "horns" => horns = true,
                    "ears" => ears = true,
                    _ => return None,
                }
            }
            Visual::Person {
                body: match body {
                    "regular" => Body::Regular,
                    "other" => Body::Other,
                    _ => return None,
                },
                look: Look { scale: scale.parse().ok()?, skin: rgb(skin)?, hair: rgb(hair)?, clothes: rgb(clothes)? },
                dress,
                broom,
                armour,
                wide,
                head,
                class,
                build,
                rags,
                horns,
                ears,
            }
        }
        ["monster", file, scale, ref rest @ ..] => {
            Visual::Monster { file: file.to_string(), scale: scale.parse().ok()?, tint: tint(rest)? }
        }
        ["beast", file, scale, ref rest @ ..] => {
            Visual::Beast { file: file.to_string(), scale: scale.parse().ok()?, tint: tint(rest)? }
        }
        ["skeleton", scale, ref rest @ ..] => Visual::Skeleton { scale: scale.parse().ok()?, tint: tint(rest)? },
        ["statue", ref rest @ ..] => Visual::Statue { tint: tint(rest)? },
        ["building", recipe] => Visual::Building { recipe: recipe.to_string(), sign: None, icon: None },
        ["building", recipe, sign, ref icon @ ..] if icon.len() <= 1 => Visual::Building {
            recipe: recipe.to_string(),
            sign: Some(sign.replace('_', " ")),
            icon: icon.first().map(|i| i.to_string()),
        },
        ["wall", style] => Visual::Wall { style: style.to_string() },
        ["flat"] => Visual::Flat,
        ["card"] => Visual::Card,
        _ => return None,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Load {
    Loading,
    Loaded,
    Failed,
}

/// A thing made of several model files: failed if any failed, loaded when all are.
/// Its picture shows until then, and stays if a file failed.
pub fn all_loaded(states: impl IntoIterator<Item = Load>) -> Load {
    let mut out = Load::Loaded;
    for s in states {
        match s {
            Load::Failed => return Load::Failed,
            Load::Loading => out = Load::Loading,
            Load::Loaded => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT: &str = "\
# trees
tree* model nature/tree.glb 1.0 0 0
tree_1 model nature/oak.glb 1.2 90 0.1
tree_10* model nature/pine.glb 1.0 0 0
house_1 building small_house
stwall* wall stone
sign* model props/signpost.glb 0.8 0 0 0.6,0.4,0.2
pentagram flat
grate_1* model castle/metal-gate.glb 1.0 0 1.0
grate_1.118 model castle/metal-gate.glb 1.0 0 0
broken line
bad model x.glb notanumber 0 0
";

    #[test]
    fn building_sign_text() {
        let m = ModelMap::parse("trading-post building shop TRADING_POST\nstore_gems building shop JEWELS gem\n");
        let shop = |sign: &str, icon: Option<&str>| Visual::Building {
            recipe: "shop".into(),
            sign: Some(sign.into()),
            icon: icon.map(String::from),
        };
        assert_eq!(m.lookup("trading-post"), &shop("TRADING POST", None));
        assert_eq!(m.lookup("store_gems"), &shop("JEWELS", Some("gem")));
    }

    #[test]
    fn exact_beats_prefix() {
        let m = ModelMap::parse(TEXT);
        assert!(matches!(m.lookup("tree_1"), Visual::Model { file, .. } if file == "nature/oak.glb"));
    }

    #[test]
    fn longest_prefix_wins() {
        let m = ModelMap::parse(TEXT);
        assert!(matches!(m.lookup("tree_101"), Visual::Model { file, .. } if file == "nature/pine.glb"));
        assert!(matches!(m.lookup("tree_2"), Visual::Model { file, .. } if file == "nature/tree.glb"));
    }

    #[test]
    fn unknown_is_card() {
        assert_eq!(ModelMap::parse(TEXT).lookup("goblin.111"), &Visual::Card);
    }

    #[test]
    fn suffix_stripped() {
        let m = ModelMap::parse(TEXT);
        assert_eq!(m.lookup("house_1.111"), &Visual::Building { recipe: "small_house".into(), sign: None, icon: None });
        assert_eq!(m.lookup("stwall_A.111"), &Visual::Wall { style: "stone".into() });
        assert_eq!(m.lookup("pentagram.111"), &Visual::Flat);
    }

    #[test]
    fn full_name_with_frame_beats_base_name() {
        let m = ModelMap::parse(TEXT);
        assert!(matches!(m.lookup("grate_1.118"), Visual::Model { dy, .. } if *dy == 0.0));
        assert!(matches!(m.lookup("grate_1.111"), Visual::Model { dy, .. } if *dy == 1.0));
    }

    #[test]
    fn bad_lines_skipped() {
        let m = ModelMap::parse(TEXT);
        assert_eq!(m.lookup("broken"), &Visual::Card);
        assert_eq!(m.lookup("bad"), &Visual::Card);
    }

    #[test]
    fn model_fields_and_tint_parsed() {
        let m = ModelMap::parse(TEXT);
        assert_eq!(
            m.lookup("tree_1"),
            &Visual::Model { file: "nature/oak.glb".into(), scale: 1.2, rot_deg: 90.0, dy: 0.1, tint: None }
        );
        assert!(matches!(m.lookup("signpost_e"), Visual::Model { tint: Some(t), .. } if *t == [0.6, 0.4, 0.2]));
        let m = ModelMap::parse("statue statue\nstatue_gold statue 1,0.8,0.3\n");
        assert_eq!(m.lookup("statue.111"), &Visual::Statue { tint: None });
        assert_eq!(m.lookup("statue_gold"), &Visual::Statue { tint: Some([1.0, 0.8, 0.3]) });
    }

    #[test]
    fn beast_parsed() {
        let m = ModelMap::parse("wolf beast beasts/Wolf.gltf 0.6\nfox beast beasts/Fox.gltf 0.5 1,0.5,0.5\n");
        assert_eq!(m.lookup("wolf.131"), &Visual::Beast { file: "beasts/Wolf.gltf".into(), scale: 0.6, tint: None });
        assert_eq!(m.lookup("fox"), &Visual::Beast { file: "beasts/Fox.gltf".into(), scale: 0.5, tint: Some([1.0, 0.5, 0.5]) });
    }

    #[test]
    fn person_parsed() {
        let m = ModelMap::parse(
            "goblin person regular 0.7 0.3,0.5,0.2 0.1,0.1,0.1 0.4,0.3,0.2\n\
             maid person other 0.7 0.3,0.5,0.2 0.1,0.1,0.1 0.4,0.3,0.2 dress 0.6,0.2,0.2 broom\n\
             dwarf person regular 0.7 0.3,0.5,0.2 0.1,0.1,0.1 0.4,0.3,0.2 armour wide 1.3 head 1.2\n\
             bad person tall 1 1,1,1 1,1,1 1,1,1\n\
             bad2 person other 1 1,1,1 1,1,1 1,1,1 dress\n",
        );
        let look = Look { scale: 0.7, skin: [0.3, 0.5, 0.2], hair: [0.1, 0.1, 0.1], clothes: [0.4, 0.3, 0.2] };
        let person = |body, dress, broom, armour, wide, head| Visual::Person { body, look, dress, broom, armour, wide, head, class: None, build: None, rags: None, horns: false, ears: false };
        assert_eq!(m.lookup("goblin.131"), &person(Body::Regular, None, false, false, 1.0, 1.0));
        assert_eq!(m.lookup("maid"), &person(Body::Other, Some([0.6, 0.2, 0.2]), true, false, 1.0, 1.0));
        assert_eq!(m.lookup("dwarf"), &person(Body::Regular, None, false, true, 1.3, 1.2));
        assert_eq!(m.lookup("bad"), &Visual::Card);
        assert_eq!(m.lookup("bad2"), &Visual::Card);
    }

    #[test]
    fn monster_parsed() {
        let m = ModelMap::parse("kobold monster monsters/Puglin.gltf 0.9 1,0.6,0.4\nskeleton monster monsters/Skeleton_A.gltf 1\n");
        assert_eq!(
            m.lookup("kobold.151"),
            &Visual::Monster { file: "monsters/Puglin.gltf".into(), scale: 0.9, tint: Some([1.0, 0.6, 0.4]) }
        );
        assert!(matches!(m.lookup("skeleton"), Visual::Monster { tint: None, .. }));
    }

    #[test]
    fn every_model_file_in_the_list_exists() {
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets");
        let text = std::fs::read_to_string(format!("{root}/models.txt")).unwrap();
        let missing: Vec<String> = text
            .lines()
            .map(|l| l.split('#').next().unwrap_or("").split_whitespace().collect::<Vec<_>>())
            .filter(|w| matches!(w.get(1), Some(&("model" | "beast" | "monster"))))
            .filter_map(|w| w.get(2).map(|f| f.to_string()))
            .filter(|f| !std::path::Path::new(&format!("{root}/models/{f}")).exists())
            .collect();
        assert!(missing.is_empty(), "drawn as flat pictures: {missing:?}");
    }

    #[test]
    fn the_model_list_uses_no_paid_models() {
        let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/models.txt")).unwrap();
        assert!(!text.contains("monsters/"), "the paid monster pack may not be shared");
    }

    #[test]
    fn indoors_by_the_floor() {
        assert!(is_indoors(["flagstone.111", "flagstone.111", "dung_floor.111", "grass.111"]));
        assert!(!is_indoors(["grass.111", "cobblesto1.111", "flagstone.111"]));
        assert!(is_indoors(["cave1.111", "woodfloor.111"]));
        assert!(is_indoors(["white-gray-x-marble_111.111", "dirtfloor.111"]));
        assert!(!is_indoors(Vec::<&str>::new()), "nothing seen yet: outdoors");
        assert!(!is_indoors(["blank.111", "torch.111"]), "only floors count");
    }

    #[test]
    fn creature_builds_parsed() {
        let m = ModelMap::parse("gob person regular 0.7 1,1,1 1,1,1 1,1,1 build goblin rags 0.4,0.3,0.2 ears\ndev person regular 1 1,1,1 1,1,1 1,1,1 build demon horns\n");
        let Visual::Person { build, rags, horns, ears, .. } = m.lookup("gob") else { panic!() };
        assert_eq!((*build, *rags, *horns, *ears), (Some(Build::Goblin), Some([0.4, 0.3, 0.2]), false, true));
        let Visual::Person { build, horns, .. } = m.lookup("dev") else { panic!() };
        assert_eq!((*build, *horns), (Some(Build::Demon), true));
    }

    #[test]
    fn builds_are_not_human_shaped() {
        for build in [Build::Goblin, Build::Troll, Build::Ogre, Build::Golem, Build::Demon, Build::Gnoll] {
            let head = build.bones().iter().find(|b| b.0 == "Head").expect("every build reshapes the head").1;
            assert!(head.iter().any(|v| (v - 1.0).abs() > 0.15), "{build:?}: a head of a different shape");
            assert!(build.bones().iter().any(|b| b.0 != "Head" && b.1.iter().any(|v| (v - 1.0).abs() > 0.15)), "{build:?}: other limbs too");
        }
    }

    #[test]
    fn monsters_once_in_the_paid_pack_are_people_or_skeletons() {
        let m = ModelMap::parse(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/models.txt")).unwrap());
        for name in ["goblin", "kobold", "troll", "ogre", "golem", "gargoyle", "devil", "big_demon", "imp", "gnoll"] {
            assert!(matches!(m.lookup(name), Visual::Person { .. }), "{name}: {:?}", m.lookup(name));
        }
        for name in ["skeleton", "skeleton_chief", "lich", "wight", "skeletalmage"] {
            assert!(matches!(m.lookup(name), Visual::Skeleton { .. }), "{name}");
        }
    }

    #[test]
    fn spellcasting_dragons_and_fire_trolls_stay_3d() {
        let m = ModelMap::parse(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/models.txt")).unwrap());
        for (cast, base) in [("dragon_spellcasting", "dragon"), ("elec_dr_spellcasting", "elec_dr"), ("chinese_dr_spellcasting", "chinese_dr")] {
            assert_eq!(m.lookup(&format!("{cast}.151")), m.lookup(base), "{cast}");
        }
        assert!(matches!(m.lookup("fire_troll.111"), Visual::Person { build: Some(Build::Troll), .. }));
    }

    #[test]
    fn players_turned_to_stone_are_statues() {
        let m = ModelMap::parse(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/models.txt")).unwrap());
        for race in ["dragon", "dwarf", "elf", "gnome", "half_orc", "halfling", "human", "troll"] {
            assert!(matches!(m.lookup(&format!("{race}_stoned.111")), Visual::Statue { .. }), "{race}");
        }
    }

    #[test]
    fn skeleton_parsed() {
        let m = ModelMap::parse("skeleton skeleton 1.0\nlich skeleton 1.05 0.75,0.6,0.95\n");
        assert_eq!(m.lookup("skeleton.151"), &Visual::Skeleton { scale: 1.0, tint: None });
        assert_eq!(m.lookup("lich"), &Visual::Skeleton { scale: 1.05, tint: Some([0.75, 0.6, 0.95]) });
    }

    #[test]
    fn several_files_load_as_one() {
        use Load::*;
        assert_eq!(all_loaded([Loaded, Loaded]), Loaded);
        assert_eq!(all_loaded([Loaded, Loading]), Loading);
        assert_eq!(all_loaded([Loading, Failed]), Failed);
        assert_eq!(all_loaded([]), Loaded);
    }

    #[test]
    fn real_mapping_file_parses_if_present() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/models.txt");
        let Ok(text) = std::fs::read_to_string(&path) else { return };
        let m = ModelMap::parse(&text);
        // Every line must parse: a bad line is skipped silently and its picture stays a card.
        let lines = text.lines().filter(|l| !l.split('#').next().unwrap_or("").trim().is_empty()).count();
        assert_eq!(m.exact.len() + m.prefixes.len(), lines);
    }
}
