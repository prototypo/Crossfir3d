//! What the player's character holds: a model for each applied weapon, bow, or shield,
//! picked by the item's client type (the server's item kind) and name. The models are
//! Quaternius' Medieval Weapons Pack, under `assets/models/weapons/`. Also what it
//! wears: an outfit and its colour for applied body armour.

use cfproto::Item;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hand {
    Right,
    Left,
}

/// The model for an applied item, or None (armour, tools, crossbows: no model yet).
pub fn held_model(kind: u16, name: &str) -> Option<(&'static str, Hand)> {
    let name = name.to_ascii_lowercase();
    let has = |words: &[&str]| words.iter().any(|w| name.contains(w));
    let right = |file| Some((file, Hand::Right));
    match kind {
        // Artifacts: mostly swords.
        100 => right("weapons/Sword_Golden.gltf"),
        101 if has(&["dagger", "knife"]) => right("weapons/Dagger.gltf"),
        101 if has(&["claymore", "great", "two-hand", "bastard"]) => right("weapons/Claymore.gltf"),
        101 => right("weapons/Sword.gltf"),
        106 if has(&["battle", "double", "great"]) => right("weapons/Axe_Double.gltf"),
        106 => right("weapons/Axe.gltf"),
        // Clubs, hammers, maces, and flails.
        121 | 126 | 129 | 141 => right("weapons/Hammer_Small.gltf"),
        136 if has(&["scythe"]) => right("weapons/Scythe.gltf"),
        136 => right("weapons/Spear.gltf"),
        150 | 151 => Some(("weapons/Bow_Wooden.gltf", Hand::Left)),
        260 | 261 if has(&["kite", "high", "tower", "large"]) => Some(("weapons/Shield_Heater.gltf", Hand::Left)),
        260 | 261 => Some(("weapons/Shield_Round.gltf", Hand::Left)),
        _ => None,
    }
}

/// The models for the applied items among `carried`, at most one per hand.
pub fn held(carried: &[&Item]) -> Vec<(&'static str, Hand)> {
    let mut out: Vec<(&'static str, Hand)> = Vec::new();
    for item in carried.iter().filter(|i| i.applied()) {
        if let Some((file, hand)) = held_model(item.kind, &item.name)
            && !out.iter().any(|(_, h)| *h == hand)
        {
            out.push((file, hand));
        }
    }
    out
}

/// The free outfits: peasant clothes, a ranger's leathers, and the ranger outfit in grey
/// (for metal, tinted to its colour).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Garb {
    Peasant,
    Ranger,
    Metal,
    /// Peasant clothes under a long robe and sash (built, not a model file).
    Robe,
}

/// The outfit and tint for applied body armour (client types 250-257), or None: the
/// character's own clothes. Metal armour is the grey ranger outfit tinted like its metal
/// (above 1 brightens the grey); leather is the ranger outfit as made; robes, dresses,
/// and aprons are peasant clothes in their colour.
pub fn worn(carried: &[&Item]) -> Option<(Garb, [f32; 3])> {
    let item = carried.iter().find(|i| i.applied() && (250..=257).contains(&i.kind))?;
    let name = item.name.to_ascii_lowercase();
    let has = |w: &str| name.contains(w);
    let metal = |tint| Some((Garb::Metal, tint));
    let peasant = |tint| Some((Garb::Peasant, tint));
    match item.kind {
        _ if has("robe") => peasant(if has("midnight") { [0.2, 0.2, 0.45] } else { [0.4, 0.3, 0.65] }),
        250 => metal([1.6, 1.65, 1.8]),
        251 if has("black") => metal([0.5, 0.5, 0.55]),
        251 if has("blue") => metal([0.6, 0.8, 1.7]),
        251 if has("green") => metal([0.6, 1.4, 0.6]),
        251 => metal([1.5, 0.55, 0.45]),
        252 if has("black") => metal([0.55, 0.55, 0.6]),
        252 => metal([1.35, 1.38, 1.45]),
        253 => metal([1.1, 1.12, 1.18]),
        254 => Some((Garb::Ranger, [1.0, 1.0, 1.0])),
        255 => peasant([0.8, 0.5, 0.65]),
        257 => peasant([0.9, 0.9, 0.85]),
        _ => peasant([0.6, 0.5, 0.35]),
    }
}

/// The colour of the applied helmet or crown (client type 271): crowns gold, others steel.
pub fn helmet(carried: &[&Item]) -> Option<[f32; 3]> {
    let item = carried.iter().find(|i| i.applied() && i.kind == 271)?;
    Some(if item.name.to_ascii_lowercase().contains("crown") { [0.95, 0.75, 0.25] } else { [0.6, 0.62, 0.66] })
}

