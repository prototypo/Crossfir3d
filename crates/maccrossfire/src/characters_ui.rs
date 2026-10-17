//! After account login: the character list and the new-character screen.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use maccrossfire::login::LoginState;
use maccrossfire::newchar::{stat_code, STAT_NAMES};

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

pub fn characters_screen(
    mut contexts: EguiContexts,
    keys: Res<ButtonInput<KeyCode>>,
    mut status: ResMut<Status>,
    mut auto: Local<bool>,
    mut commands: Commands,
    session: Option<NonSendMut<Session>>,
) -> Result {
    let Some(mut session) = session else { return Ok(()) };
    if status.0 == "Connecting..." {
        status.0.clear(); // connected: the list or form is showing now
    }
    let ctx = contexts.ctx_mut()?;
    let screen = ctx.content_rect().shrink(24.0);
    // Developer screenshots: MACCROSSFIRE_SCREEN=newchar opens the new-character
    // screen and rolls once.
    if !*auto && std::env::var("MACCROSSFIRE_SCREEN").as_deref() == Ok("newchar") && *session.login.state() == LoginState::Characters {
        *auto = true;
        let out = session.login.start_creating();
        session.send(out);
    }
    match session.login.state().clone() {
        LoginState::Characters => {
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
                    if !status.0.is_empty() {
                        ui.label(&status.0);
                    }
                    ui.add_space(12.0);
                    if chars.is_empty() {
                        ui.label("No characters yet.");
                    }
                    for (i, c) in chars.iter().enumerate() {
                        let text = format!("{} — {} {}, level {}", c.name, c.race, c.class, c.level);
                        let row = ui.selectable_label(session.login.selected == Some(i), egui::RichText::new(text).size(15.0));
                        if row.clicked() {
                            session.login.selected = Some(i);
                        }
                        if row.double_clicked() {
                            out = session.login.play(&c.name);
                        }
                    }
                    ui.add_space(16.0);
                    ui.horizontal(|ui| {
                        let chosen = session.login.selected.and_then(|i| chars.get(i));
                        let play = ui.add_enabled(chosen.is_some(), egui::Button::new("Play"));
                        if let (Some(c), true) = (chosen, play.clicked() || keys.just_pressed(KeyCode::Enter)) {
                            out = session.login.play(&c.name);
                        }
                        if ui.button("New character").clicked() {
                            out = session.login.start_creating();
                        }
                        back = ui.button("Back").clicked();
                    });
                });
            if !out.is_empty() {
                status.0.clear();
            }
            session.send(out);
            if back {
                // No account logout in the protocol: close the connection.
                commands.queue(|world: &mut World| crate::end_session(world, String::new()));
            }
        }
        LoginState::Creating => new_character(ctx, screen, &mut session),
        _ => {}
    }
    Ok(())
}

fn new_character(ctx: &egui::Context, screen: egui::Rect, session: &mut Session) {
    let mut out = Vec::new();
    let mut back = false;
    let session_pending = session.login.create_pending();
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
                // Column 1: race and class lists.
                heading(&mut cols[0], "Race");
                egui::ScrollArea::vertical().id_salt("races").max_height(220.0).show(&mut cols[0], |ui| {
                    for r in n.races.clone() {
                        let name = n.infos.get(&r).map_or(r.clone(), |i| i.name.clone());
                        if ui.selectable_label(n.race.as_deref() == Some(&r), name).clicked() {
                            n.race = Some(r);
                            n.refresh_choices();
                        }
                    }
                });
                heading(&mut cols[0], "Class");
                egui::ScrollArea::vertical().id_salt("classes").max_height(220.0).show(&mut cols[0], |ui| {
                    for c in n.classes.clone() {
                        let name = n.infos.get(&c).map_or(c.clone(), |i| i.name.clone());
                        if ui.selectable_label(n.class.as_deref() == Some(&c), name).clicked() {
                            n.class = Some(c);
                            n.refresh_choices();
                        }
                    }
                });

                // Column 2: descriptions, stat changes, choices, where to start.
                egui::ScrollArea::vertical().id_salt("about").show(&mut cols[1], |ui| {
                    for arch in [n.race.clone(), n.class.clone()].into_iter().flatten() {
                        let Some(info) = n.infos.get(&arch) else { continue };
                        heading(ui, &info.name);
                        ui.label(&info.message);
                        let changes: Vec<String> = (0..7)
                            .filter_map(|i| {
                                let v: i16 = info.stats.iter().filter(|(c, _)| *c == stat_code(i)).map(|(_, v)| *v).sum();
                                (v != 0).then(|| format!("{} {:+}", STAT_NAMES[i], v))
                            })
                            .collect();
                        if !changes.is_empty() {
                            ui.label(egui::RichText::new(changes.join("  ")).italics());
                        }
                        ui.add_space(8.0);
                    }
                    for choice in n.offered_choices().into_iter().cloned().collect::<Vec<_>>() {
                        let chosen = n.choices.entry(choice.key.clone()).or_default();
                        let shown = choice.options.iter().find(|(a, _)| a == chosen).map_or("", |(_, name)| name.as_str()).to_string();
                        egui::ComboBox::from_label(&choice.description).selected_text(shown).show_ui(ui, |ui| {
                            for (arch, name) in &choice.options {
                                ui.selectable_value(chosen, arch.clone(), name);
                            }
                        });
                    }
                    ui.add_space(8.0);
                    heading(ui, "Where to start");
                    for m in n.maps.clone() {
                        ui.radio_value(&mut n.map, Some(m.arch.clone()), &m.name);
                    }
                    if let Some(m) = n.maps.iter().find(|m| Some(&m.arch) == n.map.as_ref()) {
                        ui.label(egui::RichText::new(&m.description).small());
                    }
                });

                // Column 3: stats, name, create.
                let ui = &mut cols[2];
                heading(ui, "Stats");
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
                // Room for the turning 3D preview (M4).
                ui.allocate_space(egui::vec2(ui.available_width(), 80.0));
                if let Some(e) = &n.error {
                    ui.colored_label(egui::Color32::LIGHT_RED, e);
                }
                let problem = n.problem();
                if let Some(p) = &problem {
                    ui.label(egui::RichText::new(p).small());
                }
                ui.horizontal(|ui| {
                    let waiting = session_pending;
                    if ui.add_enabled(problem.is_none() && !waiting, egui::Button::new("Create")).clicked() {
                        out.push(());
                    }
                    back = ui.add_enabled(!waiting, egui::Button::new("Back")).clicked();
                    if waiting {
                        ui.label("Creating...");
                    }
                });
            });
        });
    if std::env::var("MACCROSSFIRE_SCREEN").as_deref() == Ok("newchar") && n.used() == n.limits.min as u16 * 7 && !n.races.is_empty() {
        n.roll(seed());
    }
    if !out.is_empty() {
        let msgs = session.login.create();
        session.send(msgs);
    }
    if back {
        session.login.back_to_list();
    }
}
