//! The settings screen: Escape in game, or the "Settings" button on the login form.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use crossfir3d::modes::Mode;
use crossfir3d::settings::{AntiAlias, FrameRate, Quality, Settings, LOG_FONT_SIZES};

use crate::graphics::CurrentSettings;
use crate::ui::{overlay_window, GOLD};
use crate::{AppState, KeyMode};

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

/// Everything `settings_screen` reads or changes in a frame.
#[derive(SystemParam)]
pub struct SettingsParams<'w, 's> {
    contexts: EguiContexts<'w, 's>,
    keys: Res<'w, ButtonInput<KeyCode>>,
    state: Res<'w, State<AppState>>,
    mode: ResMut<'w, KeyMode>,
    open: ResMut<'w, SettingsOpen>,
    current: ResMut<'w, CurrentSettings>,
}

/// Draws the settings window (in game or on the login screen), applies the changes and closes it on request.
pub fn settings_screen(mut p: SettingsParams) -> Result {
    let in_game = *p.state.get() == AppState::InGame;
    let showing = if in_game { p.mode.0 == Mode::Settings } else { p.open.0 };
    if !showing {
        return Ok(());
    }
    // In game, Escape is handled by the key modes; on the login screen, here.
    if !in_game && p.keys.just_pressed(KeyCode::Escape) {
        p.open.0 = false;
        return Ok(());
    }

    let mut s = p.current.0.clone();
    let close = settings_window(p.contexts.ctx_mut()?, &mut s);
    apply_settings(s, close, in_game, &mut p);
    Ok(())
}

/// Shows the full-screen settings window with its title, the scrolling grid and the buttons; returns whether Close was clicked.
fn settings_window(ctx: &egui::Context, s: &mut Settings) -> bool {
    let screen = ctx.content_rect().shrink(24.0);
    let mut close = false;
    overlay_window("settings", screen, 20.0)
        .show(ctx, |ui| {
            ui.set_min_size(screen.size() - egui::vec2(40.0, 40.0));
            ui.spacing_mut().slider_width = 260.0;
            ui.label(egui::RichText::new("SETTINGS").color(GOLD).strong().size(18.0));
            ui.add_space(12.0);
            // Scrolls in a short window, so the buttons below always show.
            egui::ScrollArea::vertical().max_height(ui.available_height() - 60.0).show(ui, |ui| settings_grid(ui, s));
            ui.add_space(16.0);
            close = bottom_buttons(ui, s);
        });
    close
}

/// Draws the two-column grid with every setting row.
fn settings_grid(ui: &mut egui::Ui, s: &mut Settings) {
    egui::Grid::new("settings-grid").num_columns(2).spacing([24.0, 12.0]).show(ui, |ui| {
        display_rows(ui, s);
        graphics_rows(ui, s);
        picture_rows(ui, s);
        sound_rows(ui, s);
        log_rows(ui, s);
    });
}

/// Draws the display rows: full screen, render scale and frame rate.
fn display_rows(ui: &mut egui::Ui, s: &mut Settings) {
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
}

/// Draws the graphics rows: ambient occlusion, shadows and anti-aliasing (MSAA is disabled while ambient occlusion is on).
fn graphics_rows(ui: &mut egui::Ui, s: &mut Settings) {
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
}

/// Draws the picture rows: brightness, contrast and gamma sliders.
fn picture_rows(ui: &mut egui::Ui, s: &mut Settings) {
    ui.label("Brightness");
    ui.add(egui::Slider::new(&mut s.brightness, -1.0..=1.0));
    ui.end_row();
    ui.label("Contrast");
    ui.add(egui::Slider::new(&mut s.contrast, 0.5..=1.5));
    ui.end_row();
    ui.label("Gamma");
    ui.add(egui::Slider::new(&mut s.gamma, 0.5..=2.0));
    ui.end_row();
}

/// Draws the sound row: the music checkbox.
fn sound_rows(ui: &mut egui::Ui, s: &mut Settings) {
    ui.label("Music");
    ui.checkbox(&mut s.music, "");
    ui.end_row();
}

/// Draws the log rows: text size, background opacity and the "Show what is under me" ground popup checkbox.
fn log_rows(ui: &mut egui::Ui, s: &mut Settings) {
    ui.label("Log text size");
    ui.add(egui::Slider::new(&mut s.log_font_size, LOG_FONT_SIZES.0..=LOG_FONT_SIZES.1));
    ui.end_row();
    ui.label("Log background");
    ui.add(egui::Slider::new(&mut s.log_opacity, 0.0..=100.0).suffix(" %"));
    ui.end_row();

    ui.label("Show what is under me");
    ui.checkbox(&mut s.ground_popup, "on every new square (V shows it any time)");
    ui.end_row();
}

/// Draws the "Reset to defaults" (keeps the window place) and "Close (Esc)" buttons; returns whether Close was clicked.
fn bottom_buttons(ui: &mut egui::Ui, s: &mut Settings) -> bool {
    let mut close = false;
    ui.horizontal(|ui| {
        if ui.button("Reset to defaults").clicked() {
            *s = Settings { window: s.window, ..Settings::default() };
        }
        if ui.button("Close (Esc)").clicked() {
            close = true;
        }
    });
    close
}

/// Stores the normalized settings if they changed, then closes the screen when asked (back to the world in game, or the login form).
fn apply_settings(s: Settings, close: bool, in_game: bool, p: &mut SettingsParams) {
    let s = s.normalized();
    if s != p.current.0 {
        p.current.0 = s;
    }
    if close {
        if in_game {
            p.mode.0 = Mode::World;
        } else {
            p.open.0 = false;
        }
    }
}
