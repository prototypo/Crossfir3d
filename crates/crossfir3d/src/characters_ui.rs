//! After account login: the character list and the new-character screen.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiTextureHandle};
use cfproto::{CharacterInfo, ClientMsg};
use crossfir3d::login::{still_connecting, LoginState};
use crossfir3d::newchar::{stat_code, NewChar, STAT_NAMES};
use crossfir3d::races::{default_body_path, save_body, Body};

use crate::characters3d::Preview;
use crate::{Session, Status};

const GOLD: egui::Color32 = egui::Color32::from_rgb(0xc9, 0xa2, 0x4a);

fn panel() -> egui::Frame {
    egui::Frame::new()
        .fill(egui::Color32::from_rgb(20, 22, 30))
        .stroke(egui::Stroke::new(2.0, GOLD))
        .corner_radius(8.0)
        .inner_margin(20.0)
}

fn heading(ui: &mut egui::Ui, text: &str) {
    ui.label(egui::RichText::new(text).color(GOLD).strong().size(16.0));
}

/// Seconds since the epoch in nanoseconds: a fresh seed for each roll.
fn seed() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.as_nanos() as u64)
}

/// Everything `characters_screen` reads or changes in a frame.
#[derive(SystemParam)]
pub struct CharactersParams<'w, 's> {
    contexts: EguiContexts<'w, 's>,
    keys: Res<'w, ButtonInput<KeyCode>>,
    status: ResMut<'w, Status>,
    auto: Local<'s, bool>,
    commands: Commands<'w, 's>,
    preview: ResMut<'w, Preview>,
    session: Option<NonSendMut<'w, Session>>,
}

/// Draws the screen for the current login state: the character list, the new-character form or "connecting".
pub fn characters_screen(p: CharactersParams) -> Result {
    let CharactersParams { mut contexts, keys, mut status, mut auto, mut commands, mut preview, session } = p;
    let Some(mut session) = session else { return Ok(()) };
    let picture = contexts.add_image(EguiTextureHandle::Weak(preview.image.id()));
    let connecting = still_connecting(session.login.state());
    if status.0 == "Connecting..." && !connecting {
        status.0.clear(); // the list or form is showing now
    }
    let ctx = contexts.ctx_mut()?;
    let screen = ctx.content_rect().shrink(24.0);
    open_dev_screen(&mut auto, &mut session);
    match session.login.state().clone() {
        LoginState::Characters => {
            let (out, back) = character_list(ctx, screen, &mut session, &status.0, &keys);
            send_choice(out, back, &mut session, &mut status, &mut commands);
        }
        LoginState::Creating => new_character(ctx, screen, &mut session, &mut preview.body, picture),
        _ if connecting => connecting_window(ctx),
        _ => {}
    }
    Ok(())
}

/// Sends the chosen messages (clearing the status line if there are any) and, on Back, queues closing the connection.
fn send_choice(out: Vec<ClientMsg>, back: bool, session: &mut Session, status: &mut Status, commands: &mut Commands) {
    if !out.is_empty() {
        status.0.clear();
    }
    session.send(out);
    if back {
        // No account logout in the protocol: close the connection.
        commands.queue(|world: &mut World| crate::end_session(world, String::new()));
    }
}

/// Developer screenshots: CROSSFIR3D_SCREEN=newchar[:race_arch] opens the
/// new-character screen, once, from the character list.
fn open_dev_screen(auto: &mut bool, session: &mut Session) {
    let dev = std::env::var("CROSSFIR3D_SCREEN").ok().filter(|v| v.starts_with("newchar"));
    if !*auto && dev.is_some() && *session.login.state() == LoginState::Characters {
        *auto = true;
        let out = session.login.start_creating();
        session.send(out);
    }
}

/// The small centred "Connecting..." window.
fn connecting_window(ctx: &egui::Context) {
    egui::Window::new("connecting").title_bar(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
        ui.label("Connecting...");
    });
}

/// The character window; returns the messages to send and whether Back was clicked.
fn character_list(ctx: &egui::Context, screen: egui::Rect, session: &mut Session, status: &str, keys: &ButtonInput<KeyCode>) -> (Vec<ClientMsg>, bool) {
    let mut out = Vec::new();
    let mut back = false;
    let chars = session.login.characters().to_vec();
    egui::Window::new("characters")
        .title_bar(false)
        .resizable(false)
        .movable(false)
        .fixed_rect(screen)
        .frame(panel())
        .show(ctx, |ui| {
            ui.set_min_size(screen.size() - egui::vec2(40.0, 40.0));
            heading(ui, "CHOOSE A CHARACTER");
            if !status.is_empty() {
                ui.label(status);
            }
            if let Some(notice) = &session.login.notice {
                ui.label(egui::RichText::new(notice).color(egui::Color32::from_rgb(0xe0, 0x80, 0x60)));
            }
            ui.add_space(12.0);
            if chars.is_empty() {
                ui.label("No characters yet.");
            }
            if let Some(played) = character_rows(ui, session, &chars) {
                out = played;
            }
            ui.add_space(16.0);
            ui.horizontal(|ui| {
                let (picked, went_back) = character_buttons(ui, session, &chars, keys);
                if let Some(picked) = picked {
                    out = picked;
                }
                back = went_back;
            });
        });
    (out, back)
}

