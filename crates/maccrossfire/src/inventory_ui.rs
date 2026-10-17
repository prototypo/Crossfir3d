//! Full-screen inventory (I): carried items with open bags, the ground, spells, stats.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use cfproto::{ClientMsg, Inventory, Item};
use maccrossfire::modes::Mode;
use maccrossfire::slots::{default_slots_path, save_slots, Slot};

use crate::{Inv, KeyMode, LastAction, PlayerStats, Session, Slots, Spells};

const GOLD: egui::Color32 = egui::Color32::from_rgb(0xc9, 0xa2, 0x4a);

/// Carried items, each open container followed by its contents one level deeper:
/// (depth, tag).
pub fn item_rows(inv: &Inventory) -> Vec<(usize, u32)> {
    fn walk(inv: &Inventory, location: u32, depth: usize, out: &mut Vec<(usize, u32)>) {
        for item in inv.children(location) {
            out.push((depth, item.tag));
            // Guard against a container that claims to hold itself.
            if item.open() && item.tag != location && depth < 8 {
                walk(inv, item.tag, depth + 1, out);
            }
        }
    }
    let mut out = Vec::new();
    walk(inv, inv.player, 0, &mut out);
    out
}

/// Where G sends `item`: into the chosen open container, else to the player. None if
/// that is where it already is.
pub fn get_target(inv: &Inventory, item: &Item, container: Option<u32>) -> Option<u32> {
    let to = container.filter(|&c| c != item.tag).unwrap_or(inv.player);
    (to != item.location).then_some(to)
}

#[derive(Resource, Default)]
pub struct InvSel {
    /// 0 items, 1 ground, 2 spells
    pub column: usize,
    pub row: usize,
    /// Last open container selected in the items column: G puts things into it.
    pub container: Option<u32>,
}

fn item_text(item: &Item) -> String {
    let mut s = if item.nrof > 1 { format!("{} {}", item.nrof, item.plural) } else { item.name.clone() };
    if item.applied() {
        s += " (applied)";
    }
    if item.open() {
        s += " (open)";
    }
    if item.unpaid() {
        s += " (unpaid)";
    }
    if item.locked() {
        s += " (locked)";
    }
    s
}

