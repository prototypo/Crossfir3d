//! The server's text styles: `[b]`, `[i]`, `[ul]`, `[color=...]`, and the fonts `[fixed]`,
//! `[print]`, `[hand]`, `[arcane]`, `[strange]` (a font lasts until the next one).

/// A run of text in one style.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Span {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    /// Fixed-width font (tables in help text).
    pub fixed: bool,
    pub color: Option<[u8; 3]>,
}

const COLOURS: [(&str, [u8; 3]); 13] = [
    ("black", [0, 0, 0]),
    ("white", [255, 255, 255]),
    ("red", [255, 0, 0]),
    ("orange", [255, 165, 0]),
    ("yellow", [255, 255, 0]),
    ("green", [0, 160, 0]),
    ("blue", [0, 0, 255]),
    ("navy", [0, 0, 128]),
    ("purple", [128, 0, 128]),
    ("gray", [128, 128, 128]),
    ("grey", [128, 128, 128]),
    ("brown", [139, 69, 19]),
    ("gold", [255, 215, 0]),
];

/// Parses `#rrggbb`; None if the hex digits are bad.
fn hex_colour(hex: &str) -> Option<[u8; 3]> {
    let v = u32::from_str_radix(hex, 16).ok()?;
    Some([(v >> 16) as u8, (v >> 8) as u8, v as u8])
}

/// Turns a colour name or #rrggbb into RGB.
fn colour(name: &str) -> Option<[u8; 3]> {
    if let Some(hex) = name.strip_prefix('#').filter(|h| h.len() == 6) {
        return hex_colour(hex);
    }
    let lower = name.to_ascii_lowercase();
    COLOURS.iter().find(|(n, _)| *n == lower).map(|(_, c)| *c)
}

/// Applies one tag to the style; false if it is not a style tag (then it is text).
fn apply(tag: &str, style: &mut Span) -> bool {
    toggle_tag(tag, style) || font_tag(tag, style) || colour_tag(tag, style)
}

/// Handles `[b]`, `[i]`, `[ul]` and their closers.
fn toggle_tag(tag: &str, style: &mut Span) -> bool {
    match tag {
        "b" => style.bold = true,
        "/b" => style.bold = false,
        "i" => style.italic = true,
        "/i" => style.italic = false,
        "ul" => style.underline = true,
        "/ul" => style.underline = false,
        _ => return false,
    }
    true
}

/// Handles the font tags (and `[help]`, which leaves the style alone).
fn font_tag(tag: &str, style: &mut Span) -> bool {
    match tag {
        // `[help]topic[/help]` links to a help page; here it is just the word.
        "help" | "/help" => {}
        "fixed" => (style.fixed, style.italic) = (true, false),
        "print" | "arcane" | "strange" | "/fixed" => (style.fixed, style.italic) = (false, false),
        "hand" => (style.fixed, style.italic) = (false, true),
        _ => return false,
    }
    true
}

/// Handles `[/color]` and `[color=...]`; a bad colour is not a tag.
fn colour_tag(tag: &str, style: &mut Span) -> bool {
    if tag == "/color" {
        style.color = None;
        return true;
    }
    match tag.strip_prefix("color=").map(colour) {
        Some(Some(c)) => style.color = Some(c),
        _ => return false,
    }
    true
}

/// The text split into runs of one style each, tags removed.
pub fn spans(text: &str) -> Vec<Span> {
    let mut out: Vec<Span> = Vec::new();
    let mut style = Span::default();
    let mut rest = text;
    let mut push = |s: &str, style: &Span| {
        if s.is_empty() {
            return;
        }
        match out.last_mut() {
            Some(last) if Span { text: String::new(), ..last.clone() } == Span { text: String::new(), ..style.clone() } => {
                last.text.push_str(s)
            }
            _ => out.push(Span { text: s.to_string(), ..style.clone() }),
        }
    };
    while let Some(open) = rest.find('[') {
        let Some(close) = rest[open..].find(']').map(|c| open + c) else { break };
        push(&rest[..open], &style);
        if !apply(&rest[open + 1..close], &mut style) {
            push(&rest[open..=close], &style); // not ours: keep the brackets
        }
        rest = &rest[close + 1..];
    }
    push(rest, &style);
    out
}

