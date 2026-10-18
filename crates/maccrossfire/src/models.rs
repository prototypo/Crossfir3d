//! Which picture becomes which 3D visual: `assets/models.txt`.
//!
//! Lines: `pattern kind args...`; `#` starts a comment; a pattern ending in `*` is a
//! prefix. Kinds:
//! - `model <file> <scale> <rot_deg> <dy> [r,g,b]`
//! - `building <recipe> [SIGN_TEXT [icon]]` (`_` in the sign is a space; icons in signs.rs)
//! - `wall <style>`
//! - `flat` (a floor decoration lying on the ground)
//! - `card` (keep the picture card)

#[derive(Clone, Debug, PartialEq)]
pub enum Visual {
    Model { file: String, scale: f32, rot_deg: f32, dy: f32, tint: Option<[f32; 3]> },
    Building { recipe: String, sign: Option<String>, icon: Option<String> },
    Wall { style: String },
    /// A person in stone on a plinth: a half-size 3D character, maybe tinted (bronze).
    Statue { tint: Option<[f32; 3]> },
    Flat,
    Card,
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
        [t] => {
            let v: Vec<f32> = t.split(',').map(|c| c.parse().ok()).collect::<Option<_>>()?;
            Some(Some(<[f32; 3]>::try_from(v).ok()?))
        }
        _ => None,
    }
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
        assert!(!m.exact.is_empty() || !m.prefixes.is_empty());
    }
}
