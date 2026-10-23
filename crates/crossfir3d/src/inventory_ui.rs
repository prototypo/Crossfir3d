//! Full-screen inventory (I): carried items with open bags, the ground, spells, stats.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiTextureHandle};
use cfproto::{ClientMsg, Inventory, Item, Spell, Stats};
use crossfir3d::modes::Mode;
use crossfir3d::slots::{default_slots_path, save_slots, Slot};

use crate::{Inv, KeyMode, LastAction, PlayerStats, Session, Slots, Spells};

/// Height of the item and spell pictures in the lists.
const ICON: f32 = 22.0;

/// A picture's size in the list: ICON high, as wide as its shape needs.
fn icon_size(px: Vec2) -> egui::Vec2 {
    egui::vec2(ICON * px.x / px.y.max(1.0), ICON)
}

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
    // The chosen bag may have been closed or dropped since: then to the player.
    let open = |c: u32| inv.get(c).is_some_and(|b| b.open());
    let to = container.filter(|&c| c != item.tag && open(c)).unwrap_or(inv.player);
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

/// Everything the inventory screen reads or changes in a frame.
#[derive(SystemParam)]
pub struct InventoryParams<'w, 's> {
    contexts: EguiContexts<'w, 's>,
    keys: Res<'w, ButtonInput<KeyCode>>,
    time: Res<'w, Time>,
    mode: Res<'w, KeyMode>,
    inv: Res<'w, Inv>,
    spells: Res<'w, Spells>,
    stats: Res<'w, PlayerStats>,
    slots: ResMut<'w, Slots>,
    sel: ResMut<'w, InvSel>,
    last: ResMut<'w, LastAction>,
    textures: Res<'w, crate::world::Textures>,
    session: Option<NonSend<'w, Session>>,
}

/// What the three columns list this frame.
struct Lists<'a> {
    inv: &'a Inventory,
    items: Vec<(usize, u32)>,
    ground: Vec<u32>,
    spells: Vec<&'a Spell>,
}

/// Each item's and spell's picture, as egui textures and sizes.
type Icons = std::collections::HashMap<u32, (egui::TextureId, egui::Vec2)>;

/// Arrow keys move the cursor, then it is kept on an entry that is left.
/// Arrow keys only: WASD letters are actions here (D = drop).
fn move_cursor(keys: &ButtonInput<KeyCode>, sel: &mut InvSel, lens: [usize; 3]) {
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
}

/// The item under the cursor; in the items column an open one becomes the container for G.
fn selected_item<'a>(lists: &Lists<'a>, sel: &mut InvSel) -> Option<&'a Item> {
    let tag = match sel.column {
        0 => lists.items.get(sel.row).map(|(_, t)| *t),
        1 => lists.ground.get(sel.row).copied(),
        _ => None,
    };
    let item = tag.and_then(|t| lists.inv.get(t));
    if sel.column == 0 {
        sel.container = item.filter(|i| i.open()).map(|i| i.tag);
    }
    item
}

/// The server commands for E apply, D drop, G get and X examine on the selected item.
fn item_commands(keys: &ButtonInput<KeyCode>, inv: &Inventory, item: Option<&Item>, container: Option<u32>) -> Vec<ClientMsg> {
    let mut send = Vec::new();
    if let Some(item) = item
        && keys.just_pressed(KeyCode::KeyE)
    {
        send.push(ClientMsg::Apply(item.tag));
    }
    // A page entry only pages (E); dropping, getting or examining it does nothing.
    if let Some(item) = item.filter(|i| !crossfir3d::messages::is_page_entry(i.tag)) {
        if keys.just_pressed(KeyCode::KeyD) && item.location != 0 {
            send.push(ClientMsg::Move { to: 0, tag: item.tag, count: item.nrof });
        }
        if keys.just_pressed(KeyCode::KeyG) {
            if let Some(to) = get_target(inv, item, container) {
                send.push(ClientMsg::Move { to, tag: item.tag, count: item.nrof });
            }
        }
        if keys.just_pressed(KeyCode::KeyX) {
            send.push(ClientMsg::Examine(item.tag));
        }
    }
    send
}