/// The text without its style tags.
pub fn plain(text: &str) -> String {
    spans(text).into_iter().map(|s| s.text).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(text: &str) -> Span {
        Span { text: text.into(), ..Default::default() }
    }

    #[test]
    fn every_colour_unchanged() {
        let names = [
            ("black", [0, 0, 0]), ("white", [255, 255, 255]), ("red", [255, 0, 0]),
            ("orange", [255, 165, 0]), ("yellow", [255, 255, 0]), ("green", [0, 160, 0]),
            ("blue", [0, 0, 255]), ("navy", [0, 0, 128]), ("purple", [128, 0, 128]),
            ("gray", [128, 128, 128]), ("grey", [128, 128, 128]), ("brown", [139, 69, 19]),
            ("gold", [255, 215, 0]),
        ];
        for (n, rgb) in names {
            let mixed = format!("{}{}", n[..1].to_uppercase(), &n[1..]);
            for name in [n.to_string(), n.to_uppercase(), mixed] {
                assert_eq!(colour(&name), Some(rgb), "{name}");
            }
        }
        assert_eq!(colour("#ff5500"), Some([255, 85, 0]));
        assert_eq!(colour("#FF5500"), Some([255, 85, 0]));
        for bad in ["#ff55", "#gggggg", "ff5500", "", "chartreuse"] {
            assert_eq!(colour(bad), None, "{bad}");
        }
    }

    #[test]
    fn plain_text_is_one_span() {
        assert_eq!(spans("Hello there"), [s("Hello there")]);
        assert_eq!(spans(""), []);
    }

    #[test]
    fn styles_open_and_close() {
        assert_eq!(
            spans("a [b]bold[/b] [i]it[/i] [ul]u[/ul]"),
            [
                s("a "),
                Span { bold: true, ..s("bold") },
                s(" "),
                Span { italic: true, ..s("it") },
                s(" "),
                Span { underline: true, ..s("u") },
            ]
        );
    }

    #[test]
    fn colours_by_name_and_hex() {
        assert_eq!(spans("[color=#ff5500]hot[/color]")[0].color, Some([0xff, 0x55, 0x00]));
        assert_eq!(spans("[color=blue]sea[/color] x")[0].color, Some([0x00, 0x00, 0xff]));
        assert_eq!(spans("[color=blue]sea[/color] x")[1].color, None);
    }

    #[test]
    fn a_font_lasts_until_the_next() {
        let out = spans("[fixed]a  b\n[print]c");
        assert!(out[0].fixed && out[0].text == "a  b\n");
        assert!(!out[1].fixed && out[1].text == "c");
        assert!(spans("[hand]dear sir")[0].italic);
    }

    #[test]
    fn help_links_and_unknown_brackets() {
        assert_eq!(plain("See [help]bind[/help]."), "See bind.");
        assert_eq!(plain("roll [1d6] and [zz]"), "roll [1d6] and [zz]");
        assert_eq!(plain("[color=red]no[/color] [b]way"), "no way");
    }

    #[test]
    fn apply_every_tag() {
        let red = Some([255, 0, 0]);
        let grey = Some([1, 2, 3]);
        let d = |bold, italic, underline, fixed, color| Span { text: String::new(), bold, italic, underline, fixed, color };
        let f = |bold, italic, underline, fixed, color| Span { text: "t".into(), bold, italic, underline, fixed, color };
        let none = d(false, false, false, false, None);
        let full = f(true, true, true, true, grey);
        // (tag, handled, from default, from fully set)
        let rows = [
            ("b", true, d(true, false, false, false, None), full.clone()),
            ("/b", true, none.clone(), f(false, true, true, true, grey)),
            ("i", true, d(false, true, false, false, None), full.clone()),
            ("/i", true, none.clone(), f(true, false, true, true, grey)),
            ("ul", true, d(false, false, true, false, None), full.clone()),
            ("/ul", true, none.clone(), f(true, true, false, true, grey)),
            ("fixed", true, d(false, false, false, true, None), f(true, false, true, true, grey)),
            ("print", true, none.clone(), f(true, false, true, false, grey)),
            ("arcane", true, none.clone(), f(true, false, true, false, grey)),
            ("strange", true, none.clone(), f(true, false, true, false, grey)),
            ("/fixed", true, none.clone(), f(true, false, true, false, grey)),
            ("hand", true, d(false, true, false, false, None), f(true, true, true, false, grey)),
            ("help", true, none.clone(), full.clone()),
            ("/help", true, none.clone(), full.clone()),
            ("/color", true, none.clone(), f(true, true, true, true, None)),
            ("color=red", true, d(false, false, false, false, red), f(true, true, true, true, red)),
            ("color=#ff0000", true, d(false, false, false, false, red), f(true, true, true, true, red)),
            ("color=nope", false, none.clone(), full.clone()),
            ("color=", false, none.clone(), full.clone()),
            ("B", false, none.clone(), full.clone()),
            ("x", false, none.clone(), full.clone()),
            ("", false, none.clone(), full.clone()),
        ];
        for (tag, handled, from_none, from_full) in rows {
            let mut style = none.clone();
            assert_eq!(apply(tag, &mut style), handled, "{tag}");
            assert_eq!(style, from_none, "{tag} from default");
            let mut style = full.clone();
            assert_eq!(apply(tag, &mut style), handled, "{tag}");
            assert_eq!(style, from_full, "{tag} from full");
        }
    }
}