/// What a class wears on its head when nothing else is there.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Headwear {
    /// The ranger's hood, worn in place of the hair, in the clothes' colour.
    Hood,
    PointedHat([f32; 3]),
    /// A steel helmet with a nose guard.
    Helm,
    /// A gold band with a gem.
    Circlet,
    /// The priest's tall white mitre.
    Mitre,
    /// The sorcerer's tall, stiff, starred hat.
    TallHat([f32; 3]),
    /// A soft, wide hat for the road.
    SlouchHat([f32; 3]),
    /// A leather cap with horns.
    HornedCap,
    /// Goggles pushed up on the forehead.
    Goggles,
    /// A hood, and eyes glowing in this colour.
    GlowEyes([f32; 3]),
    /// Three-cornered, its brim turned up.
    Tricorn([f32; 3]),
    /// A black hood, and a cloth over the lower face.
    Mask,
    /// No hair.
    Bald,
    /// A creature's horns on a bare head.
    Horns,
    /// A creature's pointed ears, in its skin's colour.
    Ears([f32; 3]),
}

/// The top of a staff; the colour is what glows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StaffTop {
    Plain,
    /// An orb held in three wooden prongs.
    Cradle([f32; 3]),
    /// A flame in a brass cup.
    Flame([f32; 3]),
    /// A pointed crystal in claws.
    Crystal([f32; 3]),
    /// An orb in a brass hoop.
    Hoop([f32; 3]),
    /// An orb between two horns.
    Horns([f32; 3]),
    /// A bright orb with gold rays all round.
    Sun([f32; 3]),
}

/// What a class carries in a hand when that hand holds nothing else.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Carry {
    /// A model file under assets/models (a weapon, shield or bow).
    Model(&'static str),
    /// A wooden staff with this top.
    Staff(StaffTop),
    /// A small potion bottle.
    Potion,
}

/// What a class wears on its back.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Back {
    /// A cloth cape from the shoulders to the knees, in this colour.
    Cape([f32; 3]),
    /// A short fur cloak over the shoulders.
    Fur,
    /// A quiver of arrows, their ends over the right shoulder.
    Quiver,
    /// A full sack of loot.
    Sack,
    /// A sword in its sheath, its hilt over the right shoulder.
    Scabbard,
}

/// What a class wears over its clothes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Over {
    /// A leather apron from the chest to the knees.
    Apron,
    /// A cloth panel front and back, from the shoulders to the knees, in this colour.
    Tabard([f32; 3]),
    /// A wide cloth band round the waist, its ends hanging at the left hip, in this colour.
    Sash([f32; 3]),
}

/// A class's own look: its outfit and the outfit's tint (multipliers for the model's own
/// colours, as from `worn`), its headwear, and what it carries in each hand.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClassLook {
    pub garb: Garb,
    pub tint: [f32; 3],
    pub head: Option<Headwear>,
    pub right: Option<Carry>,
    pub left: Option<Carry>,
    pub back: Option<Back>,
    pub over: Option<Over>,
}

/// What a class wears on its back, by lowercase class name.
fn class_back(key: &str) -> Option<Back> {
    match key {
        "warrior" => Some(Back::Cape([0.5, 0.06, 0.05])),
        "paladin" => Some(Back::Cape([0.12, 0.2, 0.55])),
        "sorcerer" => Some(Back::Cape([0.22, 0.06, 0.3])),
        "warlock" => Some(Back::Cape([0.06, 0.05, 0.08])),
        "barbarian" => Some(Back::Fur),
        "ranger" => Some(Back::Quiver),
        "thief" => Some(Back::Sack),
        "ninja" => Some(Back::Scabbard),
        _ => None,
    }
}

/// What a class wears over its clothes, by lowercase class name.
fn class_over(key: &str) -> Option<Over> {
    match key {
        "alchemist" => Some(Over::Apron),
        "swashbuckler" => Some(Over::Sash([0.85, 0.62, 0.15])),
        "priest" => Some(Over::Tabard([0.9, 0.72, 0.3])),
        "summoner" => Some(Over::Tabard([0.2, 0.36, 0.22])),
        _ => None,
    }
}

