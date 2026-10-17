//! What the player's character holds: a model for each applied weapon, bow, or shield,
//! picked by the item's client type (the server's item kind) and name. The models are
//! Quaternius' Medieval Weapons Pack, under `assets/models/weapons/`.

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
}
