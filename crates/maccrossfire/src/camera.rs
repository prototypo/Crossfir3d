//! Ocarina-style camera rules: camera-relative directions, swinging behind the
//! player, and pulling in when something solid is in the way.

/// 45° steps, 0..8: 0 = camera looking north, 2 = east, 4 = south, 6 = west.
pub type Heading = u8;

pub const SWING_SECS: f32 = 1.0;
pub const MIN_DISTANCE: f32 = 1.5;

const COMPASS: [&str; 8] = ["north", "northeast", "east", "southeast", "south", "southwest", "west", "northwest"];

/// Screen direction from held keys: x right, y down. Opposites cancel.
pub fn screen_dir(up: bool, down: bool, left: bool, right: bool) -> Option<(i8, i8)> {
    let d = (right as i8 - left as i8, down as i8 - up as i8);
    (d != (0, 0)).then_some(d)
}

/// Screen direction to 45° steps clockwise from "up".
fn screen_step(screen: (i8, i8)) -> u8 {
    match screen {
        (0, -1) => 0,
        (1, -1) => 1,
        (1, 0) => 2,
        (1, 1) => 3,
        (0, 1) => 4,
        (-1, 1) => 5,
        (-1, 0) => 6,
        _ => 7,
    }
}

/// The compass direction for a screen direction when the camera looks `heading`.
pub fn compass(screen: (i8, i8), heading: Heading) -> &'static str {
    COMPASS[((screen_step(screen) + heading) % 8) as usize]
}

/// Crossfire direction 1..8 (1 = north, clockwise) to a heading.
pub fn heading_of(dir: u8) -> Heading {
    (dir.clamp(1, 8) - 1) % 8
}

/// One 45° step toward behind the player, only when walking forward or diagonally
/// forward (away from the camera), like Ocarina.
pub fn swing(heading: Heading, facing: u8, screen: (i8, i8)) -> Heading {
    if screen.1 >= 0 {
        return heading;
    }
    let ahead = (heading_of(facing) + 8 - heading) % 8;
    match ahead {
        0 => heading,
        1..=4 => (heading + 1) % 8,
        _ => (heading + 7) % 8,
    }
}

/// The largest camera distance (≤ `distance`, in 0.25 steps) where `blocked` is
/// false, never below `MIN_DISTANCE`.
pub fn pull_in(distance: f32, blocked: impl Fn(f32) -> bool) -> f32 {
    let mut d = distance;
    while d > MIN_DISTANCE && blocked(d) {
        d -= 0.25;
    }
    d.max(MIN_DISTANCE)
}

/// True if a solid square lies on the ground line from the player (`from`) to the
/// camera (`to`), checked every quarter square. The player's own square never counts.
pub fn sight_blocked(from: (f32, f32), to: (f32, f32), solid: impl Fn(i32, i32) -> bool) -> bool {
    let square = |(x, y): (f32, f32)| (x.floor() as i32, y.floor() as i32);
    let own = square(from);
    let steps = ((to.0 - from.0).hypot(to.1 - from.1) / 0.25).ceil() as i32;
    (1..=steps).any(|i| {
        let t = i as f32 / steps as f32;
        let sq = square((from.0 + (to.0 - from.0) * t, from.1 + (to.1 - from.1) * t));
        sq != own && solid(sq.0, sq.1)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_blocked_by_any_solid_square_between() {
        let wall = |x: i32, y: i32| (x, y) == (0, 3);
        // From the player at (0.5, 0.5) back to a camera at (0.5, 5.5): the wall is between.
        assert!(sight_blocked((0.5, 0.5), (0.5, 5.5), wall));
        assert!(!sight_blocked((0.5, 0.5), (0.5, 2.5), wall));
        // The player's own square never blocks (the player may stand in a doorway).
        assert!(!sight_blocked((0.5, 3.5), (0.5, 3.9), wall));
    }

    const UP: (i8, i8) = (0, -1);
    const RIGHT: (i8, i8) = (1, 0);
    const LEFT: (i8, i8) = (-1, 0);
    const UP_LEFT: (i8, i8) = (-1, -1);
    const DOWN: (i8, i8) = (0, 1);

    #[test]
    fn screen_dirs_and_cancel() {
        assert_eq!(screen_dir(true, false, false, false), Some(UP));
        assert_eq!(screen_dir(false, true, true, false), Some((-1, 1)));
        assert_eq!(screen_dir(true, true, false, false), None);
        assert_eq!(screen_dir(true, true, true, false), Some(LEFT));
        assert_eq!(screen_dir(false, false, false, false), None);
    }

    #[test]
    fn compass_all_headings() {
        assert_eq!(compass(UP, 0), "north");
        assert_eq!(compass(UP, 2), "east");
        assert_eq!(compass(UP, 4), "south");
        assert_eq!(compass(RIGHT, 0), "east");
        assert_eq!(compass(RIGHT, 2), "south");
        assert_eq!(compass(UP_LEFT, 6), "southwest");
        assert_eq!(compass(LEFT, 4), "east");
        assert_eq!(compass(DOWN, 1), "southwest");
        // Every screen direction at every heading gives a valid, distinct-per-heading direction.
        let dirs = [(0, -1), (1, -1), (1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1)];
        for h in 0..8 {
            let mut seen: Vec<&str> = dirs.iter().map(|d| compass(*d, h)).collect();
            seen.sort();
            seen.dedup();
            assert_eq!(seen.len(), 8, "heading {h}");
        }
    }

    #[test]
    fn heading_of_round_trip() {
        for dir in 1..=8 {
            assert_eq!(compass(UP, heading_of(dir)), COMPASS[dir as usize - 1]);
        }
    }

    #[test]
    fn swing_only_when_walking_forward() {
        // Facing east (3) while the camera looks north (0).
        assert_eq!(swing(0, 3, RIGHT), 0); // sidestep: no swing
        assert_eq!(swing(0, 3, DOWN), 0); // walking back: no swing
        assert_eq!(swing(0, 2, (1, -1)), 1); // diagonal forward: one step toward northeast
    }

    #[test]
    fn swing_one_step_toward_facing() {
        assert_eq!(swing(0, 1, UP), 0); // already behind
        assert_eq!(swing(0, 8, UP), 7); // northwest: shortest way is back past 0
        assert_eq!(swing(7, 2, UP), 0); // facing northeast from northwest: wraps forward
        assert_eq!(swing(2, 5, UP), 3); // facing south from east
    }

    #[test]
    fn pull_in_finds_clear_spot() {
        assert_eq!(pull_in(6.0, |_| false), 6.0);
        assert_eq!(pull_in(6.0, |d| d > 4.0), 4.0);
        assert_eq!(pull_in(6.0, |d| d > 3.3), 3.25);
    }

    #[test]
    fn pull_in_never_below_min() {
        assert_eq!(pull_in(6.0, |_| true), MIN_DISTANCE);
        assert_eq!(pull_in(1.0, |_| false), MIN_DISTANCE);
    }
}