/// One selectable row per character; a double-click returns the messages that play it.
fn character_rows(ui: &mut egui::Ui, session: &mut Session, chars: &[CharacterInfo]) -> Option<Vec<ClientMsg>> {
    let mut out = None;
    for (i, c) in chars.iter().enumerate() {
        let text = format!("{} — {} {}, level {}", c.name, c.race, c.class, c.level);
        let row = ui.selectable_label(session.login.selected == Some(i), egui::RichText::new(text).size(15.0));
        if row.clicked() {
            session.login.selected = Some(i);
        }
        if row.double_clicked() {
            out = Some(session.login.play(&c.name));
        }
    }
    out
}

/// The Play, New character and Back buttons; returns the messages of a Play or New click, and whether Back was clicked.
fn character_buttons(ui: &mut egui::Ui, session: &mut Session, chars: &[CharacterInfo], keys: &ButtonInput<KeyCode>) -> (Option<Vec<ClientMsg>>, bool) {
    let mut out = None;
    let chosen = session.login.selected.and_then(|i| chars.get(i));
    let play = ui.add_enabled(chosen.is_some(), egui::Button::new("Play"));
    if let (Some(c), true) = (chosen, play.clicked() || keys.just_pressed(KeyCode::Enter)) {
        out = Some(session.login.play(&c.name));
    }
    if ui.button("New character").clicked() {
        out = Some(session.login.start_creating());
    }
    (out, ui.button("Back").clicked())
}

fn new_character(ctx: &egui::Context, screen: egui::Rect, session: &mut Session, body: &mut Body, picture: egui::TextureId) {
    let mut create = false;
    let mut back = false;
    let pending = session.login.create_pending();
    let n = &mut session.login.newchar;
    egui::Window::new("new character")
        .title_bar(false)
        .resizable(false)
        .movable(false)
        .fixed_rect(screen)
        .frame(panel())
        .show(ctx, |ui| {
            ui.set_min_size(screen.size() - egui::vec2(40.0, 40.0));
            heading(ui, "NEW CHARACTER");
            ui.add_space(8.0);
            ui.columns(3, |cols| {
                race_class_column(&mut cols[0], n);
                about_column(&mut cols[1], n);
                (create, back) = stats_column(&mut cols[2], n, body, picture, pending);
            });
        });
    dev_autofill(n);
    if create {
        let name = session.login.newchar.name.clone();
        if let Err(e) = save_body(&default_body_path(), &name, *body) {
            warn!("could not save the body choice: {e}");
        }
        let msgs = session.login.create();
        session.send(msgs);
    }
    if back {
        session.login.back_to_list();
    }
}

/// One scrolling list of archetype names; returns the one clicked this frame.
fn pick_list(ui: &mut egui::Ui, salt: &str, items: &[String], n: &NewChar, selected: Option<&str>) -> Option<String> {
    let mut clicked = None;
    egui::ScrollArea::vertical().id_salt(salt).max_height(220.0).show(ui, |ui| {
        for r in items {
            let name = n.infos.get(r).map_or(r.clone(), |i| i.name.clone());
            if ui.selectable_label(selected == Some(r), name).clicked() {
                clicked = Some(r.clone());
            }
        }
    });
    clicked
}

/// Column 1: race and class lists.
fn race_class_column(ui: &mut egui::Ui, n: &mut NewChar) {
    heading(ui, "Race");
    if let Some(r) = pick_list(ui, "races", &n.races, n, n.race.as_deref()) {
        n.race = Some(r);
        n.refresh_choices();
    }
    heading(ui, "Class");
    if let Some(c) = pick_list(ui, "classes", &n.classes, n, n.class.as_deref()) {
        n.class = Some(c);
        n.refresh_choices();
    }
}

/// The summed stat changes of a race or class, like "Str +1  Dex -2".
fn stat_changes(info: &cfproto::ArchInfo) -> String {
    (0..7)
        .filter_map(|i| {
            let v: i16 = info.stats.iter().filter(|(c, _)| *c == stat_code(i)).map(|(_, v)| *v).sum();
            (v != 0).then(|| format!("{} {:+}", STAT_NAMES[i], v))
        })
        .collect::<Vec<_>>()
        .join("  ")
}

