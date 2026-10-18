//! Combat feedback the server does not draw: a red flash and the HP lost when the
//! player is hit, and "hit", "miss", or "killed" over the creature in front of them,
//! which flashes red when hit. The protocol sends no creature health or damage, so
//! there are no creature health bars or damage numbers.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use crate::characters3d::OwnCharacter;
use crate::input::Facing;
use crate::world::{CameraRig, Card, TileVisual};
use crate::{PlayerStats, View, ViewSize};

/// The server's attack messages (MSG_TYPE_ATTACK) and the subtypes shown here.
pub const MSG_ATTACK: u8 = 14;
const DID_HIT: u8 = 1;
const DID_KILL: u8 = 4;
const MISS: u8 = 9;

/// Seconds a floating word or number lasts, and the screen flash.
const FLOAT_SECS: f32 = 1.2;
const FLASH_SECS: f32 = 0.4;
/// Seconds a hit creature stays red.
const TINT_SECS: f32 = 0.2;

#[derive(Resource, Default)]
pub struct CombatFx {
    /// Attack message subtypes not yet shown (pushed by the message handler).
    pub attacks: Vec<u8>,
    flash: Option<f32>,
    floats: Vec<Float>,
}

struct Float {
    at: Vec3,
    text: String,
    colour: egui::Color32,
    born: f32,
}

/// Facing 1 = north .. 8 = northwest to the square that way.
fn ahead(facing: u8) -> (i32, i32) {
    const STEPS: [(i32, i32); 8] = [(0, -1), (1, -1), (1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1)];
    STEPS[(facing.clamp(1, 8) - 1) as usize]
}

#[allow(clippy::too_many_arguments)]
pub fn watch(
    time: Res<Time>,
    stats: Res<PlayerStats>,
    facing: Res<Facing>,
    view: Res<View>,
    size: Res<ViewSize>,
    mut fx: ResMut<CombatFx>,
    mut last_hp: Local<Option<i16>>,
    mut tinted: Local<Vec<(Handle<StandardMaterial>, Color, f32)>>,
    own: Query<&Transform, With<OwnCharacter>>,
    cards: Query<(&TileVisual, &MeshMaterial3d<StandardMaterial>), With<Card>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let now = time.elapsed_secs();
    let Ok(me) = own.single().map(|t| t.translation) else { return };
    // Hit: HP fell (not the jump from 0 at login).
    let hp = stats.0.hp;
    if let Some(before) = *last_hp
        && hp < before
        && before > 0
    {
        fx.flash = Some(now);
        let text = format!("-{}", before - hp);
        fx.floats.push(Float { at: me + Vec3::Y * 1.25, text, colour: egui::Color32::from_rgb(0xff, 0x40, 0x30), born: now });
    }
    *last_hp = Some(hp);

    let (dx, dy) = ahead(facing.0);
    let target = me + Vec3::new(dx as f32, 1.0, dy as f32);
    for subtype in std::mem::take(&mut fx.attacks) {
        let (text, colour) = match subtype {
            DID_HIT => ("hit", egui::Color32::from_rgb(0xff, 0xe0, 0x60)),
            MISS => ("miss", egui::Color32::from_gray(0xc0)),
            DID_KILL => ("killed!", egui::Color32::from_rgb(0xff, 0xa0, 0x20)),
            _ => continue,
        };
        fx.floats.push(Float { at: target, text: text.into(), colour, born: now });
        if subtype == DID_HIT {
            let (px, py) = view.0.world((size.0 / 2) as i8, (size.1 / 2) as i8);
            for (tv, mat) in &cards {
                if (tv.x, tv.y) == (px + dx, py + dy) && tv.layer >= 6 && !tinted.iter().any(|(h, ..)| h == &mat.0)
                    && let Some(mut m) = materials.get_mut(&mat.0)
                {
                    tinted.push((mat.0.clone(), m.base_color, now));
                    m.base_color = Color::srgb(1.0, 0.25, 0.2);
                }
            }
        }
    }
    tinted.retain(|(handle, colour, at)| {
        let keep = now - at < TINT_SECS;
        if !keep && let Some(mut m) = materials.get_mut(handle) {
            m.base_color = *colour;
        }
        keep
    });
    fx.floats.retain(|f| now - f.born < FLOAT_SECS);
}

pub fn draw(
    mut contexts: EguiContexts,
    time: Res<Time>,
    fx: Res<CombatFx>,
    camera: Single<(&Camera, &GlobalTransform), With<CameraRig>>,
) -> Result {
    let ctx = contexts.ctx_mut()?;
    let now = time.elapsed_secs();
    let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("combat_fx")));
    if let Some(at) = fx.flash
        && now - at < FLASH_SECS
    {
        let alpha = (0.35 * (1.0 - (now - at) / FLASH_SECS) * 255.0) as u8;
        painter.rect_filled(ctx.content_rect(), 0.0, egui::Color32::from_rgba_unmultiplied(0xc0, 0, 0, alpha));
    }
    let (camera, eye) = camera.into_inner();
    // The world is drawn to an image at the render scale, not at the window's size.
    let screen = ctx.content_rect();
    let Some(drawn) = camera.logical_viewport_size() else { return Ok(()) };
    let scale = screen.width() / drawn.x;
    for f in &fx.floats {
        let age = (now - f.born) / FLOAT_SECS;
        let Ok(p) = camera.world_to_viewport(eye, f.at + Vec3::Y * age * 0.3) else { continue };
        let p = screen.min + egui::vec2(p.x, p.y) * scale;
        let [r, g, b, _] = f.colour.to_array();
        let colour = egui::Color32::from_rgba_unmultiplied(r, g, b, ((1.0 - age) * 255.0) as u8);
        let font = egui::FontId::proportional(26.0);
        // A dark copy behind, so the words read on sky and stone alike.
        painter.text(egui::pos2(p.x + 2.0, p.y + 2.0), egui::Align2::CENTER_CENTER, &f.text, font.clone(), egui::Color32::from_black_alpha(colour.a()));
        painter.text(egui::pos2(p.x, p.y), egui::Align2::CENTER_CENTER, &f.text, font, colour);
    }
    Ok(())
}
