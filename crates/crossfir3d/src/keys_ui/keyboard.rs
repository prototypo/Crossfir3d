//! The keyboard picture: the rows of keys and how each key looks.

use super::*;

/// The Mac laptop keyboard (every row 15 units or less): (cap, width in key units, Mac label, other label).
type Row = &'static [(Cap, f32, &'static str, &'static str)];
const ROWS: [Row; 6] = [
    &[
        (Cap::Key("Escape"), 1.0, "esc", "Esc"), (Cap::Key("F1"), 1.0, "F1", "F1"), (Cap::Key("F2"), 1.0, "F2", "F2"),
        (Cap::Key("F3"), 1.0, "F3", "F3"), (Cap::Key("F4"), 1.0, "F4", "F4"), (Cap::Key("F5"), 1.0, "F5", "F5"),
        (Cap::Key("F6"), 1.0, "F6", "F6"), (Cap::Key("F7"), 1.0, "F7", "F7"), (Cap::Key("F8"), 1.0, "F8", "F8"),
        (Cap::Key("F9"), 1.0, "F9", "F9"), (Cap::Key("F10"), 1.0, "F10", "F10"), (Cap::Key("F11"), 1.0, "F11", "F11"),
        (Cap::Key("F12"), 1.0, "F12", "F12"),
    ],
    &[
        (Cap::Key("`"), 1.0, "`", "`"), (Cap::Key("1"), 1.0, "1", "1"), (Cap::Key("2"), 1.0, "2", "2"),
        (Cap::Key("3"), 1.0, "3", "3"), (Cap::Key("4"), 1.0, "4", "4"), (Cap::Key("5"), 1.0, "5", "5"),
        (Cap::Key("6"), 1.0, "6", "6"), (Cap::Key("7"), 1.0, "7", "7"), (Cap::Key("8"), 1.0, "8", "8"),
        (Cap::Key("9"), 1.0, "9", "9"), (Cap::Key("0"), 1.0, "0", "0"), (Cap::Key("-"), 1.0, "-", "-"),
        (Cap::Key("="), 1.0, "=", "="), (Cap::Key("Backspace"), 1.5, "delete", "Backspace"),
    ],
    &[
        (Cap::Key("Tab"), 1.5, "tab", "Tab"), (Cap::Key("Q"), 1.0, "q", "q"), (Cap::Key("W"), 1.0, "w", "w"),
        (Cap::Key("E"), 1.0, "e", "e"), (Cap::Key("R"), 1.0, "r", "r"), (Cap::Key("T"), 1.0, "t", "t"),
        (Cap::Key("Y"), 1.0, "y", "y"), (Cap::Key("U"), 1.0, "u", "u"), (Cap::Key("I"), 1.0, "i", "i"),
        (Cap::Key("O"), 1.0, "o", "o"), (Cap::Key("P"), 1.0, "p", "p"), (Cap::Key("["), 1.0, "[", "["),
        (Cap::Key("]"), 1.0, "]", "]"), (Cap::Key("\\"), 1.0, "\\", "\\"),
    ],
    &[
        (Cap::Off, 1.75, "caps", "Caps"), (Cap::Key("A"), 1.0, "a", "a"), (Cap::Key("S"), 1.0, "s", "s"),
        (Cap::Key("D"), 1.0, "d", "d"), (Cap::Key("F"), 1.0, "f", "f"), (Cap::Key("G"), 1.0, "g", "g"),
        (Cap::Key("H"), 1.0, "h", "h"), (Cap::Key("J"), 1.0, "j", "j"), (Cap::Key("K"), 1.0, "k", "k"),
        (Cap::Key("L"), 1.0, "l", "l"), (Cap::Key(";"), 1.0, ";", ";"), (Cap::Key("'"), 1.0, "'", "'"),
        (Cap::Key("Enter"), 1.75, "return", "Enter"),
    ],
    &[
        (Cap::Mod(0), 2.25, "shift", "Shift"), (Cap::Key("Z"), 1.0, "z", "z"), (Cap::Key("X"), 1.0, "x", "x"),
        (Cap::Key("C"), 1.0, "c", "c"), (Cap::Key("V"), 1.0, "v", "v"), (Cap::Key("B"), 1.0, "b", "b"),
        (Cap::Key("N"), 1.0, "n", "n"), (Cap::Key("M"), 1.0, "m", "m"), (Cap::Key(","), 1.0, ",", ","),
        (Cap::Key("."), 1.0, ".", "."), (Cap::Key("/"), 1.0, "/", "/"), (Cap::Mod(0), 2.75, "shift", "Shift"),
    ],
    &[
        (Cap::Off, 1.0, "fn", "Fn"), (Cap::Mod(1), 1.0, "control", "Ctrl"), (Cap::Mod(2), 1.0, "option", "Alt"),
        (Cap::Mod(3), 1.25, "command", "Win"), (Cap::Key("Space"), 4.5, "space", "Space"),
        (Cap::Mod(3), 1.25, "command", "Win"), (Cap::Mod(2), 1.0, "option", "Alt"), (Cap::Key("Left"), 1.0, "left", "Left"),
        (Cap::Key("Up"), 1.0, "up", "Up"), (Cap::Key("Down"), 1.0, "down", "Down"), (Cap::Key("Right"), 1.0, "right", "Right"),
    ],
];

/// Draws the rows of keys and the button row under them.
pub(super) fn draw_keyboard(ui: &mut egui::Ui, v: &View, out: &mut Out, lists_w: f32) {
    // The widest rows are 15 key units, with a 7 px gap after each key.
    let unit = ((ui.available_width() - lists_w - 40.0) / 15.0 - 7.0).clamp(26.0, 56.0);
    for row in ROWS {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 7.0;
            for spec in row {
                draw_cap(ui, v, out, unit, spec);
            }
        });
    }
    ui.add_space(10.0);
    draw_buttons(ui, v, out);
}

