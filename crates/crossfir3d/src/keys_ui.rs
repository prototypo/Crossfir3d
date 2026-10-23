//! The key bindings screen (K): a keyboard picture, the spells and skills, and buttons.

use std::collections::HashMap;

use bevy::ecs::system::SystemParam;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use crossfir3d::keys::{
    action_of, bind_question, default_keys_path, game_key, reserved, save_bindings, skill_command, spell_command, Action,
    Binding, Combo, Mods,
};
use crossfir3d::modes::Mode;

use crate::input::{held_mods, key_name};
use crate::ui::{overlay_window, GOLD};
use crate::{known_skills, KeyBindings, KeyMode, PlayerStats, Session, SkillNames, Spells};

const MAC: bool = cfg!(target_os = "macos");
const YELLOW: egui::Color32 = egui::Color32::from_rgb(0xe8, 0xc8, 0x40);
const BRIGHT: egui::Color32 = egui::Color32::from_rgb(0xff, 0xf2, 0x7a);
const KEY: egui::Color32 = egui::Color32::from_rgb(0x3a, 0x3f, 0x4b);
const DIM: egui::Color32 = egui::Color32::from_rgb(0x2a, 0x2d, 0x34);
/// shift, control, option/Alt, command/Win.
const MOD_COLOURS: [egui::Color32; 4] = [
    egui::Color32::from_rgb(0xd9, 0x53, 0x4f),
    egui::Color32::from_rgb(0x8e, 0x5b, 0xd1),
    egui::Color32::from_rgb(0x3d, 0x7f, 0xd9),
    egui::Color32::from_rgb(0x3a, 0xa8, 0x60),
];

/// A question waiting for Replace or Cancel.
pub enum Ask {
    /// Remove the selected action's bindings (if `remove`), then add `binding` (if any).
    Change { remove: bool, binding: Option<Binding> },
}

#[derive(Resource, Default)]
pub struct KeysSel {
    pub action: Option<Action>,
    pub picked: Option<Combo>,
    /// Modifiers clicked before a key.
    pub mods: Mods,
    pub ask: Option<(String, Ask)>,
    /// What each key really types on this keyboard, learnt from key presses.
    pub labels: HashMap<&'static str, String>,
    pub status: String,
    /// The "New command" box under OTHER.
    pub new_command: String,
    /// The box has the keyboard: keys type text, they are not picked (and K does not close).
    pub typing: bool,
}

impl KeysSel {
    /// A fresh screen: nothing selected or asked; the learnt key labels stay.
    pub fn reset(&mut self) {
        *self = KeysSel { labels: std::mem::take(&mut self.labels), ..Default::default() };
    }
}

/// The binding a click on `key` selects: the one with the modifiers held or picked,
/// else any binding on that key.
pub fn binding_for_click<'a>(list: &'a [Binding], key: &str, mods: Mods) -> Option<&'a Binding> {
    let on_key = || list.iter().filter(|b| b.combo.key == key);
    on_key().find(|b| b.combo.mods == mods).or_else(|| on_key().next())
}

/// Which of Bind, Change, Unbind, Clear selection can act.
pub fn buttons(sel: &KeysSel, bound: bool) -> [bool; 4] {
    let (action, picked) = (sel.action.is_some(), sel.picked.is_some());
    [action && picked && !bound, action && picked && bound, action && bound, action || picked]
}

/// A key on the picture: a bindable key (by binding name), a modifier (0-3), or a
/// key bindings cannot use.
#[derive(Clone, Copy)]
enum Cap {
    Key(&'static str),
    Mod(usize),
    Off,
}

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

fn mod_on(m: Mods, i: usize) -> bool {
    [m.shift, m.ctrl, m.alt, m.cmd][i]
}

fn toggle(m: &mut Mods, i: usize) {
    let f = [&mut m.shift, &mut m.ctrl, &mut m.alt, &mut m.cmd];
    let flag = f.into_iter().nth(i).expect("i is a modifier column, 0..4");
    *flag = !*flag;
}

/// The bindings that run `action` (a typed `cast X` counts as the spell X).
fn bindings_of<'a>(bindings: &'a [Binding], action: &Action, spells: &[String], skills: &[String]) -> Vec<&'a Binding> {
    bindings.iter().filter(|b| action_of(&b.command, spells, skills) == *action).collect()
}

