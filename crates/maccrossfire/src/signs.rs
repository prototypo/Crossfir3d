//! Shop sign text in a 5×7 pixel font, like the pixel lettering on Crossfire's own
//! shop pictures, and an optional 7×7 colour icon after it. Pure, so it tests
//! without Bevy.

/// Rows top to bottom; bit 4 is the left column. A to Z; anything else is a space.
const GLYPHS: [[u8; 7]; 26] = [
    [0x0E, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11], // A
    [0x1E, 0x11, 0x11, 0x1E, 0x11, 0x11, 0x1E], // B
    [0x0E, 0x11, 0x10, 0x10, 0x10, 0x11, 0x0E], // C
    [0x1E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x1E], // D
    [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x1F], // E
    [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x10], // F
    [0x0E, 0x11, 0x10, 0x17, 0x11, 0x11, 0x0F], // G
    [0x11, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11], // H
    [0x0E, 0x04, 0x04, 0x04, 0x04, 0x04, 0x0E], // I
    [0x07, 0x02, 0x02, 0x02, 0x02, 0x12, 0x0C], // J
    [0x11, 0x12, 0x14, 0x18, 0x14, 0x12, 0x11], // K
    [0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1F], // L
    [0x11, 0x1B, 0x15, 0x15, 0x11, 0x11, 0x11], // M
    [0x11, 0x11, 0x19, 0x15, 0x13, 0x11, 0x11], // N
    [0x0E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E], // O
    [0x1E, 0x11, 0x11, 0x1E, 0x10, 0x10, 0x10], // P
    [0x0E, 0x11, 0x11, 0x11, 0x15, 0x12, 0x0D], // Q
    [0x1E, 0x11, 0x11, 0x1E, 0x14, 0x12, 0x11], // R
    [0x0F, 0x10, 0x10, 0x0E, 0x01, 0x01, 0x1E], // S
    [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04], // T
    [0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E], // U
    [0x11, 0x11, 0x11, 0x11, 0x11, 0x0A, 0x04], // V
    [0x11, 0x11, 0x11, 0x15, 0x15, 0x15, 0x0A], // W
    [0x11, 0x11, 0x0A, 0x04, 0x0A, 0x11, 0x11], // X
    [0x11, 0x11, 0x0A, 0x04, 0x04, 0x04, 0x04], // Y
    [0x1F, 0x01, 0x02, 0x04, 0x08, 0x10, 0x1F], // Z
];

/// Dark paint on the wooden board.
const INK: [u8; 4] = [30, 20, 12, 255];

/// Icon paints; `.` is see-through.
fn paint(c: char) -> Option<[u8; 4]> {
    Some(match c {
        'k' => INK,
        'c' => [80, 200, 230, 255],  // cyan
        'w' => [235, 245, 250, 255], // white
        'y' => [240, 200, 40, 255],  // yellow
        'o' => [190, 120, 20, 255],  // dark gold
        'b' => [80, 45, 20, 255],    // dark wood
        'g' => [150, 150, 160, 255], // steel
        'u' => [50, 90, 180, 255],   // blue
        'r' => [200, 40, 40, 255],   // red
        'G' => [50, 150, 50, 255],   // green
        'n' => [225, 200, 150, 255], // sacking
        _ => return None,
    })
}

/// 7×7 icons by name (`assets/models.txt`: `building shop JEWELS gem`).
fn icon(name: &str) -> Option<[&'static str; 7]> {
    Some(match name {
        "gem" => [".......", ".cwccw.", "ccccccc", "uuuuuuu", ".ccccc.", "..ccc..", "...c..."],
        "wand" => ["....y..", "...yyy.", "....y..", "...b...", "..b....", ".b.....", "b......"],
        "coin" => ["..yyy..", ".yoooy.", "yoyyyoy", "yoyyyoy", "yoyyyoy", ".yoooy.", "..yyy.."],
        "sword" => ["......g", ".....g.", "....g..", ".y.g...", "..y....", ".b.y...", "b......"],
        "shield" => ["ggggggg", "guuuuug", "guuuuug", "guuuuug", ".guuug.", "..gug..", "...g..."],
        "bow" => ["...b...", "..b.w..", ".b..w..", ".b..w..", ".b..w..", "..b.w..", "...b..."],
        "potion" => ["..bbb..", "..g.g..", "..g.g..", ".grrrg.", "grrwrrg", "grrrrrg", ".ggggg."],
        "apple" => ["...b...", "...bGG.", ".rrbrr.", "rrrrrrr", "rrrrrrr", ".rrrrr.", "..r.r.."],
        "hammer" => [".ggggg.", ".ggggg.", "...b...", "...b...", "...b...", "...b...", "...b..."],
        "sack" => ["..b.b..", "...b...", "..nnn..", ".nnnnn.", "nnnnnnn", "nnnnnnn", ".nnnnn."],
        _ => return None,
    })
}

