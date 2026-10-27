//! Wall pieces join up with neighbouring wall squares in the view.

use std::collections::{HashMap, HashSet};

pub const N: u8 = 1;
pub const E: u8 = 2;
pub const S: u8 = 4;
pub const W: u8 = 8;

pub fn join_mask(n: bool, e: bool, s: bool, w: bool) -> u8 {
    (n as u8) * N | (e as u8) * E | (s as u8) * S | (w as u8) * W
}

/// Rotates a side mask clockwise by `turns` quarter turns (N -> E -> S -> W).
fn rotate(mask: u8, turns: u32) -> u8 {
    ((mask << turns) | (mask >> ((4 - turns) % 4))) & 0xF
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Piece {
    /// No neighbours.
    Post,
    /// One neighbour; the piece runs from the centre toward it.
    End,
    Straight,
    Corner,
    Tee,
    Cross,
}

/// The piece and its yaw in degrees (clockwise seen from above). Unrotated pieces:
/// End points north, Straight runs north-south, Corner joins north and east, Tee
/// joins north, east, and south.
pub fn piece_for(mask: u8) -> (Piece, f32) {
    let mask = mask & 0xF;
    let (piece, base) = match mask.count_ones() {
        0 => (Piece::Post, 0),
        1 => (Piece::End, N),
        2 if mask == N | S || mask == E | W => (Piece::Straight, N | S),
        2 => (Piece::Corner, N | E),
        3 => (Piece::Tee, N | E | S),
        _ => (Piece::Cross, N | E | S | W),
    };
    let turns = (0..4).find(|&t| rotate(base, t) == mask).unwrap_or(0);
    (piece, turns as f32 * 90.0)
}

/// Join masks for every wall square; a neighbour not in the set (including outside
/// the view) counts as no wall.
pub fn wall_masks(walls: &HashSet<(i32, i32)>) -> HashMap<(i32, i32), u8> {
    // Map y grows southward, so north is y - 1.
    let has = |x: i32, y: i32| walls.contains(&(x, y));
    walls
        .iter()
        .map(|&(x, y)| ((x, y), join_mask(has(x, y - 1), has(x + 1, y), has(x, y + 1), has(x - 1, y))))
        .collect()
}

/// Lights that maps put on wall squares (a brazier or torch drawn over the wall in the
/// old top-down view): in 3D they hang on the wall's open side instead of inside it.
pub fn hangs_on_walls(face_name: &str) -> bool {
    let base = face_name.split('.').next().unwrap_or(face_name);
    ["brazier", "firepot", "torch", "lamppost", "candle", "chandelier", "wallclock"].iter().any(|w| base.contains(w))
}

/// The side of the wall square (x, y) open to walk on, as a step (south first, as the
/// top-down view shows it; then east, west, north). None off a wall or when walled in.
pub fn open_side(walls: &HashSet<(i32, i32)>, x: i32, y: i32) -> Option<(i32, i32)> {
    if !walls.contains(&(x, y)) {
        return None;
    }
    [(0, 1), (1, 0), (-1, 0), (0, -1)].into_iter().find(|(dx, dy)| !walls.contains(&(x + dx, y + dy)))
}

/// The side of floor square (x, y) with a wall next to it (north first: where a wall
/// clock hangs unturned); None when no wall touches it.
pub fn wall_beside(walls: &HashSet<(i32, i32)>, x: i32, y: i32) -> Option<(i32, i32)> {
    [(0, -1), (1, 0), (-1, 0), (0, 1)].into_iter().find(|(dx, dy)| walls.contains(&(x + dx, y + dy)))
}

/// The turn (radians about the up axis) that makes a thing built facing south face
/// the open side `(dx, dy)` of its wall.
pub fn facing((dx, dy): (i32, i32)) -> f32 {
    (dx as f32).atan2(dy as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_floor_square_finds_the_wall_beside_it() {
        let walls: HashSet<(i32, i32)> = [(5, 4), (6, 5)].into();
        assert_eq!(wall_beside(&walls, 5, 5), Some((0, -1)), "north wins");
        assert_eq!(wall_beside(&walls, 5, 6), None);
        assert_eq!(wall_beside(&walls, 7, 5), Some((-1, 0)));
        // Facing away from a north wall is the model's own way round.
        assert_eq!(facing((0, 1)), 0.0);
    }

    #[test]
    fn lights_on_a_wall_move_to_its_open_side() {
        let walls: HashSet<(i32, i32)> = [(5, 5), (6, 5), (5, 6)].into();
        // (5,5): south (5,6) and east (6,5) are walls; west is open.
        assert_eq!(open_side(&walls, 5, 5), Some((-1, 0)));
        assert_eq!(open_side(&walls, 6, 5), Some((0, 1))); // south first
        assert_eq!(open_side(&walls, 9, 9), None); // not on a wall
        let boxed: HashSet<(i32, i32)> = [(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)].into();
        assert_eq!(open_side(&boxed, 0, 0), None); // walled in on every side
        assert!(hangs_on_walls("brazier.111") && hangs_on_walls("torch_lit.112") && hangs_on_walls("firepot.113"));
        assert!(!hangs_on_walls("goldcoin.111") && !hangs_on_walls("club.111"));
        assert!(hangs_on_walls("wallclock.111"));
    }

    #[test]
    fn things_on_a_wall_face_its_open_side() {
        use std::f32::consts::FRAC_PI_2;
        // Built facing south (+y); turned about the up axis to face the open side.
        assert!(facing((0, 1)).abs() < 1e-6);
        assert!((facing((1, 0)) - FRAC_PI_2).abs() < 1e-6);
        assert!((facing((-1, 0)) + FRAC_PI_2).abs() < 1e-6);
    }

    /// Which sides a piece joins after rotation, to check `piece_for` against its mask.
    fn sides(piece: Piece, yaw: f32) -> u8 {
        let base = match piece {
            Piece::Post => 0,
            Piece::End => N,
            Piece::Straight => N | S,
            Piece::Corner => N | E,
            Piece::Tee => N | E | S,
            Piece::Cross => N | E | S | W,
        };
        let turns = ((yaw / 90.0).round() as i32).rem_euclid(4) as u32;
        // Rotate the 4-bit mask clockwise by `turns` quarter turns (N->E->S->W).
        ((base << turns) | (base >> (4 - turns))) & 0xF
    }

    #[test]
    fn all_16_masks_have_a_piece() {
        for mask in 0..16u8 {
            let (piece, yaw) = piece_for(mask);
            assert_eq!(sides(piece, yaw), mask, "mask {mask}: {piece:?} at {yaw}");
        }
    }

    #[test]
    fn straight_runs_east_west_and_north_south() {
        assert_eq!(piece_for(N | S), (Piece::Straight, 0.0));
        assert_eq!(piece_for(E | W).0, Piece::Straight);
        assert_eq!(piece_for(E | W).1.rem_euclid(180.0), 90.0);
    }

    #[test]
    fn corner_rotations() {
        assert_eq!(piece_for(N | E), (Piece::Corner, 0.0));
        assert_eq!(piece_for(E | S), (Piece::Corner, 90.0));
        assert_eq!(piece_for(S | W), (Piece::Corner, 180.0));
        assert_eq!(piece_for(W | N), (Piece::Corner, 270.0));
    }

    #[test]
    fn edge_of_view_is_no_wall() {
        let walls: HashSet<(i32, i32)> = [(0, 0), (1, 0)].into();
        let m = wall_masks(&walls);
        assert_eq!(m[&(0, 0)], E);
        assert_eq!(m[&(1, 0)], W);
    }

    #[test]
    fn rejoins_when_neighbour_arrives() {
        let mut walls: HashSet<(i32, i32)> = [(5, 5)].into();
        assert_eq!(wall_masks(&walls)[&(5, 5)], 0);
        walls.insert((5, 4)); // north (y grows south)
        assert_eq!(wall_masks(&walls)[&(5, 5)], N);
        assert_eq!(join_mask(true, false, true, false), N | S);
    }
}
