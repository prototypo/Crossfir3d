//! Tests for the model table parser and lookup.

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
fn rare_things_are_3d() {
    let m = ModelMap::parse(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/models.txt")).unwrap());
    let names = ["campfire", "cannon_0", "cannon_3", "cannon_8", "logs1", "logs2", "light_bulb_1", "light_bulb_4", "helmet2", "helmet3", "quiver_arrow", "quiver_bolt", "card_crown", "card_sword", "wallclock"];
    for name in names {
        assert!(matches!(m.lookup(&format!("{name}.111")), Visual::Building { .. } | Visual::Model { .. }), "{name}: {:?}", m.lookup(name));
    }
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
