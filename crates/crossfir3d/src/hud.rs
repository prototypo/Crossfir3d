//! The in-game overlay: bars, quick slots, parchment box, pop-ups, and the log.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use cfproto::Stats;
use crossfir3d::messages::Messages;
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

/// Server text with its styles (`[b]`, `[color=...]`, `[fixed]`, ...) as one wrapped
/// piece of text: bold shows in the `strong` colour.
/// The font family of bold text (egui's own fonts have no bold).
const BOLD: &str = "bold";

/// Adds the bold font to egui, once.
pub fn load_fonts(mut contexts: EguiContexts, mut done: Local<bool>) -> Result {
    if *done {
        return Ok(());
    }
    let mut fonts = egui::FontDefinitions::default();
    let data = egui::FontData::from_static(include_bytes!("../../../assets/fonts/Ubuntu-Bold.ttf"));
    fonts.font_data.insert(BOLD.into(), data.into());
    let mut family = vec![BOLD.to_string()];
    family.extend(fonts.families[&egui::FontFamily::Proportional].iter().cloned());
    fonts.families.insert(egui::FontFamily::Name(BOLD.into()), family);
    contexts.ctx_mut()?.set_fonts(fonts);
    *done = true;
    Ok(())
}

/// Turns marked-up text into a coloured, styled egui text job.
fn styled(text: &str, size: f32, base: egui::Color32, strong: egui::Color32) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    for span in crossfir3d::markup::spans(text) {
        let color = match (span.color, span.bold) {
            (Some([r, g, b]), _) => egui::Color32::from_rgb(r, g, b),
            (None, true) => strong,
            (None, false) => base,
        };
        let font_id = match (span.fixed, span.bold) {
            (true, _) => egui::FontId::monospace(size),
            (false, true) => egui::FontId::new(size, egui::FontFamily::Name(BOLD.into())),
            (false, false) => egui::FontId::proportional(size),
        };
        let underline = if span.underline { egui::Stroke::new(1.0, color) } else { egui::Stroke::NONE };
        job.append(&span.text, 0.0, egui::TextFormat { font_id, color, italics: span.italic, underline, ..Default::default() });
    }
    job
}

/// Everything `hud` reads or changes in a frame.
#[derive(SystemParam)]
pub struct HudParams<'w> {
    time: Res<'w, Time>,
    stats: Res<'w, PlayerStats>,
    slots: Res<'w, Slots>,
    inv: Res<'w, Inv>,
    spells: Res<'w, Spells>,
    hud: Res<'w, HudState>,
    msgs: ResMut<'w, Msgs>,
    settings: Res<'w, crate::graphics::CurrentSettings>,
}

/// Draws the HP, SP, grace and (when low) food bars at the top left.
fn draw_bars(ctx: &egui::Context, s: &Stats) {
    egui::Area::new("bars".into()).anchor(egui::Align2::LEFT_TOP, [10.0, 10.0]).show(ctx, |ui| {
        bar(ui, s.hp, s.max_hp, egui::Color32::from_rgb(0xd0, 0x30, 0x30), "HP");
        bar(ui, s.sp, s.max_sp, egui::Color32::from_rgb(0x30, 0x60, 0xf0), "SP");
        bar(ui, s.grace, s.max_grace, egui::Color32::from_rgb(0xc8, 0xc8, 0xc8), "Grace");
        if s.food < FOOD_WARN {
            bar(ui, s.food, 999, egui::Color32::from_rgb(0xc0, 0x80, 0x20), "Food");
        }
    });
}

/// Draws one quick slot: its number and a shortened name, gold-ringed when usable.
fn draw_slot(ui: &mut egui::Ui, i: usize, slot: &Option<Slot>, ready: bool) {
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

/// Draws the row of quick slots at the top right.
fn draw_slots(ctx: &egui::Context, p: &HudParams) {
    egui::Area::new("slots".into()).anchor(egui::Align2::RIGHT_TOP, [-10.0, 10.0]).show(ctx, |ui| {
        ui.horizontal(|ui| {
            for (i, slot) in p.slots.0 .0.iter().enumerate() {
                let ready = slot.as_ref().is_some_and(|s| usable(s, &p.inv.0, &p.spells.0));
                draw_slot(ui, i, slot, ready);
            }
        });
    });
}

/// Draws the parchment box with the book text.
fn draw_parchment(ctx: &egui::Context, text: &str) {
    egui::Window::new("parchment")
        .title_bar(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, -40.0])
        .frame(egui::Frame::new().fill(PARCHMENT).corner_radius(6.0).inner_margin(16.0).stroke(egui::Stroke::new(2.0, INK)))
        .max_width(460.0)
        .show(ctx, |ui| {
            ui.label(styled(text, 16.0, INK, egui::Color32::BLACK));
            ui.add_space(8.0);
            ui.label(egui::RichText::new("E to close").small().italics().color(INK));
        });
}

