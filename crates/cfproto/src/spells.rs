//! `addspell` / `updspell` / `delspell` (with `setup spellmon 1`) and the spell book.
//! Layout follows `append_spell` in the server's socket/request.cpp.

use std::collections::HashMap;

use crate::server::{ParseError, Reader};

#[derive(Clone, Debug, PartialEq)]
pub struct Spell {
    pub tag: u32,
    pub level: i16,
    pub casting_time: i16,
    pub mana: i16,
    pub grace: i16,
    pub damage: i16,
    pub skill: u8,
    pub path: u32,
    pub face: u32,
    pub name: String,
    pub message: String,
}

/// Parses an addspell packet into the list of spells the server sends.
pub fn parse_addspell(data: &[u8]) -> Result<Vec<Spell>, ParseError> {
    let mut r = Reader(data);
    let mut out = Vec::new();
    while !r.0.is_empty() {
        out.push(read_spell(&mut r)?);
    }
    Ok(out)
}

/// Reads one spell record; the fields are read in wire order.
fn read_spell(r: &mut Reader) -> Result<Spell, ParseError> {
    let tag = r.u32()?;
    let [level, casting_time, mana, grace, damage] = read_costs(r)?;
    let (skill, path, face) = (r.u8()?, r.u32()?, r.u32()?);
    let len = r.u8()? as usize;
    let name = read_text(r, len)?;
    let len = r.u16()? as usize;
    let message = read_text(r, len)?;
    Ok(Spell { tag, level, casting_time, mana, grace, damage, skill, path, face, name, message })
}

/// Reads the five i16 values: level, casting time, mana, grace, damage.
fn read_costs(r: &mut Reader) -> Result<[i16; 5], ParseError> {
    let mut v = [0; 5];
    for x in &mut v {
        *x = r.u16()? as i16;
    }
    Ok(v)
}

/// Reads `len` bytes as text, replacing invalid UTF-8.
fn read_text(r: &mut Reader, len: usize) -> Result<String, ParseError> {
    Ok(String::from_utf8_lossy(r.take(len)?).into_owned())
}

/// (tag, mana, grace, damage); each value only if its flag bit (0x01, 0x02, 0x04) is set.
pub fn parse_updspell(data: &[u8]) -> Result<(u32, Option<i16>, Option<i16>, Option<i16>), ParseError> {
    let mut r = Reader(data);
    let (flags, tag) = (r.u8()?, r.u32()?);
    let mut read = |bit: u8| -> Result<Option<i16>, ParseError> {
        if flags & bit == 0 { Ok(None) } else { Ok(Some(r.u16()? as i16)) }
    };
    let (mana, grace, damage) = (read(0x01)?, read(0x02)?, read(0x04)?);
    Ok((tag, mana, grace, damage))
}

#[derive(Default)]
pub struct SpellBook {
    spells: HashMap<u32, Spell>,
}

impl SpellBook {
    pub fn add(&mut self, spells: Vec<Spell>) {
        self.spells.extend(spells.into_iter().map(|s| (s.tag, s)));
    }

    pub fn update(&mut self, tag: u32, mana: Option<i16>, grace: Option<i16>, damage: Option<i16>) {
        let Some(s) = self.spells.get_mut(&tag) else { return };
        s.mana = mana.unwrap_or(s.mana);
        s.grace = grace.unwrap_or(s.grace);
        s.damage = damage.unwrap_or(s.damage);
    }

    pub fn remove(&mut self, tag: u32) {
        self.spells.remove(&tag);
    }

    pub fn by_name(&self, name: &str) -> Option<&Spell> {
        self.spells.values().find(|s| s.name == name)
    }