/// What a quick slot would hold: the selected spell, or the selected item.
fn slot_under_cursor(lists: &Lists, sel: &InvSel, item: Option<&Item>) -> Option<Slot> {
    match sel.column {
        2 => lists.spells.get(sel.row).map(|s| Slot::Spell(s.name.clone())),
        _ => item.map(|it| Slot::Item(it.name.clone())),
    }
}

/// Keys 1-4 put `slot` on a quick slot and save the slots.
fn quick_slot(keys: &ButtonInput<KeyCode>, slot: Option<Slot>, slots: &mut Slots, session: Option<&Session>) {
    let digit = [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4].iter().position(|k| keys.just_pressed(*k));
    let Some(i) = digit else { return };
    if let (Some(slot), Some(session)) = (slot, session) {
        slots.0 .0[i] = Some(slot);
        if let Err(e) = save_slots(&default_slots_path(), session.login.character(), &slots.0) {
            warn!("could not save quick slots: {e}");
        }
    }
}

/// Each item's and spell's picture, as egui textures (faces still on the way: none).
fn build_icons(contexts: &mut EguiContexts, textures: &crate::world::Textures, lists: &Lists) -> Icons {
    let faces = lists.items.iter().map(|(_, t)| *t).chain(lists.ground.iter().copied()).filter_map(|t| lists.inv.get(t)).map(|i| i.face);
    let mut icons = Icons::new();
    for face in faces.chain(lists.spells.iter().map(|s| s.face)) {
        if let Some((h, px)) = textures.picture(face as u16) {
            icons.entry(face).or_insert_with(|| (contexts.add_image(EguiTextureHandle::Weak(h.id())), icon_size(px)));
        }
    }
    icons
}

/// A row: the picture (or a gap of its width), then the selectable text.
fn row(ui: &mut egui::Ui, icons: &Icons, face: u32, selected: bool, text: egui::RichText) -> egui::Response {
    ui.horizontal(|ui| {
        match icons.get(&face) {
            Some(&(id, size)) => {
                ui.add(egui::Image::new((id, size)));
            }
            None => ui.add_space(ICON),
        }
        ui.selectable_label(selected, text)
    })
    .inner
}

/// A gold bold label that titles a column.
fn heading(ui: &mut egui::Ui, t: &str) {
    ui.label(egui::RichText::new(t).color(GOLD).strong());
}

/// A heading and its rows (index, face, selected, text); the index of the clicked row.
fn column(ui: &mut egui::Ui, title: &str, icons: &Icons, rows: impl Iterator<Item = (usize, u32, bool, egui::RichText)>) -> Option<usize> {
    heading(ui, title);
    let mut clicked = None;
    for (i, face, selected, text) in rows {
        if row(ui, icons, face, selected, text).clicked() {
            clicked = Some(i);
        }
    }
    clicked
}

/// An item's text, indented by `depth`, red if cursed.
fn item_rich(item: &Item, depth: usize) -> egui::RichText {
    let text = format!("{}{}", "    ".repeat(depth), item_text(item));
    if item.cursed() { egui::RichText::new(text).color(egui::Color32::LIGHT_RED) } else { egui::RichText::new(text) }
}

/// A spell's text: name, level and what it costs.
fn spell_text(s: &Spell) -> String {
    let cost = if s.grace > 0 { format!("{} grace", s.grace) } else { format!("{} mana", s.mana) };
    format!("{} (lvl {}, {cost})", s.name, s.level)
}

/// The stats block under the spells.
fn stats_block(ui: &mut egui::Ui, st: &Stats) {
    ui.add_space(16.0);
    heading(ui, "STATS");
    ui.label(format!(
        "Str {}  Dex {}  Con {}  Wis {}\nCha {}  Int {}  Pow {}\nLevel {}  Exp {}",
        st.str_, st.dex, st.con, st.wis, st.cha, st.int, st.pow, st.level, st.exp
    ));
}