type ClassRow = (&'static str, Garb, [f32; 3], Option<Headwear>, Option<Carry>, Option<Carry>);

const WHITE: [f32; 3] = [2.0, 2.2, 2.8];

/// Per class: name, garb, tint, headwear, right hand, left hand.
/// Peasant tints are over its brown cloth (1 keeps it); metal tints are over grey.
static CLASSES: [ClassRow; 16] = [
    ("warrior", Garb::Metal, [1.3, 1.32, 1.38], Some(Headwear::Helm), Some(Carry::Model("weapons/Sword.gltf")), Some(Carry::Model("weapons/Shield_Round.gltf"))),
    ("barbarian", Garb::Peasant, [1.0, 0.75, 0.65], Some(Headwear::HornedCap), Some(Carry::Model("weapons/Axe_Double.gltf")), None),
    ("paladin", Garb::Metal, [1.65, 1.68, 1.8], Some(Headwear::Circlet), Some(Carry::Model("weapons/Sword_Golden.gltf")), Some(Carry::Model("weapons/Shield_Heater.gltf"))),
    ("ranger", Garb::Ranger, [1.0; 3], Some(Headwear::Hood), None, Some(Carry::Model("weapons/Bow_Wooden.gltf"))),
    ("thief", Garb::Peasant, [0.45, 0.42, 0.42], Some(Headwear::Hood), Some(Carry::Model("weapons/Dagger.gltf")), None),
    ("ninja", Garb::Peasant, [0.15, 0.15, 0.17], Some(Headwear::Mask), Some(Carry::Model("weapons/Sword_2.gltf")), None),
    ("swashbuckler", Garb::Peasant, [1.4, 0.5, 0.65], Some(Headwear::Tricorn([0.32, 0.2, 0.12])), Some(Carry::Model("weapons/Sword.gltf")), None),
    ("monk", Garb::Robe, [1.9, 1.4, 0.45], Some(Headwear::Bald), None, None),
    ("priest", Garb::Peasant, WHITE, Some(Headwear::Mitre), Some(Carry::Model("weapons/Hammer_Small.gltf")), None),
    ("devotee", Garb::Peasant, WHITE, Some(Headwear::Hood), Some(Carry::Staff(StaffTop::Sun([1.0, 1.0, 0.95]))), None),
    ("wizard", Garb::Peasant, [0.55, 0.75, 2.2], Some(Headwear::PointedHat([0.45, 0.28, 0.15])), Some(Carry::Staff(StaffTop::Cradle([0.4, 0.6, 1.0]))), None),
    ("evoker", Garb::Peasant, [1.7, 0.75, 0.5], Some(Headwear::Hood), Some(Carry::Staff(StaffTop::Flame([1.0, 0.4, 0.1]))), None),
    ("sorcerer", Garb::Peasant, [1.0, 0.5, 1.8], Some(Headwear::TallHat([0.3, 0.1, 0.42])), Some(Carry::Staff(StaffTop::Crystal([0.8, 0.4, 1.0]))), None),
    ("summoner", Garb::Peasant, [1.7, 1.6, 0.6], Some(Headwear::SlouchHat([0.6, 0.45, 0.15])), Some(Carry::Staff(StaffTop::Hoop([1.0, 0.9, 0.3]))), None),
    ("warlock", Garb::Peasant, [0.55, 0.3, 0.9], Some(Headwear::GlowEyes([0.4, 1.0, 0.4])), Some(Carry::Staff(StaffTop::Horns([0.4, 1.0, 0.4]))), None),
    ("alchemist", Garb::Peasant, [1.1, 0.95, 0.75], Some(Headwear::Goggles), Some(Carry::Potion), None),
];

/// The look for a class, by arch ("wizard_class") or name ("Wizard"). Unknown: None.
pub fn class_look(class: &str) -> Option<ClassLook> {
    let key = class.to_ascii_lowercase();
    let key = key.strip_suffix("_class").unwrap_or(&key);
    let &(_, garb, tint, head, right, left) = CLASSES.iter().find(|row| row.0 == key)?;
    Some(ClassLook { garb, tint, head, right, left, back: class_back(key), over: class_over(key) })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(kind: u16, name: &str, applied: bool) -> Item {
        Item {
            tag: 1,
            location: 1,
            flags: if applied { 1 } else { 0 },
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

    #[test]
    fn weapons_by_kind_and_name() {
        assert_eq!(held_model(101, "short sword"), Some(("weapons/Sword.gltf", Hand::Right)));
        assert_eq!(held_model(101, "Dagger"), Some(("weapons/Dagger.gltf", Hand::Right)));
        assert_eq!(held_model(106, "battle axe"), Some(("weapons/Axe_Double.gltf", Hand::Right)));
        assert_eq!(held_model(136, "scythe"), Some(("weapons/Scythe.gltf", Hand::Right)));
        assert_eq!(held_model(151, "long bow"), Some(("weapons/Bow_Wooden.gltf", Hand::Left)));
        assert_eq!(held_model(261, "small shield"), Some(("weapons/Shield_Round.gltf", Hand::Left)));
        assert_eq!(held_model(145, "shovel"), None);
        assert_eq!(held_model(250, "leather armour"), None);
    }

    #[test]
    fn only_applied_items_one_per_hand() {
        let sword = item(101, "sword", true);
        let spare = item(106, "axe", false);
        let shield = item(261, "shield", true);
        let bow = item(151, "bow", true);
        assert_eq!(
            held(&[&spare, &sword, &shield, &bow]),
            [("weapons/Sword.gltf", Hand::Right), ("weapons/Shield_Round.gltf", Hand::Left)]
        );
        assert!(held(&[&spare]).is_empty());
    }

    #[test]
    fn worn_armour_picks_outfit_and_colour() {
        let plate = item(252, "plate mail", true);
        let spare = item(253, "chain mail", false);
        let robe = item(256, "robe", true);
        let sword = item(101, "sword", true);
        assert_eq!(worn(&[&spare, &sword, &plate]), Some((Garb::Metal, [1.35, 1.38, 1.45])));
        assert_eq!(worn(&[&item(254, "leather armour", true)]).map(|w| w.0), Some(Garb::Ranger));
        assert_eq!(worn(&[&robe]).map(|w| w.0), Some(Garb::Peasant));
        assert_eq!(worn(&[&item(251, "blue dragon mail", true)]), Some((Garb::Metal, [0.6, 0.8, 1.7])));
        assert_eq!(worn(&[&spare, &sword]), None);
    }

    #[test]
    fn helmet_colours() {
        assert_eq!(helmet(&[&item(271, "Crown", true)]), Some([0.95, 0.75, 0.25]));
        assert_eq!(helmet(&[&item(271, "full helmet", true)]), Some([0.6, 0.62, 0.66]));
        assert_eq!(helmet(&[&item(272, "turban", true)]), None);
    }

    #[test]
    fn every_held_model_exists() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/models");
        if !root.is_dir() {
            return;
        }
        let probes = [(100, ""), (101, "dagger"), (101, "claymore"), (101, ""), (106, "battle"), (106, ""), (121, ""), (136, "scythe"), (136, ""), (151, ""), (261, "kite"), (261, "")];
        for (kind, name) in probes {
            let (file, _) = held_model(kind, name).unwrap();
            assert!(root.join(file).is_file(), "missing {file}");
        }
    }

    #[test]
    fn plain_shirts_get_something_over_them() {
        let over = |c| class_look(c).unwrap().over;
        assert_eq!(over("alchemist"), Some(Over::Apron));
        assert!(matches!(over("swashbuckler"), Some(Over::Sash(_))));
        assert!(matches!(over("priest"), Some(Over::Tabard(_))));
        assert!(matches!(over("summoner"), Some(Over::Tabard(_))));
        assert_ne!(over("priest"), over("summoner"));
    }

    #[test]
    fn each_staff_class_has_its_own_staff_top() {
        let tops: Vec<_> = ["devotee", "wizard", "evoker", "sorcerer", "summoner", "warlock"]
            .iter()
            .map(|c| match class_look(c).unwrap().right {
                Some(Carry::Staff(top)) => std::mem::discriminant(&top),
                other => panic!("{c}: {other:?}"),
            })
            .collect();
        for (i, t) in tops.iter().enumerate() {
            assert!(!tops[..i].contains(t), "two classes share a staff top");
        }
    }

    #[test]
    fn classes_differ_from_behind() {
        let back = |c| class_look(c).unwrap().back;
        assert!(matches!(back("warrior"), Some(Back::Cape(_))));
        assert_ne!(back("warrior"), back("paladin"));
        assert_eq!(back("ranger"), Some(Back::Quiver));
        assert_eq!(back("thief"), Some(Back::Sack));
        assert_eq!(back("ninja"), Some(Back::Scabbard));
        assert_eq!(back("barbarian"), Some(Back::Fur));
    }

    #[test]
    fn class_looks_by_arch_or_name() {
        let wizard = class_look("wizard_class").unwrap();
        assert_eq!(class_look("Wizard"), Some(wizard));
        assert_eq!(wizard.head, Some(Headwear::PointedHat([0.45, 0.28, 0.15])));
        assert_eq!(class_look("paladin").unwrap().garb, Garb::Metal);
        assert_eq!(class_look("ranger").unwrap().left, Some(Carry::Model("weapons/Bow_Wooden.gltf")));
        assert_eq!(class_look("bard"), None);
    }

    #[test]
    fn thieves_wear_no_armour() {
        assert_eq!(class_look("thief").unwrap().garb, Garb::Peasant);
    }

    #[test]
    fn every_class_look_unchanged() {
        use Carry::{Model, Potion, Staff};
        use Headwear::*;
        let cape = |c| Some(Back::Cape(c));
        let white = [2.0, 2.2, 2.8];
        let want = [
            ("warrior", (Garb::Metal, [1.3, 1.32, 1.38], Some(Helm), Some(Model("weapons/Sword.gltf")), Some(Model("weapons/Shield_Round.gltf")), cape([0.5, 0.06, 0.05]), None)),
            ("barbarian", (Garb::Peasant, [1.0, 0.75, 0.65], Some(HornedCap), Some(Model("weapons/Axe_Double.gltf")), None, Some(Back::Fur), None)),
            ("paladin", (Garb::Metal, [1.65, 1.68, 1.8], Some(Circlet), Some(Model("weapons/Sword_Golden.gltf")), Some(Model("weapons/Shield_Heater.gltf")), cape([0.12, 0.2, 0.55]), None)),
            ("ranger", (Garb::Ranger, [1.0; 3], Some(Hood), None, Some(Model("weapons/Bow_Wooden.gltf")), Some(Back::Quiver), None)),
            ("thief", (Garb::Peasant, [0.45, 0.42, 0.42], Some(Hood), Some(Model("weapons/Dagger.gltf")), None, Some(Back::Sack), None)),
            ("ninja", (Garb::Peasant, [0.15, 0.15, 0.17], Some(Mask), Some(Model("weapons/Sword_2.gltf")), None, Some(Back::Scabbard), None)),
            ("swashbuckler", (Garb::Peasant, [1.4, 0.5, 0.65], Some(Tricorn([0.32, 0.2, 0.12])), Some(Model("weapons/Sword.gltf")), None, None, Some(Over::Sash([0.85, 0.62, 0.15])))),
            ("monk", (Garb::Robe, [1.9, 1.4, 0.45], Some(Bald), None, None, None, None)),
            ("priest", (Garb::Peasant, white, Some(Mitre), Some(Model("weapons/Hammer_Small.gltf")), None, None, Some(Over::Tabard([0.9, 0.72, 0.3])))),
            ("devotee", (Garb::Peasant, white, Some(Hood), Some(Staff(StaffTop::Sun([1.0, 1.0, 0.95]))), None, None, None)),
            ("wizard", (Garb::Peasant, [0.55, 0.75, 2.2], Some(PointedHat([0.45, 0.28, 0.15])), Some(Staff(StaffTop::Cradle([0.4, 0.6, 1.0]))), None, None, None)),
            ("evoker", (Garb::Peasant, [1.7, 0.75, 0.5], Some(Hood), Some(Staff(StaffTop::Flame([1.0, 0.4, 0.1]))), None, None, None)),
            ("sorcerer", (Garb::Peasant, [1.0, 0.5, 1.8], Some(TallHat([0.3, 0.1, 0.42])), Some(Staff(StaffTop::Crystal([0.8, 0.4, 1.0]))), None, cape([0.22, 0.06, 0.3]), None)),
            ("summoner", (Garb::Peasant, [1.7, 1.6, 0.6], Some(SlouchHat([0.6, 0.45, 0.15])), Some(Staff(StaffTop::Hoop([1.0, 0.9, 0.3]))), None, None, Some(Over::Tabard([0.2, 0.36, 0.22])))),
            ("warlock", (Garb::Peasant, [0.55, 0.3, 0.9], Some(GlowEyes([0.4, 1.0, 0.4])), Some(Staff(StaffTop::Horns([0.4, 1.0, 0.4]))), None, cape([0.06, 0.05, 0.08]), None)),
            ("alchemist", (Garb::Peasant, [1.1, 0.95, 0.75], Some(Goggles), Some(Potion), None, None, Some(Over::Apron))),
        ];
        for (name, look) in want {
            let cap = format!("{}{}", name[..1].to_uppercase(), &name[1..]);
            for form in [name.to_string(), cap, format!("{name}_class")] {
                assert_eq!(class_look(&form).map(|c| (c.garb, c.tint, c.head, c.right, c.left, c.back, c.over)), Some(look), "{form}");
            }
        }
    }
}