/// Draws the red pop-up messages at the top centre.
fn draw_popups(ctx: &egui::Context, msgs: &Messages) {
    egui::Area::new("popups".into()).anchor(egui::Align2::CENTER_TOP, [0.0, 20.0]).show(ctx, |ui| {
        for (text, _) in &msgs.popups {
            egui::Frame::new().fill(egui::Color32::from_rgba_unmultiplied(120, 20, 20, 200)).inner_margin(6.0).show(ui, |ui| {
                ui.label(styled(text, 14.0, egui::Color32::WHITE, egui::Color32::WHITE));
            });
        }
    });
}

/// Draws the message log at the bottom left.
fn draw_log(ctx: &egui::Context, msgs: &Messages, tall: bool, size: f32, fill: egui::Color32) {
    let lines = if tall { TALL_LOG } else { SHORT_LOG };
    egui::Area::new("log".into()).anchor(egui::Align2::LEFT_BOTTOM, [10.0, -10.0]).show(ctx, |ui| {
        egui::Frame::new().fill(fill).inner_margin(6.0).show(ui, |ui| {
            ui.set_max_width(42.0 * size);
            // One log row: the font's row height plus the gap between labels.
            let row = ui.fonts_mut(|f| f.row_height(&egui::FontId::proportional(size))) + ui.spacing().item_spacing.y;
            let height = lines as f32 * row;
            // One message can hold many lines (the message of the day), so count lines, not messages.
            // Empty messages are skipped; a blank line inside a message is a space, so it takes a full row.
            let rows: Vec<&str> = msgs.log.iter().map(|m| m.trim_end_matches('\n')).filter(|m| !m.is_empty())
                .flat_map(|m| m.split('\n')).map(|l| if l.is_empty() { " " } else { l }).collect();
            let skip = if tall { 0 } else { rows.len().saturating_sub(lines) };
            let show = |ui: &mut egui::Ui| {
                // Always the full `lines` rows, so R shows a change even with few lines; text at the bottom.
                // ponytail: a line too long for the box wraps, making the short box a row taller, the tall one scroll.
                ui.add_space((height - (rows.len() - skip) as f32 * row).max(0.0));
                for line in &rows[skip..] {
                    ui.label(styled(line, size, egui::Color32::from_gray(230), egui::Color32::WHITE));
                }
            };
            if tall {
                // min_scrolled_height: else the area cuts itself to the space left on screen last frame.
                egui::ScrollArea::vertical().max_height(height).min_scrolled_height(height).stick_to_bottom(true).show(ui, show);
            } else {
                show(ui);
            }
        });
    });
}

/// Draws the bars, slots, messages and log of the in-game HUD.
pub fn hud(mut contexts: EguiContexts, mut p: HudParams) -> Result {
    let ctx = contexts.ctx_mut()?;
    p.msgs.0.expire(p.time.elapsed_secs());
    draw_bars(ctx, &p.stats.0);
    draw_slots(ctx, &p);
    if let Some(text) = &p.msgs.0.boxed {
        draw_parchment(ctx, text);
    }
    if !p.msgs.0.popups.is_empty() {
        draw_popups(ctx, &p.msgs.0);
    }
    // The log box grows with its text: 420 px wide at the default size 10.
    let size = p.settings.0.log_font_size;
    let fill = egui::Color32::from_black_alpha((p.settings.0.log_opacity / 100.0 * 255.0).round() as u8);
    draw_log(ctx, &p.msgs.0, p.hud.tall_log, size, fill);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bold_text_uses_the_bold_font() {
        let job = styled("a [b]b[/b] [fixed][b]c", 12.0, INK, INK);
        let families: Vec<_> = job.sections.iter().map(|s| s.format.font_id.family.clone()).collect();
        assert_eq!(families[0], egui::FontFamily::Proportional);
        assert_eq!(families[1], egui::FontFamily::Name(BOLD.into()));
        assert_eq!(families[3], egui::FontFamily::Monospace);
    }
}