/// Draws one key: its colour, its label, the rings and the dashed outline, and notes a click.
fn draw_cap(ui: &mut egui::Ui, v: &View, out: &mut Out, unit: f32, spec: &CapSpec) {
    let &(cap, w, mac_label, label) = spec;
    let size = egui::vec2(unit * w + 7.0 * (w - 1.0), unit * 0.92);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    let text = cap_text(v.sel, cap, if MAC { mac_label } else { label });
    let painter = ui.painter();
    let (fill, ink, hover) = cap_look(v, cap, painter, rect);
    painter.rect_filled(rect, 6.0, fill);
    painter.text(rect.center(), egui::Align2::CENTER_CENTER, &text, egui::FontId::proportional((unit * 0.3).min(size.x * 1.6 / text.chars().count().max(1) as f32)), ink);
    if is_picked(v, cap) {
        picked_outline(painter, rect);
    }
    let resp = match hover {
        Some(h) => resp.on_hover_text(h),
        None => resp,
    };
    if resp.clicked() {
        out.clicked_cap = Some(cap);
    }
}

/// What a key says: what it really types here, else its standard label.
fn cap_text(sel: &KeysSel, cap: Cap, plain: &str) -> String {
    match cap {
        Cap::Key(k) if k.len() == 1 => sel.labels.get(k).cloned().unwrap_or_else(|| plain.into()),
        _ => plain.into(),
    }
}

/// A key's fill, ink and hover text; paints its modifier rings too.
fn cap_look(v: &View, cap: Cap, painter: &egui::Painter, rect: egui::Rect) -> Look {
    match cap {
        Cap::Off => (DIM, egui::Color32::from_gray(0x77), None),
        Cap::Mod(i) => mod_look(v, i),
        Cap::Key(k) => key_look(v, k, painter, rect),
    }
}

/// A modifier key lights up when in use, and brighter when the selected action uses it.
fn mod_look(v: &View, i: usize) -> Look {
    let (mut fill, mut ink) = (KEY, egui::Color32::from_gray(0xdd));
    if v.pic.mods_used[i] {
        (fill, ink) = (MOD_COLOURS[i], egui::Color32::WHITE);
    }
    if v.pic.lit.iter().any(|b| mod_on(b.combo.mods, i)) {
        (fill, ink) = (BRIGHT, egui::Color32::BLACK);
    }
    (fill, ink, None)
}

/// A bindable key: yellow when bound, bright when the selected action is on it.
fn key_look(v: &View, k: &'static str, painter: &egui::Painter, rect: egui::Rect) -> Look {
    let blocked = blocked_reason(k);
    let here = v.pic.on_key.get(k).map(Vec::as_slice).unwrap_or_default();
    let (mut fill, ink) = key_colours(here, blocked.is_some());
    if v.pic.lit.iter().any(|b| b.combo.key == k) {
        fill = BRIGHT;
    }
    let hover = key_hover(here, blocked);
    paint_rings(painter, rect, here);
    (fill, ink, hover)
}

/// Why a bare key cannot be bound, if it cannot.
fn blocked_reason(k: &str) -> Option<String> {
    let combo = Combo { mods: Mods::default(), key: k.into() };
    reserved(&combo, MAC).map(String::from).or_else(|| game_key(&combo).map(|j| format!("game key: {j}")))
}

/// A key's fill and ink: dim if blocked, yellow if bound.
fn key_colours(here: &[&Binding], blocked: bool) -> (egui::Color32, egui::Color32) {
    if !here.is_empty() {
        (YELLOW, egui::Color32::from_rgb(0x22, 0x22, 0x22))
    } else if blocked {
        (DIM, egui::Color32::from_gray(0x88))
    } else {
        (KEY, egui::Color32::from_gray(0xdd))
    }
}

/// The hover text of a key: what is bound to it, and why it is blocked.
fn key_hover(here: &[&Binding], blocked: Option<String>) -> Option<String> {
    let mut lines: Vec<String> = here.iter().map(|b| format!("{}: {}{}", b.combo.text(MAC), b.command, if b.all { " (all)" } else { "" })).collect();
    lines.extend(blocked);
    (!lines.is_empty()).then(|| lines.join("\n"))
}

/// One ring per modifier bound with this key.
fn paint_rings(painter: &egui::Painter, rect: egui::Rect, here: &[&Binding]) {
    let mut ring = 0.0;
    for (i, colour) in MOD_COLOURS.iter().enumerate() {
        if here.iter().any(|b| mod_on(b.combo.mods, i)) {
            painter.rect_stroke(rect.expand(1.5 + ring), 7.0, egui::Stroke::new(2.5, *colour), egui::StrokeKind::Middle);
            ring += 2.6;
        }
    }
}

/// Whether the key is part of the combination picked so far.
fn is_picked(v: &View, cap: Cap) -> bool {
    match cap {
        Cap::Key(k) => v.sel.picked.as_ref().is_some_and(|c| c.key == k),
        Cap::Mod(i) => mod_on(v.pic.picked_mods, i),
        Cap::Off => false,
    }
}

/// A dashed white outline inside a picked key.
fn picked_outline(painter: &egui::Painter, rect: egui::Rect) {
    let r = rect.shrink(3.0);
    let path = [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()];
    painter.extend(egui::Shape::dashed_line(&path, egui::Stroke::new(2.0, egui::Color32::WHITE), 5.0, 3.0));
}
