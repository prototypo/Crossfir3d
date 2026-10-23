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

/// Turns a colour name or #rrggbb into RGB.
fn colour(name: &str) -> Option<[u8; 3]> {
    if let Some(hex) = name.strip_prefix('#').filter(|h| h.len() == 6) {
        let v = u32::from_str_radix(hex, 16).ok()?;
        return Some([(v >> 16) as u8, (v >> 8) as u8, v as u8]);
    }
    Some(match name.to_ascii_lowercase().as_str() {
        "black" => [0, 0, 0],
        "white" => [255, 255, 255],
        "red" => [255, 0, 0],
        "orange" => [255, 165, 0],
        "yellow" => [255, 255, 0],
        "green" => [0, 160, 0],
        "blue" => [0, 0, 255],
        "navy" => [0, 0, 128],
        "purple" => [128, 0, 128],
        "gray" | "grey" => [128, 128, 128],
        "brown" => [139, 69, 19],
        "gold" => [255, 215, 0],
        _ => return None,
    })
}

/// Applies one tag to the style; false if it is not a style tag (then it is text).
fn apply(tag: &str, style: &mut Span) -> bool {
    match tag {
        "b" => style.bold = true,
        "/b" => style.bold = false,
        "i" => style.italic = true,
        "/i" => style.italic = false,
        "ul" => style.underline = true,
        "/ul" => style.underline = false,
        "/color" => style.color = None,
        // `[help]topic[/help]` links to a help page; here it is just the word.
        "help" | "/help" => {}
        "fixed" => (style.fixed, style.italic) = (true, false),
        "print" | "arcane" | "strange" | "/fixed" => (style.fixed, style.italic) = (false, false),
        "hand" => (style.fixed, style.italic) = (false, true),
        _ => match tag.strip_prefix("color=").map(colour) {
            Some(Some(c)) => style.color = Some(c),
            _ => return false,
        },
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
}
