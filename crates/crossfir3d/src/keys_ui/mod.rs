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

mod keyboard;
mod lists;
mod press;
#[cfg(test)]
mod tests;

use keyboard::draw_keyboard;
use lists::{draw_buttons, draw_lists};
use press::{apply_clicks, ask_modal, handle_press};

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

/// Runs the key bindings screen: picks a key, shows its binding and applies edits.
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


/// Draws the colour key for bound keys, modifiers and game keys.
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

