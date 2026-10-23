//! Combat feedback the server does not draw: a red flash and the HP lost when the
//! player is hit, and "hit", "miss", or "killed" over the creature in front of them,
//! which flashes red when hit (a picture card, or a 3D creature). The protocol sends no creature health or damage, so
//! there are no creature health bars or damage numbers.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use crate::characters3d::{Character, OwnCharacter};
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

type Tinted = Vec<(Handle<StandardMaterial>, Color, f32)>;

#[derive(SystemParam)]
pub struct WatchParams<'w, 's> {
    time: Res<'w, Time>,
    stats: Res<'w, PlayerStats>,
    facing: Res<'w, Facing>,
    view: Res<'w, View>,
    size: Res<'w, ViewSize>,
    own: Query<'w, 's, &'static Transform, With<OwnCharacter>>,
    things: Query<'w, 's, (Entity, &'static TileVisual, Has<Card>, Has<Character>)>,
    children: Query<'w, 's, &'static Children>,
    meshes: Query<'w, 's, &'static MeshMaterial3d<StandardMaterial>>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
}

pub fn watch(mut w: WatchParams, mut fx: ResMut<CombatFx>, mut last_hp: Local<Option<i16>>, mut tinted: Local<Tinted>) {
    let now = w.time.elapsed_secs();
    let Ok(me) = w.own.single().map(|t| t.translation) else { return };
    let hp = w.stats.0.hp;
    flash_on_hp_drop(&mut fx, *last_hp, hp, me, now);
    *last_hp = Some(hp);
    show_attacks(&mut w, &mut fx, &mut tinted, me, now);
    restore_tints(&mut tinted, &mut w.materials, now);
    fx.floats.retain(|f| now - f.born < FLOAT_SECS);
}

/// Flashes the screen and floats the HP lost when HP fell.
fn flash_on_hp_drop(fx: &mut CombatFx, last_hp: Option<i16>, hp: i16, me: Vec3, now: f32) {
    // Hit: HP fell (not the jump from 0 at login).
    if let Some(before) = last_hp
        && hp < before
        && before > 0
    {
        fx.flash = Some(now);
        let text = format!("-{}", before - hp);
        fx.floats.push(Float { at: me + Vec3::Y * 1.25, text, colour: egui::Color32::from_rgb(0xff, 0x40, 0x30), born: now });
    }
}

/// Floats "hit", "miss" or "killed!" over the square ahead for each pending attack, tinting a hit target.
fn show_attacks(w: &mut WatchParams, fx: &mut CombatFx, tinted: &mut Tinted, me: Vec3, now: f32) {
    let (dx, dy) = ahead(w.facing.0);
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
            tint_hit(w, tinted, (dx, dy), now);
        }
    }
}

/// Turns the materials of the creature one square ahead red, remembering the originals.
fn tint_hit(w: &mut WatchParams, tinted: &mut Tinted, (dx, dy): (i32, i32), now: f32) {
    let (px, py) = w.view.0.world((w.size.0 / 2) as i8, (w.size.1 / 2) as i8);
    // A picture card, or every part of a 3D creature (each has its own materials).
    let hit = w.things.iter().filter(|(_, tv, ..)| (tv.x, tv.y) == (px + dx, py + dy) && tv.layer >= 6);
    let parts = hit.flat_map(|(e, _, card, character)| {
        let own = card.then_some(e).into_iter();
        own.chain(character.then(|| w.children.iter_descendants(e)).into_iter().flatten())
    });
    for mat in parts.filter_map(|e| w.meshes.get(e).ok()) {
        if !tinted.iter().any(|(h, ..)| h == &mat.0)
            && let Some(mut m) = w.materials.get_mut(&mat.0)
        {
            tinted.push((mat.0.clone(), m.base_color, now));
            m.base_color = Color::srgb(1.0, 0.25, 0.2);
        }
    }
}

/// Puts back the original colour of every tinted material older than TINT_SECS.
fn restore_tints(tinted: &mut Tinted, materials: &mut Assets<StandardMaterial>, now: f32) {
    tinted.retain(|(handle, colour, at)| {
        let keep = now - at < TINT_SECS;
        if !keep && let Some(mut m) = materials.get_mut(handle) {
            m.base_color = *colour;
        }
        keep
    });
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
