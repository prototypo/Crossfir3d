//! What the client knows about the map around the player, in view coordinates.

use std::collections::HashMap;

use crate::map2::{Map2Item, TileOp};

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
}

impl MapView {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply(&mut self, items: &[Map2Item]) {
        for item in items {
            match item {
                Map2Item::Scroll { dx, dy } => {
                    let (dx, dy) = (*dx, *dy);
                    self.cells = self
                        .cells
                        .drain()
                        .filter_map(|((x, y), c)| Some(((x.checked_sub(dx)?, y.checked_sub(dy)?), c)))
                        .collect();
                    self.origin.0 += dx as i32;
                    self.origin.1 += dy as i32;
                }
                Map2Item::Tile { x, y, ops } => {
                    for op in ops {
                        match op {
                            TileOp::Clear => {
                                self.cells.remove(&(*x, *y));
                            }
                            TileOp::Darkness(d) => self.cells.entry((*x, *y)).or_default().darkness = *d,
                            TileOp::Label => {}
                            TileOp::Layer { layer, face, .. } => {
                                self.cells.entry((*x, *y)).or_default().layers[*layer as usize] = *face;
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn clear(&mut self) {
        self.cells.clear();
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
}