/// Bound commands that are not a known spell or skill (typed `bind say hi`, ...),
/// each once, sorted: the OTHER list.
pub fn other_commands(bindings: &[Binding], spells: &[String], skills: &[String]) -> Vec<String> {
    let mut out: Vec<String> = bindings
        .iter()
        .filter_map(|b| match action_of(&b.command, spells, skills) {
            Action::Command(c) => Some(c),
            _ => None,
        })
        .collect();
    out.sort();
    out.dedup();
    out
}

/// The action a command typed in the "New command" box selects (a known spell or skill
/// is that spell or skill); None for an empty box.
pub fn new_command(text: &str, spells: &[String], skills: &[String]) -> Option<Action> {
    let text = text.trim();
    (!text.is_empty()).then(|| action_of(text, spells, skills))
}

fn command_of(action: &Action) -> String {
    match action {
        Action::Spell(n) => spell_command(n),
        Action::Skill(n) => skill_command(n),
        Action::Command(c) => c.clone(),
    }
}

fn action_name(action: &Action) -> &str {
    match action {
        Action::Spell(n) | Action::Skill(n) | Action::Command(n) => n,
    }
}

/// Everything the key bindings screen reads or changes in a frame.
#[derive(SystemParam)]
pub struct KeysScreenParams<'w, 's> {
    contexts: EguiContexts<'w, 's>,
    keys: Res<'w, ButtonInput<KeyCode>>,
    typed: MessageReader<'w, 's, KeyboardInput>,
    mode: Res<'w, KeyMode>,
    spells: Res<'w, Spells>,
    stats: Res<'w, PlayerStats>,
    skill_names: Res<'w, SkillNames>,
    bindings: ResMut<'w, KeyBindings>,
    sel: ResMut<'w, KeysSel>,
    session: Option<NonSend<'w, Session>>,
}

/// The names of the spells and the skills the player knows.
struct Known {
    spells: Vec<String>,
    skills: Vec<String>,
}

/// What a frame needs to change bindings: the known names, and where to save.
struct Env<'a> {
    known: &'a Known,
    session: Option<&'a Session>,
}

/// What the keyboard picture shows: per key, the bindings on it; modifiers in use;
/// the bindings of the selected action; the modifiers picked so far.
struct Picture<'a> {
    on_key: HashMap<&'a str, Vec<&'a Binding>>,
    mods_used: [bool; 4],
    lit: Vec<&'a Binding>,
    picked_mods: Mods,
}

/// Read-only state for drawing one frame of the screen.
struct View<'a> {
    sel: &'a KeysSel,
    list: &'a [Binding],
    known: &'a Known,
    pic: Picture<'a>,
    buttons: [bool; 4],
}

/// What the player did while the screen was drawn.
#[derive(Default)]
struct Out {
    clicked_cap: Option<Cap>,
    clicked_action: Option<Action>,
    pressed: Option<usize>,
    new_text: String,
    typing: bool,
    typed_action: Option<Action>,
}

type Look = (egui::Color32, egui::Color32, Option<String>);
type CapSpec = (Cap, f32, &'static str, &'static str);

pub fn keys_screen(mut p: KeysScreenParams) -> Result {
    learn_labels(&mut p.typed, &mut p.sel);
    if p.mode.0 != Mode::Keys {
        return Ok(());
    }
    let known = Known {
        spells: p.spells.0.all().iter().map(|s| s.name.clone()).collect(),
        skills: known_skills(&p.stats.0, &p.skill_names),
    };
    pick_pressed_key(&p.keys, &mut p.sel);
    let bound = is_bound(&p.sel, &p.bindings.0.list, &known);
    let buttons = buttons(&p.sel, bound);

    let ctx = p.contexts.ctx_mut()?;
    let screen = ctx.content_rect().shrink(24.0);
    let mut out = Out { new_text: std::mem::take(&mut p.sel.new_command), ..Default::default() };
    let view = View {
        sel: &p.sel,
        list: &p.bindings.0.list,
        known: &known,
        pic: build_picture(&p.sel, &p.bindings.0.list, &known),
        buttons,
    };
    overlay_window("key bindings", screen, 16.0)
        .show(ctx, |ui| draw_screen(ui, &view, &mut out, screen.size()));

    let pressed = out.pressed;
    apply_clicks(&mut p.sel, out, &p.bindings.0.list, &p.keys, &known);
    let env = Env { known: &known, session: p.session.as_deref() };
    if let Some(i) = pressed
        && handle_press(i, &mut p.sel, &mut p.bindings, &env)
    {
        return Ok(());
    }
    ask_modal(ctx, &mut p.sel, &mut p.bindings, &env);
    Ok(())
}

