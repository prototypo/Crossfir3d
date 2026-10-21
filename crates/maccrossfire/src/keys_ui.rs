//! The key bindings screen (K): a keyboard picture, the spells and skills, and buttons.

use std::collections::HashMap;

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use maccrossfire::keys::{
    action_of, bind_question, default_keys_path, game_key, reserved, save_bindings, skill_command, spell_command, Action,
    Binding, Combo, Mods,
};
use maccrossfire::modes::Mode;

use crate::input::{held_mods, key_name};
use crate::{known_skills, KeyBindings, KeyMode, PlayerStats, Session, SkillNames, Spells};

const MAC: bool = cfg!(target_os = "macos");
const GOLD: egui::Color32 = egui::Color32::from_rgb(0xc9, 0xa2, 0x4a);
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
    let flag = f.into_iter().nth(i).unwrap();
    *flag = !*flag;
}

/// The bindings that run `action` (a typed `cast X` counts as the spell X).
fn bindings_of<'a>(bindings: &'a [Binding], action: &Action, spells: &[String], skills: &[String]) -> Vec<&'a Binding> {
    bindings.iter().filter(|b| action_of(&b.command, spells, skills) == *action).collect()
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

#[allow(clippy::too_many_arguments)]
pub fn keys_screen(
    mut contexts: EguiContexts,
    keys: Res<ButtonInput<KeyCode>>,
    mut typed: MessageReader<KeyboardInput>,
    mode: Res<KeyMode>,
    spells: Res<Spells>,
    stats: Res<PlayerStats>,
    skill_names: Res<SkillNames>,
    mut bindings: ResMut<KeyBindings>,
    mut sel: ResMut<KeysSel>,
    session: Option<NonSend<Session>>,
) -> Result {
    // Learn what each key types, on every screen, so the picture shows this keyboard.
    for e in typed.read() {
        if let (Some(name), Key::Character(c)) = (key_name(e.key_code), &e.logical_key)
            && c.chars().count() == 1
            && !c.trim().is_empty()
        {
            sel.labels.insert(name, c.to_string());
        }
    }
    if mode.0 != Mode::Keys {
        return Ok(());
    }
    let spell_names: Vec<String> = spells.0.all().iter().map(|s| s.name.clone()).collect();
    let skills = known_skills(&stats.0, &skill_names);

    // A real key press picks that combination (its modifiers plus the key).
    if sel.ask.is_none()
        && let Some(key) = keys.get_just_pressed().find_map(|k| key_name(*k))
    {
        let mods = held_mods(&keys);
        if !(mods.none() && matches!(key, "Escape" | "K")) {
            sel.picked = Some(Combo { mods, key: key.into() });
        }
    }

    let bound_now = |sel: &KeysSel, list: &[Binding]| {
        sel.action.as_ref().is_some_and(|a| !bindings_of(list, a, &spell_names, &skills).is_empty())
    };
    let bound = bound_now(&sel, &bindings.0.list);
    let [can_bind, can_change, can_unbind, can_clear] = buttons(&sel, bound);

    // What the picture shows: per key, the modifier sets bound on it; modifiers in use.
    let mut on_key: HashMap<&str, Vec<&Binding>> = HashMap::new();
    let mut mods_used = [false; 4];
    for b in &bindings.0.list {
        on_key.entry(b.combo.key.as_str()).or_default().push(b);
        for (i, used) in mods_used.iter_mut().enumerate() {
            *used |= mod_on(b.combo.mods, i);
        }
    }
    let lit: Vec<&Binding> =
        sel.action.as_ref().map(|a| bindings_of(&bindings.0.list, a, &spell_names, &skills)).unwrap_or_default();
    let picked_mods = sel.picked.as_ref().map_or(sel.mods, |c| c.mods);

    let ctx = contexts.ctx_mut()?;
    let frame = egui::Frame::new()
        .fill(egui::Color32::from_rgba_unmultiplied(20, 22, 30, 238))
        .stroke(egui::Stroke::new(2.0, GOLD))
        .corner_radius(8.0)
        .inner_margin(16.0);
    let screen = ctx.content_rect().shrink(24.0);
    let mut clicked_cap: Option<Cap> = None;
    let mut clicked_action: Option<Action> = None;
    let mut pressed: Option<usize> = None;
    egui::Window::new("key bindings")
        .title_bar(false)
        .resizable(false)
        .movable(false)
        .fixed_rect(screen)
        .frame(frame)
        .show(ctx, |ui| {
            ui.set_min_size(screen.size() - egui::vec2(32.0, 32.0));
            ui.label(egui::RichText::new("K or Esc: close · pick an action, then keys (click them or press them) · also: bind / unbind in the command box").small());
            ui.add_space(6.0);
            legend(ui);
            ui.add_space(8.0);
            ui.horizontal_top(|ui| {
                let lists_w = 460.0f32.min(ui.available_width() * 0.35);
                ui.vertical(|ui| {
                    // The widest rows are 15 key units, with a 7 px gap after each key.
                    let unit = ((ui.available_width() - lists_w - 40.0) / 15.0 - 7.0).clamp(26.0, 56.0);
                    for row in ROWS {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 7.0;
                            for &(cap, w, mac_label, label) in row {
                                let size = egui::vec2(unit * w + 7.0 * (w - 1.0), unit * 0.92);
                                let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
                                let text = match cap {
                                    Cap::Key(k) if k.len() == 1 => sel.labels.get(k).cloned().unwrap_or_else(|| (if MAC { mac_label } else { label }).into()),
                                    _ => (if MAC { mac_label } else { label }).into(),
                                };
                                let painter = ui.painter();
                                let mut fill = KEY;
                                let mut ink = egui::Color32::from_gray(0xdd);
                                let mut hover = None;
                                match cap {
                                    Cap::Off => (fill, ink) = (DIM, egui::Color32::from_gray(0x77)),
                                    Cap::Mod(i) => {
                                        if mods_used[i] {
                                            (fill, ink) = (MOD_COLOURS[i], egui::Color32::WHITE);
                                        }
                                        if lit.iter().any(|b| mod_on(b.combo.mods, i)) {
                                            (fill, ink) = (BRIGHT, egui::Color32::BLACK);
                                        }
                                    }
                                    Cap::Key(k) => {
                                        let combo = Combo { mods: Mods::default(), key: k.into() };
                                        let blocked = reserved(&combo, MAC).map(String::from).or_else(|| game_key(&combo).map(|j| format!("game key: {j}")));
                                        let here = on_key.get(k).map(Vec::as_slice).unwrap_or_default();
                                        if here.is_empty() {
                                            if blocked.is_some() {
                                                (fill, ink) = (DIM, egui::Color32::from_gray(0x88));
                                            }
                                        } else {
                                            (fill, ink) = (YELLOW, egui::Color32::from_rgb(0x22, 0x22, 0x22));
                                        }
                                        if lit.iter().any(|b| b.combo.key == k) {
                                            fill = BRIGHT;
                                        }
                                        let mut lines: Vec<String> = here.iter().map(|b| format!("{}: {}{}", b.combo.text(MAC), b.command, if b.all { " (all)" } else { "" })).collect();
                                        lines.extend(blocked);
                                        hover = (!lines.is_empty()).then(|| lines.join("\n"));
                                        // One ring per modifier bound with this key.
                                        let mut ring = 0.0;
                                        for (i, colour) in MOD_COLOURS.iter().enumerate() {
                                            if here.iter().any(|b| mod_on(b.combo.mods, i)) {
                                                painter.rect_stroke(rect.expand(1.5 + ring), 7.0, egui::Stroke::new(2.5, *colour), egui::StrokeKind::Middle);
                                                ring += 2.6;
                                            }
                                        }
                                    }
                                }
                                painter.rect_filled(rect, 6.0, fill);
                                painter.text(rect.center(), egui::Align2::CENTER_CENTER, &text, egui::FontId::proportional((unit * 0.3).min(size.x * 1.6 / text.chars().count().max(1) as f32)), ink);
                                let picked = match cap {
                                    Cap::Key(k) => sel.picked.as_ref().is_some_and(|c| c.key == k),
                                    Cap::Mod(i) => mod_on(picked_mods, i),
                                    Cap::Off => false,
                                };
                                if picked {
                                    let r = rect.shrink(3.0);
                                    let path = [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()];
                                    painter.extend(egui::Shape::dashed_line(&path, egui::Stroke::new(2.0, egui::Color32::WHITE), 5.0, 3.0));
                                }
                                let resp = match hover {
                                    Some(h) => resp.on_hover_text(h),
                                    None => resp,
                                };
                                if resp.clicked() {
                                    clicked_cap = Some(cap);
                                }
                            }
                        });
                    }
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        for (i, (name, on)) in [("Bind", can_bind), ("Change", can_change), ("Unbind", can_unbind), ("Clear selection", can_clear)].into_iter().enumerate() {
                            if ui.add_enabled(on && sel.ask.is_none(), egui::Button::new(name)).clicked() {
                                pressed = Some(i);
                            }
                        }
                        ui.label(&sel.status);
                    });
                });
                ui.add_space(16.0);
                let heading = |ui: &mut egui::Ui, t: &str| ui.label(egui::RichText::new(t).color(GOLD).strong());
                let row = |ui: &mut egui::Ui, action: Action, clicked: &mut Option<Action>| {
                    let combos: Vec<String> = bindings_of(&bindings.0.list, &action, &spell_names, &skills).iter().map(|b| b.combo.text(MAC)).collect();
                    let text = format!("{}   {}", action_name(&action), combos.join(", "));
                    if ui.selectable_label(sel.action.as_ref() == Some(&action), text).clicked() {
                        *clicked = Some(action);
                    }
                };
                ui.vertical(|ui| {
                    ui.set_width(lists_w / 2.0);
                    heading(ui, "SPELLS");
                    egui::ScrollArea::vertical().id_salt("spells").show(ui, |ui| {
                        for n in &spell_names {
                            row(ui, Action::Spell(n.clone()), &mut clicked_action);
                        }
                    });
                });
                ui.vertical(|ui| {
                    ui.set_width(lists_w / 2.0);
                    heading(ui, "SKILLS");
                    egui::ScrollArea::vertical().id_salt("skills").show(ui, |ui| {
                        for n in &skills {
                            row(ui, Action::Skill(n.clone()), &mut clicked_action);
                        }
                    });
                });
            });
        });

    if let Some(a) = clicked_action {
        sel.action = if sel.action.as_ref() == Some(&a) { None } else { Some(a) };
    }
    match clicked_cap {
        Some(Cap::Mod(i)) => match &mut sel.picked {
            Some(c) => toggle(&mut c.mods, i),
            None => toggle(&mut sel.mods, i),
        },
        Some(Cap::Key(k)) => {
            let here = bindings.0.list.iter().find(|b| b.combo.key == k).map(|b| action_of(&b.command, &spell_names, &skills));
            match (&sel.action, here) {
                // Clicking a bound key with nothing selected selects what it runs.
                (None, Some(action)) if sel.picked.is_none() => sel.action = Some(action),
                _ => {
                    let mods = sel.picked.as_ref().map_or(sel.mods, |c| c.mods);
                    sel.picked = Some(Combo { mods, key: k.into() });
                }
            }
        }
        Some(Cap::Off) | None => {}
    }

    let done = |sel: &mut KeysSel, status: String| {
        (sel.action, sel.picked, sel.mods, sel.ask) = (None, None, Mods::default(), None);
        sel.status = status;
    };
    let save = |bindings: &KeyBindings| {
        if let Some(s) = &session
            && let Err(e) = save_bindings(&default_keys_path(), s.login.character(), &bindings.0)
        {
            warn!("could not save key bindings: {e}");
        }
    };
    let apply = |sel: &mut KeysSel, bindings: &mut KeyBindings, remove: bool, binding: Option<Binding>| {
        let Some(action) = sel.action.clone() else { return };
        if remove {
            let drop: Vec<Binding> = bindings_of(&bindings.0.list, &action, &spell_names, &skills).into_iter().cloned().collect();
            bindings.0.list.retain(|b| !drop.contains(b));
        }
        let status = match &binding {
            Some(b) => format!("Bound {} to {}.", b.combo.text(MAC), action_name(&action)),
            None => format!("Unbound {}.", action_name(&action)),
        };
        if let Some(b) = binding {
            bindings.0.bind(b);
        }
        save(bindings);
        done(sel, status);
    };

    if let Some(i) = pressed {
        let action = sel.action.clone();
        let shared = action.as_ref().is_some_and(|a| bindings_of(&bindings.0.list, a, &spell_names, &skills).iter().any(|b| b.all));
        match (i, action) {
            (3, _) => done(&mut sel, "Selection cleared.".into()),
            (2, Some(_)) if shared => sel.ask = Some(("This removes it for all your characters.".into(), Ask::Change { remove: true, binding: None })),
            (2, Some(_)) => apply(&mut sel, &mut bindings, true, None),
            (0 | 1, Some(action)) => {
                let Some(combo) = sel.picked.clone() else { return Ok(()) };
                let binding = Binding { all: false, combo, edit: false, command: command_of(&action) };
                let remove = i == 1;
                let mut questions = Vec::new();
                match bind_question(&bindings.0, &binding.combo, &binding.command, MAC) {
                    Err(reason) => {
                        sel.status = format!("{} can't be bound: {reason}.", binding.combo.text(MAC));
                        return Ok(());
                    }
                    Ok(q) => questions.extend(q.map(|q| q.replace(" Type y or n.", ""))),
                }
                if remove && shared {
                    questions.push("This removes it for all your characters.".into());
                }
                if questions.is_empty() {
                    apply(&mut sel, &mut bindings, remove, Some(binding));
                } else {
                    sel.ask = Some((questions.join("\n"), Ask::Change { remove, binding: Some(binding) }));
                }
            }
            _ => {}
        }
    }

    if let Some((question, _)) = &sel.ask {
        let question = question.clone();
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
        match answer {
            Some(true) => {
                if let Some((_, Ask::Change { remove, binding })) = sel.ask.take() {
                    apply(&mut sel, &mut bindings, remove, binding);
                }
            }
            Some(false) => done(&mut sel, "Not changed.".into()),
            None => {}
        }
    }
    Ok(())
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
    use super::{buttons, KeysSel};
    use maccrossfire::keys::{Action, Combo, Mods};

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
