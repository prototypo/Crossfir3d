//! map2 command decoding. See protocol.txt "map2" and MAP2_COORD_ENCODE in the server.

use crate::server::ParseError;

pub const FACE_IS_ANIM: u16 = 0x8000;
pub const ANIM_MASK: u16 = 0x1fff;

#[derive(Debug, Clone, PartialEq)]
pub enum Map2Item {
    Scroll { dx: i8, dy: i8 },
    Tile { x: i8, y: i8, ops: Vec<TileOp> },
}

#[derive(Debug, Clone, PartialEq)]
pub enum TileOp {
    Clear,
    Darkness(u8),
    Label,
    /// `layer` is 0..=9. `face` keeps the animation bits.
    Layer { layer: u8, face: u16, anim_speed: Option<u8>, smooth: Option<u8> },
}

const COORD_OFFSET: i16 = 15;

/// Parses a map2 packet into scrolls and tiles with their layer ops.
pub fn parse_map2(data: &[u8]) -> Result<Vec<Map2Item>, ParseError> {
    let short = || ParseError("map2 packet too short".into());
    let mut items = Vec::new();
    let mut i = 0;
    while i < data.len() {
        let v = u16::from_be_bytes([data[i], *data.get(i + 1).ok_or_else(short)?]);
        i += 2;
        let x = ((v >> 10) & 0x3f) as i16 - COORD_OFFSET;
        let y = ((v >> 4) & 0x3f) as i16 - COORD_OFFSET;
        let (x, y) = (x as i8, y as i8);
        match v & 0xf {
            1 => items.push(Map2Item::Scroll { dx: x, dy: y }),
            0 => items.push(Map2Item::Tile { x, y, ops: parse_tile_ops(data, &mut i)? }),
            kind => return Err(ParseError(format!("map2 coord kind {kind} unknown"))),
        }
    }
    Ok(items)
}

/// Reads the ops of one tile from `data[*i..]` up to the 255 terminator, advancing `i`.
fn parse_tile_ops(data: &[u8], i: &mut usize) -> Result<Vec<TileOp>, ParseError> {
    let short = || ParseError("map2 packet too short".into());
    let mut ops = Vec::new();
    loop {
        let t = *data.get(*i).ok_or_else(short)?;
        *i += 1;
        if t == 255 {
            break;
        }
        let mut len = (t >> 5) as usize;
        if len == 7 {
            len = *data.get(*i).ok_or_else(short)? as usize;
            *i += 1;
        }
        let d = data.get(*i..*i + len).ok_or_else(short)?;
        *i += len;
        ops.extend(tile_op(t, d));
    }
    Ok(ops)
}

/// Decodes one op of type `t & 0x1f` with payload `d`; None for reserved or malformed ops.
fn tile_op(t: u8, d: &[u8]) -> Option<TileOp> {
    match t & 0x1f {
        0x0 => Some(TileOp::Clear),
        0x1 if d.len() == 1 => Some(TileOp::Darkness(d[0])),
        0x2 => Some(TileOp::Label),
        ty @ 0x10..=0x19 => layer_op(ty, d),
        _ => None, // reserved or malformed: skipped by its length
    }
}