/// Learns what each key types, on every screen, so the picture shows this keyboard.
fn learn_labels(typed: &mut MessageReader<KeyboardInput>, sel: &mut KeysSel) {
    for e in typed.read() {
        if let (Some(name), Key::Character(c)) = (key_name(e.key_code), &e.logical_key)
            && c.chars().count() == 1
            && !c.trim().is_empty()
        {
            sel.labels.insert(name, c.to_string());
        }
    }
}

/// A real key press picks that combination (its modifiers plus the key).
fn pick_pressed_key(keys: &ButtonInput<KeyCode>, sel: &mut KeysSel) {
    if sel.ask.is_none()
        && !sel.typing
        && let Some(key) = keys.get_just_pressed().find_map(|k| key_name(*k))
    {
        let mods = held_mods(keys);
        if !(mods.none() && matches!(key, "Escape" | "K")) {
            sel.picked = Some(Combo { mods, key: key.into() });
        }
    }
}

/// Whether the selected action has any binding.
fn is_bound(sel: &KeysSel, list: &[Binding], known: &Known) -> bool {
    sel.action.as_ref().is_some_and(|a| !bindings_of(list, a, &known.spells, &known.skills).is_empty())
}

/// Whether any binding of `action` is shared by all characters.
fn is_shared(list: &[Binding], action: &Action, known: &Known) -> bool {
    bindings_of(list, action, &known.spells, &known.skills).iter().any(|b| b.all)
}

/// Works out what the picture shows from the bindings and the selection.
fn build_picture<'a>(sel: &KeysSel, list: &'a [Binding], known: &Known) -> Picture<'a> {
    let mut on_key: HashMap<&str, Vec<&Binding>> = HashMap::new();
    let mut mods_used = [false; 4];
    for b in list {
        on_key.entry(b.combo.key.as_str()).or_default().push(b);
        for (i, used) in mods_used.iter_mut().enumerate() {
            *used |= mod_on(b.combo.mods, i);
        }
    }
    let lit = sel.action.as_ref().map(|a| bindings_of(list, a, &known.spells, &known.skills)).unwrap_or_default();
    let picked_mods = sel.picked.as_ref().map_or(sel.mods, |c| c.mods);
    Picture { on_key, mods_used, lit, picked_mods }
}

/// Draws the help line, the legend, the keyboard and the lists.
fn draw_screen(ui: &mut egui::Ui, v: &View, out: &mut Out, screen: egui::Vec2) {
    ui.set_min_size(screen - egui::vec2(32.0, 32.0));
    ui.label(egui::RichText::new("K or Esc: close · pick an action (or type a new command under OTHER), then keys (click them or press them) · also: bind / unbind in the command box").small());
    ui.add_space(6.0);
    legend(ui);
    ui.add_space(8.0);
    ui.horizontal_top(|ui| {
        let lists_w = 660.0f32.min(ui.available_width() * 0.42);
        ui.vertical(|ui| draw_keyboard(ui, v, out, lists_w));
        ui.add_space(16.0);
        draw_lists(ui, v, out, lists_w);
    });
}

