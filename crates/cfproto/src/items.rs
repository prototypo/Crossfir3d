//! Items (`item2`, `upditem`) and the client's view of the inventory as a tree.
//! Layouts follow `add_object_to_socklist` and `esrv_update_item` in socket/item.cpp.

use std::collections::HashMap;

use crate::server::{ParseError, Reader};

/// "singular\0plural" (plural optional) to (singular, plural).
fn split_name(raw: &[u8]) -> (String, String) {
    let text = String::from_utf8_lossy(raw);
    let mut parts = text.splitn(2, '\0');
    let one = parts.next().unwrap_or("").to_string();
    let many = parts.next().filter(|p| !p.is_empty()).map_or_else(|| one.clone(), String::from);
    (one, many)
}

fn lname(r: &mut Reader) -> Result<(String, String), ParseError> {
    let len = r.u8()? as usize;
    Ok(split_name(r.take(len)?))
}

#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub tag: u32,
    /// 0 = the ground under the player; otherwise the tag of the player or a container.
    pub location: u32,
    pub flags: u32,
    pub weight: i32,
    pub face: u32,
    pub name: String,
    pub plural: String,
    pub anim: u16,
    pub anim_speed: u8,
    pub nrof: u32,
    pub kind: u16,
}

impl Item {
    pub fn applied(&self) -> bool {
        self.flags & 0x000F != 0
    }
    pub fn cursed(&self) -> bool {
        self.flags & (0x0800 | 0x1000) != 0
    }
    pub fn locked(&self) -> bool {
        self.flags & 0x8000 != 0
    }
    pub fn open(&self) -> bool {
        self.flags & 0x2000 != 0
    }
    pub fn unpaid(&self) -> bool {
        self.flags & 0x0200 != 0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ItemChange {
    Location(u32),
    Flags(u32),
    Weight(i32),
    Face(u32),
    Name(String, String),
    Anim(u16),
    AnimSpeed(u8),
    Nrof(u32),
}

/// item2 data: location, then items until the end.
pub fn parse_item2(data: &[u8]) -> Result<(u32, Vec<Item>), ParseError> {
    let mut r = Reader(data);
    let location = r.u32()?;
    let mut items = Vec::new();
    while !r.0.is_empty() {
        let (tag, flags, weight, face) = (r.u32()?, r.u32()?, r.u32()? as i32, r.u32()?);
        let (name, plural) = lname(&mut r)?;
        let (anim, anim_speed, nrof, kind) = (r.u16()?, r.u8()?, r.u32()?, r.u16()?);
        items.push(Item { tag, location, flags, weight, face, name, plural, anim, anim_speed, nrof, kind });
    }
    Ok((location, items))
}

/// upditem data: flag byte, tag, then the flagged fields in bit order 0x01..0x80.
pub fn parse_upditem(data: &[u8]) -> Result<(u32, Vec<ItemChange>), ParseError> {
    let mut r = Reader(data);
    let (flags, tag) = (r.u8()?, r.u32()?);
    let mut changes = Vec::new();
    for bit in 0..8 {
        if flags & (1 << bit) != 0 {
            changes.push(upd_field(&mut r, bit)?);
        }
    }
    Ok((tag, changes))
}

/// Reads the upditem field for flag bit `bit` (0..8) by its width.
fn upd_field(r: &mut Reader, bit: u8) -> Result<ItemChange, ParseError> {
    match bit {
        4 => lname(r).map(|(one, many)| ItemChange::Name(one, many)),
        5 | 6 => upd_narrow(r, bit),
        _ => upd_word(r, bit),
    }
}

/// Reads the four-byte upditem fields (bits 0, 1, 2, 3 and 7).
fn upd_word(r: &mut Reader, bit: u8) -> Result<ItemChange, ParseError> {
    let v = r.u32()?;
    Ok(match bit {
        0 => ItemChange::Location(v),
        1 => ItemChange::Flags(v),
        2 => ItemChange::Weight(v as i32),
        3 => ItemChange::Face(v),
        _ => ItemChange::Nrof(v),
    })
}

/// Reads the short upditem fields (bit 5 anim, bit 6 anim speed).
fn upd_narrow(r: &mut Reader, bit: u8) -> Result<ItemChange, ParseError> {
    Ok(match bit {
        5 => ItemChange::Anim(r.u16()?),
        _ => ItemChange::AnimSpeed(r.u8()?),
    })
}

#[derive(Default)]
pub struct Inventory {
    pub player: u32,
    items: HashMap<u32, Item>,
}

impl Inventory {
    /// Known tags are replaced (the server resends an item when it moves).
    pub fn apply_item2(&mut self, location: u32, items: Vec<Item>) {
        for mut item in items {
            item.location = location;
            self.items.insert(item.tag, item);
        }
    }

    pub fn apply_upd(&mut self, tag: u32, changes: &[ItemChange]) {
        let Some(item) = self.items.get_mut(&tag) else { return };
        for change in changes {
            match change.clone() {
                ItemChange::Location(l) => item.location = l,
                ItemChange::Flags(f) => item.flags = f,
                ItemChange::Weight(w) => item.weight = w,
                ItemChange::Face(f) => item.face = f,
                ItemChange::Name(one, many) => (item.name, item.plural) = (one, many),
                ItemChange::Anim(a) => item.anim = a,
                ItemChange::AnimSpeed(s) => item.anim_speed = s,
                ItemChange::Nrof(n) => item.nrof = n,
            }
        }
    }

    pub fn delete(&mut self, tags: &[u32]) {
        for tag in tags {
            self.items.remove(tag);
        }
    }

    /// Removes everything inside `location`, and inside those, and so on. 0 = ground.
    pub fn delete_inside(&mut self, location: u32) {
        let inside: Vec<u32> = self.items.values().filter(|i| i.location == location).map(|i| i.tag).collect();
        for tag in inside {
            self.items.remove(&tag);
            // The player can be listed on the ground too; clearing the ground must not
            // clear what they carry.
            if tag != location && tag != self.player {
                self.delete_inside(tag);
            }
        }
    }

    pub fn get(&self, tag: u32) -> Option<&Item> {
        self.items.get(&tag)
    }

    /// Items directly inside `location`, sorted by kind, then name.
    pub fn children(&self, location: u32) -> Vec<&Item> {
        let mut out: Vec<&Item> = self.items.values().filter(|i| i.location == location).collect();
        out.sort_by(|a, b| (a.kind, &a.name, a.tag).cmp(&(b.kind, &b.name, b.tag)));
        out
    }

    pub fn carried(&self) -> Vec<&Item> {
        self.children(self.player)
    }

    pub fn ground(&self) -> Vec<&Item> {
        self.children(0)
    }

    /// First carried item, or item in an open carried container, with this name.
    pub fn find_by_name(&self, name: &str) -> Option<&Item> {
        let carried = self.carried();
        let in_open_bags = carried.iter().filter(|c| c.open()).flat_map(|c| self.children(c.tag));
        carried.iter().copied().chain(in_open_bags).find(|i| i.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One item's bytes in item2 layout.
    fn item_bytes(tag: u32, flags: u32, name: &[u8], nrof: u32, kind: u16) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(&tag.to_be_bytes());
        b.extend_from_slice(&flags.to_be_bytes());
        b.extend_from_slice(&500i32.to_be_bytes());
        b.extend_from_slice(&7u32.to_be_bytes());
        b.push(name.len() as u8);
        b.extend_from_slice(name);
        b.extend_from_slice(&0u16.to_be_bytes());
        b.push(0);
        b.extend_from_slice(&nrof.to_be_bytes());
        b.extend_from_slice(&kind.to_be_bytes());
        b
    }

    fn item(tag: u32, location: u32, name: &str, kind: u16) -> Item {
        Item {
            tag,
            location,
            flags: 0,
            weight: 0,
            face: 0,
            name: name.into(),
            plural: name.into(),
            anim: 0,
            anim_speed: 0,
            nrof: 1,
            kind,
        }
    }

    #[test]
    fn parse_item2_two_items() {
        let mut d = 42u32.to_be_bytes().to_vec();
        d.extend(item_bytes(5, 0x0001, b"sword\0swords", 1, 100));
        d.extend(item_bytes(6, 0, b"potion\0potions", 3, 200));
        let (loc, items) = parse_item2(&d).unwrap();
        assert_eq!(loc, 42);
        assert_eq!(items.len(), 2);
        assert_eq!((items[0].tag, items[0].name.as_str(), items[0].plural.as_str()), (5, "sword", "swords"));
        assert!(items[0].applied());
        assert_eq!((items[1].nrof, items[1].kind, items[1].weight, items[1].face), (3, 200, 500, 7));
    }

    #[test]
    fn item2_name_without_plural() {
        let mut d = 0u32.to_be_bytes().to_vec();
        d.extend(item_bytes(5, 0, b"rock", 1, 1));
        let (_, items) = parse_item2(&d).unwrap();
        assert_eq!(items[0].plural, "rock");
    }

    #[test]
    fn item2_cut_short_is_error() {
        let mut d = 0u32.to_be_bytes().to_vec();
        let mut one = item_bytes(5, 0, b"rock", 1, 1);
        one.truncate(10);
        d.extend(one);
        assert!(parse_item2(&d).is_err());
    }

    #[test]
    fn upditem_nrof_and_flags() {
        let mut d = vec![0x82];
        d.extend_from_slice(&9u32.to_be_bytes());
        d.extend_from_slice(&0x0800u32.to_be_bytes());
        d.extend_from_slice(&4u32.to_be_bytes());
        let (tag, changes) = parse_upditem(&d).unwrap();
        assert_eq!(tag, 9);
        assert_eq!(changes, vec![ItemChange::Flags(0x0800), ItemChange::Nrof(4)]);
    }

    #[test]
    fn upditem_name_and_location() {
        let mut d = vec![0x11];
        d.extend_from_slice(&9u32.to_be_bytes());
        d.extend_from_slice(&3u32.to_be_bytes());
        d.push(9);
        d.extend_from_slice(b"bag\0bags.");
        let (_, changes) = parse_upditem(&d).unwrap();
        assert_eq!(changes, vec![ItemChange::Location(3), ItemChange::Name("bag".into(), "bags.".into())]);
    }

    /// Frozen copy of the original `parse_upditem`, kept to prove the refactor is equivalent.
    fn old_parse_upditem(data: &[u8]) -> Result<(u32, Vec<ItemChange>), ParseError> {
        let mut r = Reader(data);
        let (flags, tag) = (r.u8()?, r.u32()?);
        let mut changes = Vec::new();
        for bit in 0..8 {
            if flags & (1 << bit) == 0 {
                continue;
            }
            changes.push(match bit {
                0 => ItemChange::Location(r.u32()?),
                1 => ItemChange::Flags(r.u32()?),
                2 => ItemChange::Weight(r.u32()? as i32),
                3 => ItemChange::Face(r.u32()?),
                4 => {
                    let (one, many) = lname(&mut r)?;
                    ItemChange::Name(one, many)
                }
                5 => ItemChange::Anim(r.u16()?),
                6 => ItemChange::AnimSpeed(r.u8()?),
                _ => ItemChange::Nrof(r.u32()?),
            });
        }
        Ok((tag, changes))
    }

    /// An lname field: length byte, then a name with an optional plural and sometimes a non-UTF8 byte.
    fn upd_lname(next: &mut impl FnMut() -> u32) -> Vec<u8> {
        let mut name: Vec<u8> = (0..next() % 8).map(|_| b'a' + (next() % 26) as u8).collect();
        if next() % 2 == 0 {
            name.push(0);
            name.extend((0..next() % 8).map(|_| b'a' + (next() % 26) as u8));
        }
        if next() % 4 == 0 {
            name.push(0xFF);
        }
        name.insert(0, name.len() as u8);
        name
    }

    /// A well-formed upditem packet for `flags`, with random field bytes and names.
    fn upd_packet(flags: u8, next: &mut impl FnMut() -> u32) -> Vec<u8> {
        let mut d = vec![flags];
        d.extend((0..4).map(|_| next() as u8));
        for bit in 0..8 {
            match bit {
                _ if flags & (1 << bit) == 0 => {}
                4 => d.extend(upd_lname(next)),
                _ => d.extend((0..[4, 4, 4, 4, 0, 2, 1, 4][bit]).map(|_| next() as u8)),
            }
        }
        d
    }

    #[test]
    fn parse_upditem_matches_old() {
        let mut seed = 0x2545_F491_4F6C_DD1Du64;
        let mut next = move || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (seed >> 33) as u32
        };
        let mut cases: Vec<Vec<u8>> = vec![
            vec![0x82, 0, 0, 0, 9, 0, 0, 8, 0, 0, 0, 0, 4],
            vec![0x11, 0, 0, 0, 9, 0, 0, 0, 3, 9, b'b', b'a', b'g', 0, b'b', b'a', b'g', b's', b'.'],
        ];
        for _ in 0..8000 {
            let n = next() % 40;
            cases.push((0..n).map(|_| next() as u8).collect());
        }
        for i in 0..12000u32 {
            let flags = if i < 256 { i as u8 } else { next() as u8 };
            let mut d = upd_packet(flags, &mut next);
            if i % 3 == 0 {
                d.truncate(next() as usize % (d.len() + 1));
            }
            cases.push(d);
        }
        for d in &cases {
            assert_eq!(parse_upditem(d), old_parse_upditem(d), "input {d:?}");
        }
    }

    #[test]
    fn inventory_moves_item_into_container() {
        let mut inv = Inventory { player: 1, ..Default::default() };
        inv.apply_item2(1, vec![item(10, 1, "bag", 50), item(11, 1, "apple", 60)]);
        inv.apply_item2(10, vec![item(11, 10, "apple", 60)]);
        assert_eq!(inv.carried().len(), 1);
        assert_eq!(inv.children(10)[0].tag, 11);
    }

    #[test]
    fn upd_location_moves_item() {
        let mut inv = Inventory { player: 1, ..Default::default() };
        inv.apply_item2(1, vec![item(11, 1, "apple", 60)]);
        inv.apply_upd(11, &[ItemChange::Location(0), ItemChange::Nrof(2)]);
        assert_eq!(inv.ground()[0].nrof, 2);
        assert!(inv.carried().is_empty());
    }

    #[test]
    fn clearing_the_ground_keeps_the_players_items() {
        let mut inv = Inventory { player: 1, ..Default::default() };
        inv.apply_item2(1, vec![item(10, 1, "bow", 151)]);
        inv.apply_item2(0, vec![item(1, 0, "Gatey", 0), item(5, 0, "apple", 60)]);
        inv.delete_inside(0);
        assert!(inv.get(5).is_none());
        assert_eq!(inv.carried().len(), 1);
    }

    #[test]
    fn delete_inside_is_recursive() {
        let mut inv = Inventory { player: 1, ..Default::default() };
        inv.apply_item2(1, vec![item(10, 1, "bag", 50)]);
        inv.apply_item2(10, vec![item(20, 10, "pouch", 50)]);
        inv.apply_item2(20, vec![item(30, 20, "gem", 70)]);
        inv.delete_inside(10);
        assert!(inv.get(20).is_none() && inv.get(30).is_none());
        assert!(inv.get(10).is_some());
    }

    #[test]
    fn children_sorted() {
        let mut inv = Inventory { player: 1, ..Default::default() };
        inv.apply_item2(1, vec![item(1, 1, "zebra", 5), item(2, 1, "apple", 9), item(3, 1, "bread", 5)]);
        let names: Vec<_> = inv.carried().iter().map(|i| i.name.clone()).collect();
        assert_eq!(names, ["bread", "zebra", "apple"]);
    }

    #[test]
    fn find_by_name_looks_in_open_container() {
        let mut inv = Inventory { player: 1, ..Default::default() };
        let mut bag = item(10, 1, "bag", 50);
        bag.flags = 0x2000;
        inv.apply_item2(1, vec![bag]);
        inv.apply_item2(10, vec![item(11, 10, "potion of healing", 60)]);
        inv.apply_item2(0, vec![item(12, 0, "potion of healing", 60)]);
        assert_eq!(inv.find_by_name("potion of healing").unwrap().tag, 11);
        assert!(inv.find_by_name("sword").is_none());
    }

    #[test]
    fn delete_unknown_tag_is_noop() {
        let mut inv = Inventory::default();
        inv.delete(&[99]);
        inv.delete_inside(99);
        inv.apply_upd(99, &[ItemChange::Nrof(1)]);
        assert!(inv.get(99).is_none());
    }
}
