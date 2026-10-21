//! The in-game overlay: bars, quick slots, parchment box, pop-ups, and the log.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use crossfir3d::slots::{usable, Slot};

use crate::{HudState, Inv, Msgs, PlayerStats, Slots, Spells};

const PARCHMENT: egui::Color32 = egui::Color32::from_rgb(0xf4, 0xec, 0xd8);
const INK: egui::Color32 = egui::Color32::from_rgb(0x2a, 0x22, 0x18);
const SHORT_LOG: usize = 5;
const SLOT_PX: f32 = 56.0;
const TALL_LOG: usize = 20;
/// Food is out of 999; show the bar only when it gets low.
const FOOD_WARN: i16 = 200;

fn bar(ui: &mut egui::Ui, value: i16, max: i16, color: egui::Color32, label: &str) {
    let frac = if max > 0 { (value.max(0) as f32 / max as f32).min(1.0) } else { 0.0 };
    let text = egui::RichText::new(format!("{label} {value}/{max}")).color(egui::Color32::WHITE).small();
    ui.add(egui::ProgressBar::new(frac).fill(color).desired_width(180.0).text(text));
}

fn slot_label(slot: &Option<Slot>) -> &str {
    match slot {
        Some(Slot::Spell(n) | Slot::Item(n)) => n,
        None => "",
    }
}

pub fn hud(
    mut contexts: EguiContexts,
    time: Res<Time>,
    stats: Res<PlayerStats>,
    slots: Res<Slots>,
    inv: Res<Inv>,
    spells: Res<Spells>,
    hud: Res<HudState>,
    mut msgs: ResMut<Msgs>,
) -> Result {
    let ctx = contexts.ctx_mut()?;
    let s = &stats.0;
    msgs.0.expire(time.elapsed_secs());

    egui::Area::new("bars".into()).anchor(egui::Align2::LEFT_TOP, [10.0, 10.0]).show(ctx, |ui| {
        bar(ui, s.hp, s.max_hp, egui::Color32::from_rgb(0xd0, 0x30, 0x30), "HP");
        bar(ui, s.sp, s.max_sp, egui::Color32::from_rgb(0x30, 0x60, 0xf0), "SP");
        bar(ui, s.grace, s.max_grace, egui::Color32::from_rgb(0xc8, 0xc8, 0xc8), "Grace");
        if s.food < FOOD_WARN {
            bar(ui, s.food, 999, egui::Color32::from_rgb(0xc0, 0x80, 0x20), "Food");
        }
    });

    egui::Area::new("slots".into()).anchor(egui::Align2::RIGHT_TOP, [-10.0, 10.0]).show(ctx, |ui| {
        ui.horizontal(|ui| {
            for (i, slot) in slots.0 .0.iter().enumerate() {
                let ready = slot.as_ref().is_some_and(|s| usable(s, &inv.0, &spells.0));
                let gold = egui::Color32::from_rgb(0xf0, 0xc8, 0x3c);
                let stroke = if ready { gold } else { egui::Color32::GRAY };
                egui::Frame::new()
                    .fill(egui::Color32::from_black_alpha(140))
                    .stroke(egui::Stroke::new(2.0, stroke))
                    .corner_radius(24.0)
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        ui.set_width(SLOT_PX);
                        ui.set_height(SLOT_PX);
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new((i + 1).to_string()).strong().color(gold));
                            let color = if ready { egui::Color32::WHITE } else { egui::Color32::GRAY };
                            let name: String = slot_label(slot).chars().take(10).collect();
                            ui.add(egui::Label::new(egui::RichText::new(name).small().color(color)).truncate());
                        });
                    });
            }
        });
    });

    if let Some(text) = &msgs.0.boxed {
        egui::Window::new("parchment")
            .title_bar(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, -40.0])
            .frame(egui::Frame::new().fill(PARCHMENT).corner_radius(6.0).inner_margin(16.0).stroke(egui::Stroke::new(2.0, INK)))
            .max_width(460.0)
            .show(ctx, |ui| {
                ui.label(egui::RichText::new(text).color(INK).size(16.0).family(egui::FontFamily::Proportional));
                ui.add_space(8.0);
                ui.label(egui::RichText::new("E to close").small().italics().color(INK));
            });
    }

    if !msgs.0.popups.is_empty() {
        egui::Area::new("popups".into()).anchor(egui::Align2::CENTER_TOP, [0.0, 20.0]).show(ctx, |ui| {
            for (text, _) in &msgs.0.popups {
                egui::Frame::new().fill(egui::Color32::from_rgba_unmultiplied(120, 20, 20, 200)).inner_margin(6.0).show(ui, |ui| {
                    ui.label(egui::RichText::new(text).color(egui::Color32::WHITE));
                });
            }
        });
    }

    let lines = if hud.tall_log { TALL_LOG } else { SHORT_LOG };
    egui::Area::new("log".into()).anchor(egui::Align2::LEFT_BOTTOM, [10.0, -10.0]).show(ctx, |ui| {
        egui::Frame::new().fill(egui::Color32::from_black_alpha(120)).inner_margin(6.0).show(ui, |ui| {
            ui.set_max_width(420.0);
            let show = |ui: &mut egui::Ui| {
                let skip = if hud.tall_log { 0 } else { msgs.0.log.len().saturating_sub(lines) };
                for line in msgs.0.log.iter().skip(skip) {
                    ui.label(egui::RichText::new(line).small().color(egui::Color32::from_gray(230)));
                }
            };
            if hud.tall_log {
                egui::ScrollArea::vertical().max_height(lines as f32 * 16.0).stick_to_bottom(true).show(ui, show);
            } else {
                show(ui);
            }
        });
    });
    Ok(())
}
