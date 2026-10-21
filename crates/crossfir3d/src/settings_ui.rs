//! The settings screen: Escape in game, or the "Settings" button on the login form.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use crossfir3d::modes::Mode;
use crossfir3d::settings::{AntiAlias, FrameRate, Quality, Settings, LOG_FONT_SIZES};

use crate::graphics::CurrentSettings;
use crate::{AppState, KeyMode};

const GOLD: egui::Color32 = egui::Color32::from_rgb(0xc9, 0xa2, 0x4a);

/// Settings opened from the login form (in game, `KeyMode::Settings` is used).
#[derive(Resource, Default)]
pub struct SettingsOpen(pub bool);

fn quality_row(ui: &mut egui::Ui, label: &str, value: &mut Quality) {
    ui.label(label);
    ui.horizontal(|ui| {
        for (q, name) in [(Quality::Off, "Off"), (Quality::Low, "Low"), (Quality::Medium, "Medium"), (Quality::High, "High")] {
            ui.selectable_value(value, q, name);
        }
    });
    ui.end_row();
}

pub fn settings_screen(
    mut contexts: EguiContexts,
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<AppState>>,
    mut mode: ResMut<KeyMode>,
    mut open: ResMut<SettingsOpen>,
    mut current: ResMut<CurrentSettings>,
) -> Result {
    let in_game = *state.get() == AppState::InGame;
    let showing = if in_game { mode.0 == Mode::Settings } else { open.0 };
    if !showing {
        return Ok(());
    }
    // In game, Escape is handled by the key modes; on the login screen, here.
    if !in_game && keys.just_pressed(KeyCode::Escape) {
        open.0 = false;
        return Ok(());
    }

    let mut s = current.0.clone();
    let mut close = false;
    let ctx = contexts.ctx_mut()?;
    let frame = egui::Frame::new()
        .fill(egui::Color32::from_rgba_unmultiplied(20, 22, 30, 238))
        .stroke(egui::Stroke::new(2.0, GOLD))
        .corner_radius(8.0)
        .inner_margin(20.0);
    let screen = ctx.content_rect().shrink(24.0);
    egui::Window::new("settings")
        .title_bar(false)
        .resizable(false)
        .movable(false)
        .fixed_rect(screen)
        .frame(frame)
        .show(ctx, |ui| {
            ui.set_min_size(screen.size() - egui::vec2(40.0, 40.0));
            ui.spacing_mut().slider_width = 260.0;
            ui.label(egui::RichText::new("SETTINGS").color(GOLD).strong().size(18.0));
            ui.add_space(12.0);
            egui::Grid::new("settings-grid").num_columns(2).spacing([24.0, 12.0]).show(ui, |ui| {
                ui.label("Full screen");
                ui.checkbox(&mut s.fullscreen, "");
                ui.end_row();

                ui.label("Render scale");
                ui.horizontal(|ui| {
                    for (v, name) in [(0.5, "50%"), (0.75, "75%"), (1.0, "100%")] {
                        ui.selectable_value(&mut s.render_scale, v, name);
                    }
                });
                ui.end_row();

                ui.label("Frame rate");
                ui.horizontal(|ui| {
                    for (f, name) in [
                        (FrameRate::MatchScreen, "Match screen"),
                        (FrameRate::Cap30, "30"),
                        (FrameRate::Cap60, "60"),
                        (FrameRate::Cap120, "120"),
                        (FrameRate::Unlimited, "Unlimited"),
                    ] {
                        ui.selectable_value(&mut s.frame_rate, f, name);
                    }
                });
                ui.end_row();

                quality_row(ui, "Ambient occlusion", &mut s.ambient_occlusion);
                quality_row(ui, "Shadows", &mut s.shadows);

                ui.label("Anti-aliasing");
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut s.anti_alias, AntiAlias::Off, "Off");
                    let msaa_ok = s.ambient_occlusion == Quality::Off;
                    ui.add_enabled_ui(msaa_ok, |ui| {
                        ui.selectable_value(&mut s.anti_alias, AntiAlias::Msaa4, "MSAA 4x");
                    });
                    ui.selectable_value(&mut s.anti_alias, AntiAlias::Taa, "TAA");
                    if !msaa_ok {
                        ui.label(egui::RichText::new("MSAA needs ambient occlusion off").small().italics());
                    }
                });
                ui.end_row();

                ui.label("Brightness");
                ui.add(egui::Slider::new(&mut s.brightness, -1.0..=1.0));
                ui.end_row();
                ui.label("Contrast");
                ui.add(egui::Slider::new(&mut s.contrast, 0.5..=1.5));
                ui.end_row();
                ui.label("Gamma");
                ui.add(egui::Slider::new(&mut s.gamma, 0.5..=2.0));
                ui.end_row();

                ui.label("Music");
                ui.checkbox(&mut s.music, "");
                ui.end_row();

                ui.label("Log text size");
                ui.add(egui::Slider::new(&mut s.log_font_size, LOG_FONT_SIZES.0..=LOG_FONT_SIZES.1));
                ui.end_row();
                ui.label("Log background");
                ui.add(egui::Slider::new(&mut s.log_opacity, 0.0..=100.0).suffix(" %"));
                ui.end_row();

                ui.label("Show what is under me");
                ui.checkbox(&mut s.ground_popup, "on every new square (B shows it any time)");
                ui.end_row();
            });
            ui.add_space(16.0);
            ui.horizontal(|ui| {
                if ui.button("Reset to defaults").clicked() {
                    s = Settings { window: s.window, ..Settings::default() };
                }
                if ui.button("Close (Esc)").clicked() {
                    close = true;
                }
            });
        });

    let s = s.normalized();
    if s != current.0 {
        current.0 = s;
    }
    if close {
        if in_game {
            mode.0 = Mode::World;
        } else {
            open.0 = false;
        }
    }
    Ok(())
}
