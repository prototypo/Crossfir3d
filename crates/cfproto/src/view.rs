//! What the client knows about the map around the player, in view coordinates.

use std::collections::HashMap;

use crate::map2::{Map2Item, TileOp};

/// Layers from here up hold creatures and flying things, which map memory leaves out.
const LIVING_LAYER: usize = 6;
/// The server also sends big pictures' corners this far past the view's right and bottom edges.
const HEAD_MARGIN: i32 = 8;

#[derive(Clone, PartialEq, Debug)]
pub struct Cell {
    pub layers: [u16; 10],
    pub darkness: u8,
}

impl Default for Cell {
    fn default() -> Self {
        Cell { layers: [0; 10], darkness: 255 }
    }
}

#[derive(Default)]
pub struct MapView {
    /// Total scroll so far. World coords = origin + view coords.
    pub origin: (i32, i32),
    cells: HashMap<(i8, i8), Cell>,
    /// Still things last seen on squares the player cannot see now, in world coords.
    memory: HashMap<(i32, i32), Cell>,
}

impl MapView {
    pub fn new() -> Self {
        Self::default()
    }

    /// Applies decoded map2 items to the view: scrolls the cells and updates tiles.
    pub fn apply(&mut self, items: &[Map2Item]) {
        for item in items {
            match item {
                Map2Item::Scroll { dx, dy } => self.scroll(*dx, *dy),
                Map2Item::Tile { x, y, ops } => {
                    for op in ops {
                        self.apply_op(*x, *y, op);
                    }
                }
            }
        }
    }

    /// Shifts every cell by (-dx, -dy), dropping cells that leave the i8 range, and moves the origin.
    fn scroll(&mut self, dx: i8, dy: i8) {
        self.cells = self
            .cells
            .drain()
            .filter_map(|((x, y), c)| Some(((x.checked_sub(dx)?, y.checked_sub(dy)?), c)))
            .collect();
        self.origin.0 += dx as i32;
        self.origin.1 += dy as i32;
    }

    /// Applies one tile operation to the cell at (x, y).
    fn apply_op(&mut self, x: i8, y: i8, op: &TileOp) {
        match op {
            // The server clears a square when the player can no longer see it.
            TileOp::Clear => self.remember(x, y),
            TileOp::Darkness(d) => self.cells.entry((x, y)).or_default().darkness = *d,
            TileOp::Label => {}
            TileOp::Layer { layer, face, .. } => {
                if *layer == 0 && *face != 0 {
                    self.memory.remove(&self.world(x, y));
                }
                let cell = self.cells.entry((x, y)).or_default();
                // TileOp fields are public, so an out-of-range layer is ignored rather than trusted.
                if let Some(l) = cell.layers.get_mut(*layer as usize) {
                    *l = *face;
                }
            }
        }
    }

    /// Moves the cell at (x, y), if any, into map memory.
    fn remember(&mut self, x: i8, y: i8) {
        let world = self.world(x, y);
        let Some(cell) = self.cells.remove(&(x, y)) else { return };
        let still = cell.layers.iter().take(LIVING_LAYER).enumerate().filter(|(_, f)| **f != 0);
        for (layer, face) in still {
            self.memory.entry(world).or_default().layers[layer] = *face;
        }
    }

    /// Moves cells that scrolled out of a view of `w` by `h` squares into map memory.
    /// The server sends nothing more about them, not even when they come back into view.
    pub fn remember_outside(&mut self, w: u8, h: u8) {
        let inside = |(x, y): (i8, i8)| (0..w as i32 + HEAD_MARGIN).contains(&(x as i32)) && (0..h as i32 + HEAD_MARGIN).contains(&(y as i32));
        let outside: Vec<_> = self.cells.keys().copied().filter(|k| !inside(*k)).collect();
        for (x, y) in outside {
            self.remember(x, y);
        }
    }

    /// Forgets the map: the view and the memory.
    pub fn clear(&mut self) {
        self.cells.clear();
        self.memory.clear();
    }

    /// What the player remembers, by world coords.
    pub fn remembered(&self) -> impl Iterator<Item = ((i32, i32), &Cell)> {
        self.memory.iter().map(|(k, v)| (*k, v))
    }

    pub fn cell(&self, x: i8, y: i8) -> Option<&Cell> {
        self.cells.get(&(x, y))
    }

    pub fn cells(&self) -> impl Iterator<Item = ((i8, i8), &Cell)> {
        self.cells.iter().map(|(k, v)| (*k, v))
    }