#[allow(clippy::too_many_arguments)]
pub fn inventory_screen(
    mut contexts: EguiContexts,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mode: Res<KeyMode>,
    inv: Res<Inv>,
    spells: Res<Spells>,
    stats: Res<PlayerStats>,
    mut slots: ResMut<Slots>,
    mut sel: ResMut<InvSel>,
    mut last: ResMut<LastAction>,
    session: Option<NonSend<Session>>,
) -> Result {
    if mode.0 != Mode::Inventory {
        return Ok(());
    }
    let inv = &inv.0;
    let items = item_rows(inv);
    let ground: Vec<u32> = inv.ground().iter().map(|i| i.tag).collect();
    let spell_list = spells.0.all();
    let lens = [items.len(), ground.len(), spell_list.len()];

    // Arrow keys only: WASD letters are actions here (D = drop).
    if keys.just_pressed(KeyCode::ArrowLeft) {
        sel.column = sel.column.saturating_sub(1);
    }
    if keys.just_pressed(KeyCode::ArrowRight) {
        sel.column = (sel.column + 1).min(2);
    }
    if keys.just_pressed(KeyCode::ArrowUp) {
        sel.row = sel.row.saturating_sub(1);
    }
    if keys.just_pressed(KeyCode::ArrowDown) {
        sel.row += 1;
    }
    // A deleted or moved entry: stay on the nearest one that is left.
    sel.row = sel.row.min(lens[sel.column].saturating_sub(1));

    let selected_tag = match sel.column {
        0 => items.get(sel.row).map(|(_, t)| *t),
        1 => ground.get(sel.row).copied(),
        _ => None,
    };
    let selected_item = selected_tag.and_then(|t| inv.get(t));
    if sel.column == 0 {
        sel.container = selected_item.filter(|i| i.open()).map(|i| i.tag);
    }

    let mut send: Vec<ClientMsg> = Vec::new();
    if let Some(item) = selected_item {
        if keys.just_pressed(KeyCode::KeyE) {
            send.push(ClientMsg::Apply(item.tag));
        }
        if keys.just_pressed(KeyCode::KeyD) && item.location != 0 {
            send.push(ClientMsg::Move { to: 0, tag: item.tag, count: item.nrof });
        }
        if keys.just_pressed(KeyCode::KeyG) {
            if let Some(to) = get_target(inv, item, sel.container) {
                send.push(ClientMsg::Move { to, tag: item.tag, count: item.nrof });
            }
        }
        if keys.just_pressed(KeyCode::KeyX) {
            send.push(ClientMsg::Examine(item.tag));
        }
    }
    let digit = [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4].iter().position(|k| keys.just_pressed(*k));
    if let Some(i) = digit {
        let slot = match sel.column {
            2 => spell_list.get(sel.row).map(|s| Slot::Spell(s.name.clone())),
            _ => selected_item.map(|it| Slot::Item(it.name.clone())),
        };
        if let (Some(slot), Some(session)) = (slot, &session) {
            slots.0 .0[i] = Some(slot);
            if let Err(e) = save_slots(&default_slots_path(), session.login.character(), &slots.0) {
                warn!("could not save quick slots: {e}");
            }
        }
    }
    if let (false, Some(session)) = (send.is_empty(), &session) {
        last.0 = Some(time.elapsed_secs());
        session.send(send);
    }

    let ctx = contexts.ctx_mut()?;
    let frame = egui::Frame::new()
        .fill(egui::Color32::from_rgba_unmultiplied(20, 22, 30, 238))
        .stroke(egui::Stroke::new(2.0, GOLD))
        .corner_radius(8.0)
        .inner_margin(16.0);
    let screen = ctx.content_rect().shrink(24.0);
    let mut clicked: Option<(usize, usize)> = None;
    egui::Window::new("inventory")
        .title_bar(false)
        .resizable(false)
        .movable(false)
        .fixed_rect(screen)
        .frame(frame)
        .show(ctx, |ui| {
            ui.set_min_size(screen.size() - egui::vec2(32.0, 32.0));
            ui.label(egui::RichText::new("I or Esc: close · arrows: select · E apply · D drop · G get · X examine · 1-4 quick slot").small());
            ui.add_space(8.0);
            ui.columns(3, |cols| {
                let heading = |ui: &mut egui::Ui, t: &str| {
                    ui.label(egui::RichText::new(t).color(GOLD).strong());
                };
                heading(&mut cols[0], "ITEMS");
                for (row, (depth, tag)) in items.iter().enumerate() {
                    let Some(item) = inv.get(*tag) else { continue };
                    let text = format!("{}{}", "    ".repeat(*depth), item_text(item));
                    let rich = if item.cursed() { egui::RichText::new(text).color(egui::Color32::LIGHT_RED) } else { egui::RichText::new(text) };
                    if cols[0].selectable_label(sel.column == 0 && sel.row == row, rich).clicked() {
                        clicked = Some((0, row));
                    }
                }
                heading(&mut cols[1], "ON THE GROUND");
                for (row, tag) in ground.iter().enumerate() {
                    let Some(item) = inv.get(*tag) else { continue };
                    if cols[1].selectable_label(sel.column == 1 && sel.row == row, item_text(item)).clicked() {
                        clicked = Some((1, row));
                    }
                }
                heading(&mut cols[2], "SPELLS");
                for (row, s) in spell_list.iter().enumerate() {
                    let cost = if s.grace > 0 { format!("{} grace", s.grace) } else { format!("{} mana", s.mana) };
                    let text = format!("{} (lvl {}, {cost})", s.name, s.level);
                    if cols[2].selectable_label(sel.column == 2 && sel.row == row, text).clicked() {
                        clicked = Some((2, row));
                    }
                }
                cols[2].add_space(16.0);
                heading(&mut cols[2], "STATS");
                let st = &stats.0;
                cols[2].label(format!(
                    "Str {}  Dex {}  Con {}  Wis {}\nCha {}  Int {}  Pow {}\nLevel {}  Exp {}",
                    st.str_, st.dex, st.con, st.wis, st.cha, st.int, st.pow, st.level, st.exp
                ));
            });
        });
    if let Some((column, row)) = clicked {
        (sel.column, sel.row) = (column, row);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(tag: u32, location: u32, name: &str, flags: u32) -> Item {
        Item {
            tag,
            location,
            flags,
            weight: 0,
            face: 0,
            name: name.into(),
            plural: name.into(),
            anim: 0,
            anim_speed: 0,
            nrof: 1,
            kind: 1,
        }
    }

    fn inv() -> Inventory {
        let mut inv = Inventory::default();
        inv.player = 1;
        inv.apply_item2(1, vec![item(10, 1, "bag", 0x2000), item(11, 1, "closed sack", 0), item(12, 1, "sword", 0)]);
        inv.apply_item2(10, vec![item(20, 10, "gem", 0)]);
        inv.apply_item2(11, vec![item(21, 11, "hidden", 0)]);
        inv.apply_item2(0, vec![item(30, 0, "apple", 0)]);
        inv
    }

    #[test]
    fn rows_show_open_bag_contents_only() {
        assert_eq!(item_rows(&inv()), vec![(0, 10), (1, 20), (0, 11), (0, 12)]);
    }

    #[test]
    fn get_goes_to_player_or_chosen_bag() {
        let inv = inv();
        let apple = inv.get(30).unwrap();
        assert_eq!(get_target(&inv, apple, None), Some(1));
        assert_eq!(get_target(&inv, apple, Some(10)), Some(10));
        let gem = inv.get(20).unwrap();
        assert_eq!(get_target(&inv, gem, None), Some(1)); // out of the bag
        assert_eq!(get_target(&inv, gem, Some(10)), None); // already there
        let bag = inv.get(10).unwrap();
        assert_eq!(get_target(&inv, bag, Some(10)), None); // not into itself
        assert_eq!(get_target(&inv, inv.get(12).unwrap(), None), None); // already carried
    }
}