/// Draws the rows of keys and the button row under them.
fn draw_keyboard(ui: &mut egui::Ui, v: &View, out: &mut Out, lists_w: f32) {
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

/// Draws the Bind, Change, Unbind and Clear selection buttons and the status.
fn draw_buttons(ui: &mut egui::Ui, v: &View, out: &mut Out) {
    ui.horizontal(|ui| {
        let [can_bind, can_change, can_unbind, can_clear] = v.buttons;
        for (i, (name, on)) in [("Bind", can_bind), ("Change", can_change), ("Unbind", can_unbind), ("Clear selection", can_clear)].into_iter().enumerate() {
            if ui.add_enabled(on && v.sel.ask.is_none(), egui::Button::new(name)).clicked() {
                out.pressed = Some(i);
            }
        }
        ui.label(&v.sel.status);
    });
}

/// Draws the SPELLS, SKILLS and OTHER columns.
fn draw_lists(ui: &mut egui::Ui, v: &View, out: &mut Out, lists_w: f32) {
    let w = lists_w / 3.0;
    ui.vertical(|ui| {
        ui.set_width(w);
        heading(ui, "SPELLS");
        scroll_list(ui, v, &mut out.clicked_action, "spells", v.known.spells.iter().map(|n| Action::Spell(n.clone())).collect());
    });
    ui.vertical(|ui| {
        ui.set_width(w);
        heading(ui, "SKILLS");
        scroll_list(ui, v, &mut out.clicked_action, "skills", v.known.skills.iter().map(|n| Action::Skill(n.clone())).collect());
    });
    ui.vertical(|ui| {
        ui.set_width(w);
        heading(ui, "OTHER");
        scroll_list(ui, v, &mut out.clicked_action, "other", other_actions(v));
        new_command_box(ui, v, out);
    });
}

fn heading(ui: &mut egui::Ui, t: &str) {
    ui.label(egui::RichText::new(t).color(GOLD).strong());
}

/// A scrolling list of actions, each with its keys; a click is noted in `clicked`.
fn scroll_list(ui: &mut egui::Ui, v: &View, clicked: &mut Option<Action>, salt: &str, actions: Vec<Action>) {
    egui::ScrollArea::vertical().id_salt(salt).show(ui, |ui| {
        for action in actions {
            action_row(ui, v, action, clicked);
        }
    });
}

/// One selectable line: the action's name and the keys bound to it.
fn action_row(ui: &mut egui::Ui, v: &View, action: Action, clicked: &mut Option<Action>) {
    let combos: Vec<String> = bindings_of(v.list, &action, &v.known.spells, &v.known.skills).iter().map(|b| b.combo.text(MAC)).collect();
    let text = format!("{}   {}", action_name(&action), combos.join(", "));
    if ui.selectable_label(v.sel.action.as_ref() == Some(&action), text).clicked() {
        *clicked = Some(action);
    }
}

/// The OTHER list; a new command typed below shows here, selected, until it is bound.
fn other_actions(v: &View) -> Vec<Action> {
    let mut others = other_commands(v.list, &v.known.spells, &v.known.skills);
    if let Some(Action::Command(c)) = &v.sel.action
        && !others.contains(c)
    {
        others.insert(0, c.clone());
    }
    others.into_iter().map(Action::Command).collect()
}

/// The "New command" text box under OTHER; Enter selects what was typed.
fn new_command_box(ui: &mut egui::Ui, v: &View, out: &mut Out) {
    ui.add_space(8.0);
    ui.label(egui::RichText::new("New command (Enter selects it):").small());
    let edit = ui.add(egui::TextEdit::singleline(&mut out.new_text).hint_text("say hello"));
    out.typing = edit.has_focus();
    if edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
        out.typed_action = new_command(&out.new_text, &v.known.spells, &v.known.skills);
        out.new_text.clear();
    }
}

/// Applies what was typed and clicked while drawing.
fn apply_clicks(sel: &mut KeysSel, out: Out, list: &[Binding], keys: &ButtonInput<KeyCode>, known: &Known) {
    (sel.new_command, sel.typing) = (out.new_text, out.typing);
    if let Some(a) = out.typed_action {
        sel.action = Some(a);
        sel.status = "Now pick keys, then press Bind.".into();
    }
    if let Some(a) = out.clicked_action {
        sel.action = if sel.action.as_ref() == Some(&a) { None } else { Some(a) };
    }
    match out.clicked_cap {
        Some(Cap::Mod(i)) => match &mut sel.picked {
            Some(c) => toggle(&mut c.mods, i),
            None => toggle(&mut sel.mods, i),
        },
        Some(Cap::Key(k)) => click_key(sel, k, list, keys, known),
        Some(Cap::Off) | None => {}
    }
}

/// A click on a key: selects what a bound key runs (if nothing is selected), else picks the key.
fn click_key(sel: &mut KeysSel, k: &'static str, list: &[Binding], keys: &ButtonInput<KeyCode>, known: &Known) {
    let mods = held_mods(keys);
    let mods = if mods.none() { sel.mods } else { mods };
    let here = binding_for_click(list, k, mods).map(|b| action_of(&b.command, &known.spells, &known.skills));
    match (&sel.action, here) {
        // Clicking a bound key with nothing selected selects what it runs.
        (None, Some(action)) if sel.picked.is_none() => sel.action = Some(action),
        _ => {
            let mods = sel.picked.as_ref().map_or(sel.mods, |c| c.mods);
            sel.picked = Some(Combo { mods, key: k.into() });
        }
    }
}

