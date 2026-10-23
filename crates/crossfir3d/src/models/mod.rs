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
    /// Reads a build name from models.txt.
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
    /// Reads models.txt into exact and prefix picture-to-model rules.
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
    match *words {
        ["model", ref rest @ ..] => parse_model(rest),
        ["person", ref rest @ ..] => parse_person(rest),
        ["monster" | "beast", ..] => parse_creature(words),
        ["building", ref rest @ ..] => parse_building(rest),
        _ => parse_plain(words),
    }
}

/// `file scale rot dy [r,g,b]`.
fn parse_model(words: &[&str]) -> Option<Visual> {
    let [file, scale, rot, dy, rest @ ..] = words else { return None };
    Some(Visual::Model {
        file: file.to_string(),
        scale: scale.parse().ok()?,
        rot_deg: rot.parse().ok()?,
        dy: dy.parse().ok()?,
        tint: tint(rest)?,
    })
}

/// `monster file scale [tint]` or `beast file scale [tint]`.
fn parse_creature(words: &[&str]) -> Option<Visual> {
    Some(match *words {
        ["monster", file, scale, ref rest @ ..] => {
            Visual::Monster { file: file.to_string(), scale: scale.parse().ok()?, tint: tint(rest)? }
        }
        ["beast", file, scale, ref rest @ ..] => {
            Visual::Beast { file: file.to_string(), scale: scale.parse().ok()?, tint: tint(rest)? }
        }
        _ => return None,
    })
}

/// The visuals with few or no words: skeleton, statue, wall, flat, card.
fn parse_plain(words: &[&str]) -> Option<Visual> {
    Some(match *words {
        ["skeleton", scale, ref rest @ ..] => Visual::Skeleton { scale: scale.parse().ok()?, tint: tint(rest)? },
        ["statue", ref rest @ ..] => Visual::Statue { tint: tint(rest)? },
        ["wall", style] => Visual::Wall { style: style.to_string() },
        ["flat"] => Visual::Flat,
        ["card"] => Visual::Card,
        _ => return None,
    })
}

/// `recipe [sign [icon]]`; `_` in the sign becomes a space.
fn parse_building(words: &[&str]) -> Option<Visual> {
    Some(match *words {
        [recipe] => Visual::Building { recipe: recipe.to_string(), sign: None, icon: None },
        [recipe, sign, ref icon @ ..] if icon.len() <= 1 => Visual::Building {
            recipe: recipe.to_string(),
            sign: Some(sign.replace('_', " ")),
            icon: icon.first().map(|i| i.to_string()),
        },
        _ => return None,
    })
}

/// A person's body word: `regular` or `other`.
fn parse_body(word: &str) -> Option<Body> {
    match word {
        "regular" => Some(Body::Regular),
        "other" => Some(Body::Other),
        _ => None,
    }
}

/// `body scale skin hair clothes` then option words, each applied to the default person.
fn parse_person(words: &[&str]) -> Option<Visual> {
    let [body, scale, skin, hair, clothes, rest @ ..] = words else { return None };
    let mut person = Visual::Person {
        body: parse_body(body)?,
        look: Look { scale: scale.parse().ok()?, skin: rgb(skin)?, hair: rgb(hair)?, clothes: rgb(clothes)? },
        dress: None,
        broom: false,
        armour: false,
        wide: 1.0,
        head: 1.0,
        class: None,
        build: None,
        rags: None,
        horns: false,
        ears: false,
    };
    let mut rest = rest.iter();
    while let Some(&w) = rest.next() {
        if !person_flag(&mut person, w) {
            person_value(&mut person, w, rest.next()?)?;
        }
    }
    Some(person)
}

/// Sets the on/off option `w` on a person; false if `w` is not one.
fn person_flag(person: &mut Visual, w: &str) -> bool {
    let Visual::Person { broom, armour, horns, ears, .. } = person else { return false };
    match w {
        "broom" => *broom = true,
        "armour" => *armour = true,
        "horns" => *horns = true,
        "ears" => *ears = true,
        _ => return false,
    }
    true
}

/// Sets the option `w` that takes the word `v` on a person; None if `w` or `v` is bad.
fn person_value(person: &mut Visual, w: &str, v: &str) -> Option<()> {
    match w {
        "wide" | "head" => person_size(person, w, v),
        "dress" | "rags" => person_colour(person, w, v),
        "class" | "build" => person_kind(person, w, v),
        _ => None,
    }
}

/// `wide` or `head`: a number.
fn person_size(person: &mut Visual, w: &str, v: &str) -> Option<()> {
    let Visual::Person { wide, head, .. } = person else { return None };
    *if w == "wide" { wide } else { head } = v.parse().ok()?;
    Some(())
}

/// `dress` or `rags`: an `r,g,b` word.
fn person_colour(person: &mut Visual, w: &str, v: &str) -> Option<()> {
    let Visual::Person { dress, rags, .. } = person else { return None };
    *if w == "dress" { dress } else { rags } = Some(rgb(v)?);
    Some(())
}

/// `class` or `build`: a name.
fn person_kind(person: &mut Visual, w: &str, v: &str) -> Option<()> {
    let Visual::Person { class, build, .. } = person else { return None };
    if w == "class" {
        *class = Some(class_look(v)?);
    } else {
        *build = Some(Build::parse(v)?);
    }
    Some(())
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
mod tests;