/// The text, then the named icon if there is one, as RGBA pixels (see-through round
/// them, one pixel of margin): (width, height, bytes).
pub fn sign_pixels(text: &str, icon_name: Option<&str>) -> (u32, u32, Vec<u8>) {
    let chars: Vec<char> = text.chars().map(|c| c.to_ascii_uppercase()).collect();
    let icon = icon_name.and_then(icon);
    // Two pixels between the last letter and the icon.
    let icon_x = chars.len() * 6 + 2;
    let w = if icon.is_some() { icon_x + 8 } else { chars.len() * 6 + 1 } as u32;
    let h = 9;
    let mut px = vec![0u8; (w * h * 4) as usize];
    for (row, line) in icon.iter().flatten().enumerate() {
        for (col, c) in line.chars().enumerate() {
            if let Some(rgba) = paint(c) {
                let at = ((1 + row) * w as usize + icon_x + col) * 4;
                px[at..at + 4].copy_from_slice(&rgba);
            }
        }
    }
    for (i, c) in chars.iter().enumerate() {
        let Some(glyph) = c.is_ascii_uppercase().then(|| GLYPHS[(*c as u8 - b'A') as usize]) else { continue };
        for (row, bits) in glyph.iter().enumerate() {
            for col in 0..5 {
                if bits & (0x10 >> col) != 0 {
                    let (x, y) = (1 + i * 6 + col, 1 + row);
                    let at = (y * w as usize + x) * 4;
                    px[at..at + 4].copy_from_slice(&INK);
                }
            }
        }
    }
    (w, h, px)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inked(text: &str) -> Vec<String> {
        let (w, h, px) = sign_pixels(text, None);
        (0..h).map(|y| (0..w).map(|x| if px[((y * w + x) * 4 + 3) as usize] > 0 { '#' } else { '.' }).collect()).collect()
    }

    #[test]
    fn letters_draw_in_their_cells() {
        let rows = inked("Hi");
        assert_eq!(rows.len(), 9);
        assert_eq!(rows[0], ".".repeat(13), "top margin");
        assert_eq!(rows[1], ".#...#..###..");
        assert_eq!(rows[4], ".#####...#...");
        assert_eq!(rows[7], ".#...#..###..");
    }

    #[test]
    fn icon_follows_the_text() {
        let (w, h, px) = sign_pixels("AB", Some("gem"));
        assert_eq!((w, h), (22, 9));
        // The gem (its top on the second row) starts one pixel into the icon, in cyan.
        let at = ((2 * w + 15) * 4) as usize;
        assert_eq!(&px[at..at + 4], &[80, 200, 230, 255]);
        // An unknown icon is left out.
        assert_eq!(sign_pixels("AB", Some("unicorn")).0, 13);
    }

    #[test]
    fn every_icon_is_7_by_7_in_known_paints() {
        for name in ["gem", "wand", "coin", "sword", "shield", "bow", "potion", "apple", "hammer", "sack"] {
            let rows = icon(name).unwrap();
            for r in rows {
                assert_eq!(r.chars().count(), 7, "{name}: {r}");
                assert!(r.chars().all(|c| c == '.' || paint(c).is_some()), "{name}: {r}");
            }
        }
    }

    #[test]
    fn spaces_and_unknowns_are_blank() {
        let (w, _, px) = sign_pixels("A 1", None);
        assert_eq!(w, 19);
        assert!(px.chunks(4).filter(|p| p[3] > 0).count() > 0);
        let blank: Vec<String> = inked(" 1");
        assert!(blank.iter().all(|r| !r.contains('#')));
    }
}