/// Ends the current selection and question, and says `status`.
fn done(sel: &mut KeysSel, status: String) {
    (sel.action, sel.picked, sel.mods, sel.ask) = (None, None, Mods::default(), None);
    sel.status = status;
}

/// Saves the key bindings for the logged-in character, if there is one.
fn save(bindings: &KeyBindings, session: Option<&Session>) {
    if let Some(s) = session
        && let Err(e) = save_bindings(&default_keys_path(), s.login.character(), &bindings.0)
    {
        warn!("could not save key bindings: {e}");
    }
}

/// Removes the selected action's bindings (if `remove`), adds `binding` (if any), and saves.
fn apply(sel: &mut KeysSel, bindings: &mut KeyBindings, env: &Env, remove: bool, binding: Option<Binding>) {
    let Some(action) = sel.action.clone() else { return };
    if remove {
        let drop: Vec<Binding> = bindings_of(&bindings.0.list, &action, &env.known.spells, &env.known.skills).into_iter().cloned().collect();
        bindings.0.list.retain(|b| !drop.contains(b));
    }
    let status = match &binding {
        Some(b) => format!("Bound {} to {}.", b.combo.text(MAC), action_name(&action)),
        None => format!("Unbound {}.", action_name(&action)),
    };
    if let Some(b) = binding {
        bindings.0.bind(b);
    }
    save(bindings, env.session);
    done(sel, status);
}

/// Handles a pressed button (0 Bind, 1 Change, 2 Unbind, 3 Clear); true means stop the frame here.
fn handle_press(i: usize, sel: &mut KeysSel, bindings: &mut KeyBindings, env: &Env) -> bool {
    let action = sel.action.clone();
    let shared = action.as_ref().is_some_and(|a| is_shared(&bindings.0.list, a, env.known));
    match (i, action) {
        (3, _) => done(sel, "Selection cleared.".into()),
        (2, Some(_)) if shared => sel.ask = Some(("This removes it for all your characters.".into(), Ask::Change { remove: true, binding: None })),
        (2, Some(_)) => apply(sel, bindings, env, true, None),
        (0 | 1, Some(action)) => return bind_press(sel, bindings, env, i == 1, action),
        _ => {}
    }
    false
}

/// Bind or Change: applies the picked key now, or asks first; true means stop the frame here.
fn bind_press(sel: &mut KeysSel, bindings: &mut KeyBindings, env: &Env, remove: bool, action: Action) -> bool {
    let Some(combo) = sel.picked.clone() else { return true };
    let binding = Binding { all: false, combo, edit: false, command: command_of(&action) };
    let mut questions = Vec::new();
    match bind_question(&bindings.0, &binding.combo, &binding.command, MAC) {
        Err(reason) => {
            sel.status = format!("{} can't be bound: {reason}.", binding.combo.text(MAC));
            return true;
        }
        Ok(q) => questions.extend(q.map(|q| q.replace(" Type y or n.", ""))),
    }
    if remove && is_shared(&bindings.0.list, &action, env.known) {
        questions.push("This removes it for all your characters.".into());
    }
    if questions.is_empty() {
        apply(sel, bindings, env, remove, Some(binding));
    } else {
        sel.ask = Some((questions.join("\n"), Ask::Change { remove, binding: Some(binding) }));
    }
    false
}

/// The Replace or Cancel question, if one is waiting.
fn ask_modal(ctx: &egui::Context, sel: &mut KeysSel, bindings: &mut KeyBindings, env: &Env) {
    let Some((question, _)) = &sel.ask else { return };
    match modal_answer(ctx, question.clone()) {
        Some(true) => {
            if let Some((_, Ask::Change { remove, binding })) = sel.ask.take() {
                apply(sel, bindings, env, remove, binding);
            }
        }
        Some(false) => done(sel, "Not changed.".into()),
        None => {}
    }
}