    pub fn world(&self, x: i8, y: i8) -> (i32, i32) {
        (self.origin.0 + x as i32, self.origin.1 + y as i32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layer(layer: u8, face: u16) -> TileOp {
        TileOp::Layer { layer, face, anim_speed: None, smooth: None }
    }

    fn view_with(x: i8, y: i8, ops: Vec<TileOp>) -> MapView {
        let mut v = MapView::new();
        v.apply(&[Map2Item::Tile { x, y, ops }]);
        v
    }

    #[test]
    fn view_sets_layers_and_darkness() {
        let v = view_with(5, 5, vec![layer(0, 7), layer(6, 9), TileOp::Darkness(100)]);
        let c = v.cell(5, 5).unwrap();
        assert_eq!((c.layers[0], c.layers[6], c.darkness), (7, 9, 100));
    }

    #[test]
    fn a_cleared_square_is_remembered_without_creatures_until_seen_again() {
        let mut v = view_with(1, 2, vec![layer(0, 7), layer(3, 8), layer(6, 9)]);
        v.apply(&[Map2Item::Scroll { dx: 1, dy: 1 }]);
        v.apply(&[Map2Item::Tile { x: 0, y: 1, ops: vec![TileOp::Clear] }]);
        assert!(v.cell(0, 1).is_none());
        let (at, cell) = v.remembered().next().unwrap();
        assert_eq!((at, cell.layers[0], cell.layers[3], cell.layers[6]), ((1, 2), 7, 8, 0));
        v.apply(&[Map2Item::Tile { x: 0, y: 1, ops: vec![layer(0, 7)] }]);
        assert_eq!(v.remembered().count(), 0);
    }

    #[test]
    fn a_big_picture_corner_alone_keeps_the_remembered_floor() {
        let mut v = view_with(1, 1, vec![layer(0, 7)]);
        v.apply(&[Map2Item::Tile { x: 1, y: 1, ops: vec![TileOp::Clear, layer(3, 8)] }]);
        assert_eq!(v.remembered().next().unwrap().1.layers[0], 7);
        v.apply(&[Map2Item::Tile { x: 1, y: 1, ops: vec![TileOp::Clear] }]);
        let cell = v.remembered().next().unwrap().1;
        assert_eq!((cell.layers[0], cell.layers[3]), (7, 8));
    }

    #[test]
    fn cells_past_the_view_and_its_margin_are_remembered() {
        let mut v = view_with(10, 10, vec![layer(0, 1)]);
        v.apply(&[Map2Item::Tile { x: 18, y: 0, ops: vec![layer(0, 2)] }]);
        v.apply(&[Map2Item::Tile { x: -1, y: 0, ops: vec![layer(0, 3)] }]);
        v.remember_outside(11, 11);
        assert!(v.cell(10, 10).is_some() && v.cell(18, 0).is_some());
        assert!(v.cell(-1, 0).is_none());
        assert_eq!(v.remembered().next().unwrap().0, (-1, 0));
    }

    #[test]
    fn a_new_map_forgets_memory() {
        let mut v = view_with(1, 1, vec![layer(0, 7)]);
        v.apply(&[Map2Item::Tile { x: 1, y: 1, ops: vec![TileOp::Clear] }]);
        v.clear();
        assert_eq!(v.remembered().count(), 0);
    }

    #[test]
    fn view_scroll_moves_cells() {
        let mut v = view_with(5, 5, vec![layer(0, 7)]);
        v.apply(&[Map2Item::Scroll { dx: 1, dy: 0 }]);
        assert!(v.cell(5, 5).is_none());
        assert_eq!(v.cell(4, 5).unwrap().layers[0], 7);
        assert_eq!(v.origin, (1, 0));
        assert_eq!(v.world(4, 5), (5, 5));
    }

    #[test]
    fn view_clear() {
        let mut v = view_with(1, 2, vec![layer(0, 7)]);
        v.apply(&[Map2Item::Tile { x: 1, y: 2, ops: vec![TileOp::Clear] }]);
        assert!(v.cell(1, 2).is_none());
    }

    #[test]
    fn clear_then_data_keeps_data() {
        let v = view_with(1, 2, vec![TileOp::Clear, layer(0, 3)]);
        assert_eq!(v.cell(1, 2).unwrap().layers[0], 3);
    }

    #[test]
    fn view_face_zero_clears_layer() {
        let mut v = view_with(0, 0, vec![layer(3, 7)]);
        v.apply(&[Map2Item::Tile { x: 0, y: 0, ops: vec![layer(3, 0)] }]);
        assert_eq!(v.cell(0, 0).unwrap().layers[3], 0);
    }

    #[test]
    fn newmap_clear_keeps_origin() {
        let mut v = view_with(0, 0, vec![layer(0, 1)]);
        v.apply(&[Map2Item::Scroll { dx: 2, dy: 3 }]);
        v.clear();
        assert_eq!(v.cells().count(), 0);
        assert_eq!(v.origin, (2, 3));
    }

    #[test]
    fn scroll_drops_underflow_and_every_op_applies() {
        let mut v = view_with(i8::MIN, 0, vec![layer(0, 1)]);
        v.apply(&[Map2Item::Tile { x: 5, y: 5, ops: vec![layer(2, 8), TileOp::Darkness(3)] }]);
        v.apply(&[Map2Item::Scroll { dx: 1, dy: -2 }]);
        assert!(v.cell(i8::MIN, 0).is_none() && v.cell(i8::MIN, 2).is_none());
        let c = v.cell(4, 7).unwrap().clone();
        assert_eq!((c.layers[2], c.darkness, v.origin, v.cells().count()), (8, 3, (1, -2), 1));
        v.apply(&[Map2Item::Tile { x: 4, y: 7, ops: vec![TileOp::Label] }]);
        assert_eq!(v.cell(4, 7), Some(&c));
        v.apply(&[Map2Item::Tile { x: 4, y: 7, ops: vec![TileOp::Clear] }]);
        assert!(v.cell(4, 7).is_none());
    }

    #[test]
    fn out_of_range_layer_is_ignored() {
        let v = view_with(1, 1, vec![layer(10, 5)]);
        assert_eq!(v.cell(1, 1).unwrap().layers, [0; 10]);
    }
}