    /// Sorted by level, then name.
    pub fn all(&self) -> Vec<&Spell> {
        let mut out: Vec<&Spell> = self.spells.values().collect();
        out.sort_by(|a, b| (a.level, &a.name).cmp(&(b.level, &b.name)));
        out
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn spell_bytes(tag: u32, level: i16, name: &str) -> Vec<u8> {
        let mut b = tag.to_be_bytes().to_vec();
        for v in [level, 5, 10, 0, 8] {
            b.extend_from_slice(&v.to_be_bytes());
        }
        b.push(2);
        b.extend_from_slice(&4u32.to_be_bytes());
        b.extend_from_slice(&99u32.to_be_bytes());
        b.push(name.len() as u8);
        b.extend_from_slice(name.as_bytes());
        b.extend_from_slice(&5u16.to_be_bytes());
        b.extend_from_slice(b"Burns");
        b
    }

    const GOLDEN: (u64, usize, usize) = (17415989996775463756, 4698, 3799);

    /// Next value of a 64-bit linear congruential generator.
    fn lcg(state: &mut u64) -> u64 {
        *state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        *state >> 33
    }

    /// Input `i`: even = 0..3 valid records cut short by 0..3 bytes, odd = random bytes.
    fn random_input(state: &mut u64, i: usize) -> Vec<u8> {
        if i % 2 == 1 {
            let n = lcg(state) % 40;
            return (0..n).map(|_| lcg(state) as u8).collect();
        }
        let mut d = Vec::new();
        for k in 0..lcg(state) % 4 {
            d.extend(spell_bytes(lcg(state) as u32, lcg(state) as i16, &"n".repeat((lcg(state) % 6 + k) as usize)));
        }
        d.truncate(d.len().saturating_sub((lcg(state) % 4) as usize));
        d
    }

    /// FNV-1a 64-bit hash of `{:?}` of every result, plus the Ok count and total spells parsed.
    fn golden_stats(parse: fn(&[u8]) -> Result<Vec<Spell>, ParseError>) -> (u64, usize, usize) {
        let (mut state, mut hash, mut oks, mut spells) = (12345u64, 0xcbf29ce484222325u64, 0, 0);
        for i in 0..20_000 {
            let result = parse(&random_input(&mut state, i));
            if let Ok(v) = &result {
                oks += 1;
                spells += v.len();
            }
            for b in format!("{:?}", result).bytes() {
                hash = (hash ^ b as u64).wrapping_mul(0x100000001b3);
            }
        }
        (hash, oks, spells)
    }

    #[test]
    fn parse_addspell_golden() {
        assert_eq!(golden_stats(parse_addspell), GOLDEN);
    }

    #[test]
    fn parse_addspell_two() {
        let mut d = spell_bytes(7, 1, "small fireball");
        d.extend(spell_bytes(8, 3, "magic bullet"));
        let s = parse_addspell(&d).unwrap();
        assert_eq!(s.len(), 2);
        assert_eq!((s[0].tag, s[0].level, s[0].mana, s[0].damage), (7, 1, 10, 8));
        assert_eq!((s[0].skill, s[0].path, s[0].face), (2, 4, 99));
        assert_eq!((s[0].name.as_str(), s[0].message.as_str()), ("small fireball", "Burns"));
        assert_eq!(s[1].name, "magic bullet");
    }

    #[test]
    fn addspell_cut_short_is_error() {
        let mut d = spell_bytes(7, 1, "x");
        d.truncate(d.len() - 2);
        assert!(parse_addspell(&d).is_err());
    }

    #[test]
    fn parse_updspell_mana_only() {
        let mut d = vec![0x01];
        d.extend_from_slice(&7u32.to_be_bytes());
        d.extend_from_slice(&12i16.to_be_bytes());
        assert_eq!(parse_updspell(&d).unwrap(), (7, Some(12), None, None));
    }

    #[test]
    fn spellbook_by_name_update_and_remove() {
        let mut book = SpellBook::default();
        book.add(parse_addspell(&[spell_bytes(8, 3, "b"), spell_bytes(7, 1, "a")].concat()).unwrap());
        assert_eq!(book.all().iter().map(|s| s.tag).collect::<Vec<_>>(), [7, 8]);
        book.update(7, Some(20), None, None);
        assert_eq!(book.by_name("a").unwrap().mana, 20);
        book.remove(7);
        book.remove(99);
        assert!(book.by_name("a").is_none());
    }
}