/// Shows the question with Replace and Cancel; Some(true) for Replace, Some(false) for Cancel.
fn modal_answer(ctx: &egui::Context, question: String) -> Option<bool> {
    let mut answer = None;
    egui::Modal::new(egui::Id::new("keys ask")).show(ctx, |ui| {
        ui.label(question);
        ui.horizontal(|ui| {
            if ui.button("Replace").clicked() {
                answer = Some(true);
            }
            if ui.button("Cancel").clicked() {
                answer = Some(false);
            }
        });
    });
    answer
}

fn legend(ui: &mut egui::Ui) {
    let names = if MAC { ["shift", "control", "option", "command"] } else { ["Shift", "Ctrl", "Alt", "Win"] };
    ui.horizontal(|ui| {
        let chip = |ui: &mut egui::Ui, text: &str, fill: egui::Color32, ink: egui::Color32| {
            egui::Frame::new().fill(fill).corner_radius(4.0).inner_margin(egui::vec2(8.0, 2.0)).show(ui, |ui| {
                ui.label(egui::RichText::new(text).color(ink).small());
            });
        };
        chip(ui, "bound key", YELLOW, egui::Color32::from_rgb(0x22, 0x22, 0x22));
        for (i, n) in names.iter().enumerate() {
            chip(ui, n, MOD_COLOURS[i], egui::Color32::WHITE);
        }
        chip(ui, "game key", DIM, egui::Color32::from_gray(0x88));
    });
}

#[cfg(test)]
mod tests {
    use super::{binding_for_click, buttons, new_command, other_commands, KeysSel};
    use crossfir3d::keys::{Action, Binding, Combo, Mods};

    #[test]
    fn typed_new_command_becomes_the_action() {
        let spells = ["burning hands".to_string()];
        assert_eq!(new_command("  say hello ", &spells, &[]), Some(Action::Command("say hello".into())));
        assert_eq!(new_command("cast Burning Hands", &spells, &[]), Some(Action::Spell("burning hands".into())));
        assert_eq!(new_command("   ", &spells, &[]), None);
    }

    #[test]
    fn other_list_has_each_plain_command_once() {
        let b = |key: &str, command: &str| Binding { all: false, combo: Combo { mods: Mods::default(), key: key.into() }, edit: false, command: command.into() };
        let list = [b("Q", "say hi"), b("W", "cast burning hands"), b("E", "use_skill praying"), b("R", "say hi"), b("T", "cast fireball")];
        let (spells, skills) = (["burning hands".to_string()], ["praying".to_string()]);
        assert_eq!(other_commands(&list, &spells, &skills), ["cast fireball", "say hi"]);
    }

    #[test]
    fn click_picks_the_binding_with_the_held_modifiers() {
        let b = |mods: Mods, command: &str| Binding { all: false, combo: Combo { mods, key: "G".into() }, edit: false, command: command.into() };
        let shift_cmd = Mods { shift: true, cmd: true, ..Default::default() };
        let list = [b(Mods::default(), "say plain"), b(shift_cmd, "say shift cmd")];
        assert_eq!(binding_for_click(&list, "G", shift_cmd).unwrap().command, "say shift cmd");
        assert_eq!(binding_for_click(&list, "G", Mods::default()).unwrap().command, "say plain");
        assert_eq!(binding_for_click(&list[1..], "G", Mods::default()).unwrap().command, "say shift cmd");
        assert!(binding_for_click(&list, "H", Mods::default()).is_none());
    }

    #[test]
    fn reset_clears_all_but_labels() {
        let mut s = KeysSel { action: Some(Action::Command("x".into())), status: "y".into(), new_command: "z".into(), typing: true, ..Default::default() };
        s.labels.insert("Q", "a".into());
        s.reset();
        assert!(s.action.is_none() && s.status.is_empty() && s.new_command.is_empty() && !s.typing);
        assert_eq!(s.labels.get("Q").map(String::as_str), Some("a"));
    }

    #[test]
    fn buttons_follow_input() {
        let mut s = KeysSel::default();
        assert_eq!(buttons(&s, false), [false, false, false, false]);
        s.action = Some(Action::Spell("burning hands".into()));
        assert_eq!(buttons(&s, false), [false, false, false, true]);
        s.picked = Some(Combo { mods: Mods { alt: true, ..Default::default() }, key: "6".into() });
        assert_eq!(buttons(&s, false), [true, false, false, true]);
        assert_eq!(buttons(&s, true), [false, true, true, true]);
    }
}