/// Builds a Layer op (face, optional anim_speed and smooth); None when the payload is under 2 bytes.
fn layer_op(ty: u8, d: &[u8]) -> Option<TileOp> {
    let [f0, f1, rest @ ..] = d else { return None };
    let face = u16::from_be_bytes([*f0, *f1]);
    let (anim_speed, smooth) = match (rest, face & FACE_IS_ANIM != 0) {
        ([s], true) => (Some(*s), None),
        ([s], false) => (None, Some(*s)),
        ([a, b, ..], _) => (Some(*a), Some(*b)),
        _ => (None, None),
    };
    Some(TileOp::Layer { layer: ty - 0x10, face, anim_speed, smooth })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn coord(x: i8, y: i8, kind: u16) -> [u8; 2] {
        let v = (((x as i16 + 15) as u16 & 0x3f) << 10) | (((y as i16 + 15) as u16 & 0x3f) << 4) | kind;
        v.to_be_bytes()
    }

    fn tile(x: i8, y: i8, rest: &[u8]) -> Vec<u8> {
        let mut b = coord(x, y, 0).to_vec();
        b.extend_from_slice(rest);
        b
    }

    fn ops(data: &[u8]) -> Vec<TileOp> {
        match parse_map2(data).unwrap().as_slice() {
            [Map2Item::Tile { ops, .. }] => ops.clone(),
            other => panic!("expected one tile, got {other:?}"),
        }
    }

    #[test]
    fn coord_decode() {
        assert_eq!(
            parse_map2(&tile(5, 7, &[0x21, 200, 255])).unwrap(),
            vec![Map2Item::Tile { x: 5, y: 7, ops: vec![TileOp::Darkness(200)] }]
        );
    }

    #[test]
    fn layer_face_only() {
        assert_eq!(
            ops(&tile(0, 0, &[0x50, 0x00, 0x2a, 255])),
            vec![TileOp::Layer { layer: 0, face: 42, anim_speed: None, smooth: None }]
        );
    }

    #[test]
    fn layer_anim_speed() {
        assert_eq!(
            ops(&tile(0, 0, &[0x76, 0x80, 0x05, 4, 255])),
            vec![TileOp::Layer { layer: 6, face: 0x8005, anim_speed: Some(4), smooth: None }]
        );
    }

    #[test]
    fn layer_smooth() {
        assert_eq!(
            ops(&tile(0, 0, &[0x70, 0x00, 0x09, 2, 255])),
            vec![TileOp::Layer { layer: 0, face: 9, anim_speed: None, smooth: Some(2) }]
        );
    }

    #[test]
    fn layer_full() {
        assert_eq!(
            ops(&tile(0, 0, &[0x99, 0x80, 0x05, 4, 2, 255])),
            vec![TileOp::Layer { layer: 9, face: 0x8005, anim_speed: Some(4), smooth: Some(2) }]
        );
    }

    #[test]
    fn negative_coord() {
        assert!(matches!(
            parse_map2(&tile(-1, -1, &[0x00, 255])).unwrap()[0],
            Map2Item::Tile { x: -1, y: -1, .. }
        ));
    }

    #[test]
    fn clear_op() {
        assert_eq!(ops(&tile(1, 1, &[0x00, 255])), vec![TileOp::Clear]);
    }

    #[test]
    fn scroll_item() {
        let mut b = coord(1, 0, 1).to_vec();
        b.extend(tile(2, 2, &[0x00, 255]));
        let items = parse_map2(&b).unwrap();
        assert_eq!(items[0], Map2Item::Scroll { dx: 1, dy: 0 });
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn label_skipped() {
        assert_eq!(ops(&tile(0, 0, &[0xE2, 3, 5, 1, b'x', 255])), vec![TileOp::Label]);
    }

    #[test]
    fn reserved_type_skipped() {
        assert_eq!(ops(&tile(0, 0, &[0x25, 9, 0x21, 50, 255])), vec![TileOp::Darkness(50)]);
    }

    #[test]
    fn bad_coord_kind() {
        assert!(parse_map2(&coord(0, 0, 3)).is_err());
    }

    fn old_parse_map2(data: &[u8]) -> Result<Vec<Map2Item>, ParseError> {
        let short = || ParseError("map2 packet too short".into());
        let mut items = Vec::new();
        let mut i = 0;
        while i < data.len() {
            let v = u16::from_be_bytes([data[i], *data.get(i + 1).ok_or_else(short)?]);
            i += 2;
            let x = ((v >> 10) & 0x3f) as i16 - COORD_OFFSET;
            let y = ((v >> 4) & 0x3f) as i16 - COORD_OFFSET;
            let (x, y) = (x as i8, y as i8);
            match v & 0xf {
                1 => items.push(Map2Item::Scroll { dx: x, dy: y }),
                0 => {
                    let mut ops = Vec::new();
                    loop {
                        let t = *data.get(i).ok_or_else(short)?;
                        i += 1;
                        if t == 255 {
                            break;
                        }
                        let mut len = (t >> 5) as usize;
                        if len == 7 {
                            len = *data.get(i).ok_or_else(short)? as usize;
                            i += 1;
                        }
                        let d = data.get(i..i + len).ok_or_else(short)?;
                        i += len;
                        match t & 0x1f {
                            0x0 => ops.push(TileOp::Clear),
                            0x1 if len == 1 => ops.push(TileOp::Darkness(d[0])),
                            0x2 => ops.push(TileOp::Label),
                            ty @ 0x10..=0x19 if len >= 2 => {
                                let face = u16::from_be_bytes([d[0], d[1]]);
                                let (anim_speed, smooth) = match (len, face & FACE_IS_ANIM != 0) {
                                    (3, true) => (Some(d[2]), None),
                                    (3, false) => (None, Some(d[2])),
                                    (4.., _) => (Some(d[2]), Some(d[3])),
                                    _ => (None, None),
                                };
                                ops.push(TileOp::Layer { layer: ty - 0x10, face, anim_speed, smooth });
                            }
                            _ => {} // reserved or malformed: skipped by its length
                        }
                    }
                    items.push(Map2Item::Tile { x, y, ops });
                }
                kind => return Err(ParseError(format!("map2 coord kind {kind} unknown"))),
            }
        }
        Ok(items)
    }

    fn same(data: &[u8]) {
        let (a, b) = (parse_map2(data), old_parse_map2(data));
        match (a, b) {
            (Ok(a), Ok(b)) => assert_eq!(a, b, "{data:?}"),
            (Err(a), Err(b)) => assert_eq!(a.0, b.0, "{data:?}"),
            (a, b) => panic!("{data:?}: {a:?} vs {b:?}"),
        }
    }

    #[test]
    fn matches_old_parse_map2() {
        let mut s: u64 = 0x2545_F491_4F6C_DD1D;
        let mut rnd = move |n: u64| {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (s >> 33) % n
        };
        for d in [&tile(5, 7, &[0x21, 200, 255])[..], &tile(0, 0, &[0x99, 0x80, 0x05, 4, 2, 255]), &tile(0, 0, &[0xE2, 3, 5, 1, b'x', 255]), &tile(0, 0, &[0x50, 0x00]), &coord(0, 0, 3), &[0x00], &[]] {
            same(d);
        }
        for _ in 0..20_000 {
            let n = rnd(64) as usize;
            let b: Vec<u8> = (0..n).map(|_| rnd(256) as u8).collect();
            same(&b);
        }
        let types = [0x0, 0x1, 0x2, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1f, 0x05];
        for _ in 0..20_000 {
            let mut b = Vec::new();
            for _ in 0..=rnd(3) {
                let kind = if rnd(20) == 0 { rnd(16) } else { rnd(2) };
                b.extend(coord(rnd(31) as i8 - 15, rnd(31) as i8 - 15, kind as u16));
                if kind != 0 {
                    continue;
                }
                for _ in 0..rnd(4) {
                    let mut len = rnd(8) as usize;
                    b.push(((len as u8) << 5) | types[rnd(types.len() as u64) as usize]);
                    if len == 7 {
                        len = rnd(8) as usize;
                        b.push(len as u8);
                    }
                    for k in 0..len {
                        b.push(if k == 0 && rnd(2) == 0 { 0x80 } else { rnd(256) as u8 });
                    }
                }
                if rnd(10) != 0 {
                    b.push(255);
                }
            }
            if rnd(5) == 0 {
                b.truncate(rnd(b.len() as u64 + 1) as usize);
            }
            same(&b);
        }
    }

    #[test]
    fn truncated() {
        assert!(parse_map2(&tile(0, 0, &[0x50, 0x00])).is_err());
        assert!(parse_map2(&tile(0, 0, &[0x21, 9])).is_err()); // no 255 terminator
        assert!(parse_map2(&[0x00]).is_err());
    }
}