/// One combo box per choice the chosen race and class offer.
fn choice_boxes(ui: &mut egui::Ui, n: &mut NewChar) {
    for choice in n.offered_choices().into_iter().cloned().collect::<Vec<_>>() {
        let chosen = n.choices.entry(choice.key.clone()).or_default();
        let shown = choice.options.iter().find(|(a, _)| a == chosen).map_or("", |(_, name)| name.as_str()).to_string();
        egui::ComboBox::from_label(&choice.description).selected_text(shown).show_ui(ui, |ui| {
            for (arch, name) in &choice.options {
                ui.selectable_value(chosen, arch.clone(), name);
            }
        });
    }
}

/// The "Where to start" radio list and the chosen map's description.
fn start_picker(ui: &mut egui::Ui, n: &mut NewChar) {
    heading(ui, "Where to start");
    for m in n.maps.clone() {
        ui.radio_value(&mut n.map, Some(m.arch.clone()), &m.name);
    }
    if let Some(m) = n.maps.iter().find(|m| Some(&m.arch) == n.map.as_ref()) {
        ui.label(egui::RichText::new(&m.description).small());
    }
}

/// Column 2: descriptions, stat changes, choices, where to start.
fn about_column(ui: &mut egui::Ui, n: &mut NewChar) {
    egui::ScrollArea::vertical().id_salt("about").show(ui, |ui| {
        for arch in [n.race.clone(), n.class.clone()].into_iter().flatten() {
            let Some(info) = n.infos.get(&arch) else { continue };
            heading(ui, &info.name);
            ui.label(&info.message);
            let changes = stat_changes(info);
            if !changes.is_empty() {
                ui.label(egui::RichText::new(changes).italics());
            }
            ui.add_space(8.0);
        }
        choice_boxes(ui, n);
        ui.add_space(8.0);
        start_picker(ui, n);
    });
}

/// The stats grid: one row per stat with minus/plus buttons and the final value.
fn stats_grid(ui: &mut egui::Ui, n: &mut NewChar) {
    egui::Grid::new("stats").num_columns(4).show(ui, |ui| {
        let finals = n.final_stats();
        for i in 0..7 {
            ui.label(STAT_NAMES[i]);
            ui.horizontal(|ui| {
                if ui.small_button("−").clicked() {
                    n.adjust(i, -1);
                }
                ui.label(n.stats[i].to_string());
                if ui.small_button("+").clicked() {
                    n.adjust(i, 1);
                }
            });
            ui.label("=");
            let f = egui::RichText::new(finals[i].to_string()).strong();
            ui.label(if finals[i] < 1 { f.color(egui::Color32::LIGHT_RED) } else { f });
            ui.end_row();
        }
    });
}

/// Column 3: stats, name, body, Create and Back; returns (create, back) clicked.
fn stats_column(ui: &mut egui::Ui, n: &mut NewChar, body: &mut Body, picture: egui::TextureId, pending: bool) -> (bool, bool) {
    heading(ui, "Stats");
    stats_grid(ui, n);
    ui.label(format!("Points {} / {}", n.used(), n.limits.points));
    if ui.button("Roll the dice").clicked() {
        n.roll(seed());
    }
    ui.add_space(12.0);
    ui.horizontal(|ui| {
        ui.label("Name");
        ui.text_edit_singleline(&mut n.name);
    });
    ui.label(egui::RichText::new("Characters are saved once they have some experience.").small().italics());
    ui.add_space(8.0);
    // The server does not know a body type; the client keeps it per character.
    ui.horizontal(|ui| {
        ui.label("Body");
        ui.radio_value(body, Body::Regular, "Broad");
        ui.radio_value(body, Body::Other, "Slim");
    });
    ui.image((picture, egui::vec2(200.0, 200.0)));
    if let Some(e) = &n.error {
        ui.colored_label(egui::Color32::LIGHT_RED, e);
    }
    let problem = n.problem();
    if let Some(p) = &problem {
        ui.label(egui::RichText::new(p).small());
    }
    let (mut create, mut back) = (false, false);
    ui.horizontal(|ui| {
        create = ui.add_enabled(problem.is_none() && !pending, egui::Button::new("Create")).clicked();
        back = ui.add_enabled(!pending, egui::Button::new("Back")).clicked();
        if pending {
            ui.label("Creating...");
        }
    });
    (create, back)
}

/// Dev screenshots (CROSSFIR3D_SCREEN=newchar[:race]): roll stats and pick the race.
fn dev_autofill(n: &mut NewChar) {
    let dev = std::env::var("CROSSFIR3D_SCREEN").ok().filter(|v| v.starts_with("newchar"));
    if dev.is_some() && n.used() == n.limits.min as u16 * 7 && !n.races.is_empty() {
        n.roll(seed());
    }
    if let Some(race) = dev.as_deref().and_then(|v| v.strip_prefix("newchar:"))
        && n.race.as_deref() != Some(race)
        && n.races.iter().any(|r| r == race)
    {
        n.race = Some(race.to_string());
        n.refresh_choices();
    }
}
