//! Guesses 3D shape from Crossfire face names. The server sends no height.

/// Ground height in tiles for a floor face.
pub fn ground_height(face_name: &str) -> f32 {
    let base = base_name(face_name);
    // First match wins, so "deep_sea" is checked before "sea".
    const HEIGHTS: [(&str, f32); 6] = [
        ("deep_sea", -0.4),
        ("sea", -0.25),
        ("lake", -0.25),
        ("swamp", -0.1),
        ("mountain", 0.6),
        ("hills", 0.3),
    ];
    HEIGHTS.iter().find(|(word, _)| base.starts_with(word)).map_or(0.0, |(_, h)| *h)
}

/// True for faces that should be drawn as solid blocks.
pub fn is_wall(face_name: &str) -> bool {
    let base = base_name(face_name);
    // Spell walls, doors and wall clocks share the word "wall" but are not solid walls.
    const NOT_WALLS: [&str; 7] = ["fire", "lightning", "bul", "speedb", "cloud", "door", "clock"];
    base.contains("wall") && !NOT_WALLS.iter().any(|w| base.contains(w))
}

/// "stwall_3.111" -> "stwall_3"
fn base_name(face_name: &str) -> &str {
    face_name.split('.').next().unwrap_or(face_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heights_are_ordered() {
        assert!(ground_height("deep_sea.111") < ground_height("sea.111"));
        assert!(ground_height("sea.111") < ground_height("grass.111"));
        assert!(ground_height("grass.111") < ground_height("hills.111"));
        assert!(ground_height("hills.111") < ground_height("mountain1.111"));
        assert!(ground_height("swamp.111") < ground_height("grass.111"));
    }

    #[test]
    fn town_floor_is_flat() {
        assert_eq!(ground_height("cobblesto1.111"), 0.0);
        assert_eq!(ground_height("grass.111"), 0.0);
    }

    #[test]
    fn variants_follow_base_name() {
        assert_eq!(ground_height("sea_S.111"), ground_height("sea.111"));
        assert_eq!(ground_height("mountain2_snow.111"), ground_height("mountain1.111"));
        assert_eq!(ground_height("hillsrocky.111"), ground_height("hills.111"));
    }

    #[test]
    fn walls() {
        for w in ["wall_0.111", "stwall_3.111", "timberwall_A.111", "cwall_marble.111", "white_wall_left.111"] {
            assert!(is_wall(w), "{w}");
        }
        for n in ["grass.111", "firewall_1.111", "lightningwall.111", "stwall_door_0.111", "bul_wall_0.111", "wallclock.111"] {
            assert!(!is_wall(n), "{n}");
        }
    }
}