/// The items and ground columns; the (column, row) clicked, if any.
fn draw_item_columns(cols: &mut [egui::Ui], lists: &Lists, icons: &Icons, sel: &InvSel) -> Option<(usize, usize)> {
    let at = |c: usize, i: usize| sel.column == c && sel.row == i;
    // `i` counts every entry, also the ones skipped for a tag missing from the inventory.
    let items = lists.items.iter().enumerate().filter_map(|(i, (depth, tag))| {
        let item = lists.inv.get(*tag)?;
        Some((i, item.face, at(0, i), item_rich(item, *depth)))
    });
    let items = column(&mut cols[0], "ITEMS", icons, items).map(|i| (0, i));
    let ground = lists.ground.iter().enumerate().filter_map(|(i, tag)| {
        let item = lists.inv.get(*tag)?;
        Some((i, item.face, at(1, i), egui::RichText::new(item_text(item))))
    });
    let ground = column(&mut cols[1], "ON THE GROUND", icons, ground).map(|i| (1, i));
    ground.or(items)
}

/// The three columns and the stats; the (column, row) clicked, if any.
fn draw_columns(cols: &mut [egui::Ui], lists: &Lists, icons: &Icons, sel: &InvSel, stats: &Stats) -> Option<(usize, usize)> {
    let clicked = draw_item_columns(cols, lists, icons, sel);
    let spells = lists.spells.iter().enumerate().map(|(i, s)| (i, s.face, sel.column == 2 && sel.row == i, egui::RichText::new(spell_text(s))));
    let spells = column(&mut cols[2], "SPELLS", icons, spells).map(|i| (2, i));
    stats_block(&mut cols[2], stats);
    spells.or(clicked)
}

/// The inventory window; the (column, row) clicked, if any.
fn draw_window(ctx: &egui::Context, lists: &Lists, icons: &Icons, sel: &InvSel, stats: &Stats) -> Option<(usize, usize)> {
    let frame = egui::Frame::new()
        .fill(egui::Color32::from_rgba_unmultiplied(20, 22, 30, 238))
        .stroke(egui::Stroke::new(2.0, GOLD))
        .corner_radius(8.0)
        .inner_margin(16.0);
    let screen = ctx.content_rect().shrink(24.0);
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
            ui.columns(3, |cols| draw_columns(cols, lists, icons, sel, stats))
        })
        .and_then(|r| r.inner)
        .flatten()
}

pub fn inventory_screen(mut p: InventoryParams) -> Result {
    if p.mode.0 != Mode::Inventory {
        return Ok(());
    }
    let inv = &p.inv.0;
    let ground = inv.ground().iter().map(|i| i.tag).filter(|&t| t != inv.player).collect(); // not you
    let lists = Lists { inv, items: item_rows(inv), ground, spells: p.spells.0.all() };

    move_cursor(&p.keys, &mut p.sel, [lists.items.len(), lists.ground.len(), lists.spells.len()]);
    let item = selected_item(&lists, &mut p.sel);
    let send = item_commands(&p.keys, inv, item, p.sel.container);
    let slot = slot_under_cursor(&lists, &p.sel, item);
    quick_slot(&p.keys, slot, &mut p.slots, p.session.as_deref());
    if let (false, Some(session)) = (send.is_empty(), &p.session) {
        p.last.0 = Some(p.time.elapsed_secs());
        session.send(send);
    }

    let icons = build_icons(&mut p.contexts, &p.textures, &lists);
    let ctx = p.contexts.ctx_mut()?;
    let clicked = draw_window(ctx, &lists, &icons, &p.sel, &p.stats.0);
    if let Some((column, row)) = clicked {
        (p.sel.column, p.sel.row) = (column, row);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icons_keep_their_shape_in_the_row() {
        assert_eq!(icon_size(Vec2::new(32.0, 32.0)), egui::vec2(ICON, ICON));
        assert_eq!(icon_size(Vec2::new(64.0, 32.0)), egui::vec2(2.0 * ICON, ICON)); // a wide picture stays wide
        assert_eq!(icon_size(Vec2::new(32.0, 96.0)), egui::vec2(ICON / 3.0, ICON));
    }

    #[test]
    fn get_ignores_a_bag_that_is_no_longer_open() {
        let mut inv = inv();
        let apple_tag = 30;
        inv.apply_upd(10, &[cfproto::ItemChange::Flags(0)]); // the bag is closed
        let apple = inv.get(apple_tag).unwrap();
        assert_eq!(get_target(&inv, apple, Some(10)), Some(1));
        assert_eq!(get_target(&inv, apple, Some(99)), Some(1)); // gone
    }

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
