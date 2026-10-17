//! Character creation data from `requestinfo`: race/class info, starting maps, stat
//! limits. Formats follow the server's socket/requestinfo.cpp.

use crate::server::{ParseError, Reader};

#[derive(Clone, Debug, PartialEq)]
pub struct Choice {
    /// e.g. "race_choice_1"; sent back as `choice <key> <arch>`.
    pub key: String,
    pub description: String,
    /// (arch, display name)
    pub options: Vec<(String, String)>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ArchInfo {
    pub arch: String,
    pub name: String,
    pub message: String,
    /// (stat code, change)
    pub stats: Vec<(u8, i16)>,
    pub choices: Vec<Choice>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StartMap {
    pub arch: String,
    pub name: String,
    pub description: String,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StatLimits {
    pub points: u16,
    pub min: u8,
    pub max: u8,
}

impl Default for StatLimits {
    fn default() -> Self {
        StatLimits { points: 84, min: 3, max: 18 }
    }
}

/// `race_info` / `class_info` data: `<arch>\n` then `name`, `msg`, `stats`, `choice`
/// fields. An unknown field stops parsing; what was read is kept.
pub fn parse_arch_info(data: &[u8]) -> Result<ArchInfo, ParseError> {
    let nl = data.iter().position(|&b| b == b'\n').ok_or_else(|| ParseError("info: no arch line".into()))?;
    let mut info = ArchInfo { arch: String::from_utf8_lossy(&data[..nl]).into_owned(), ..Default::default() };
    let mut r = Reader(&data[nl + 1..]);
    let text = |b: &[u8]| String::from_utf8_lossy(b).into_owned();
    loop {
        let word: Vec<u8> = r.0.iter().take_while(|&&b| b != b' ').copied().collect();
        if word.is_empty() || word.len() == r.0.len() {
            break;
        }
        r.take(word.len() + 1)?;
        match &word[..] {
            b"name" => {
                let len = r.u8()? as usize;
                info.name = text(r.take(len)?);
            }
            b"msg" => {
                let len = r.u16()? as usize;
                info.message = text(r.take(len)?);
            }
            b"stats" => loop {
                let code = r.u8()?;
                if code == 0 {
                    break;
                }
                info.stats.push((code, r.u16()? as i16));
            },
            b"choice" => {
                let len = r.u8()? as usize;
                let key = text(r.take(len)?);
                let len = r.u8()? as usize;
                let description = text(r.take(len)?);
                let mut options = Vec::new();
                loop {
                    let len = r.u8()? as usize;
                    if len == 0 {
                        break;
                    }
                    let arch = text(r.take(len)?);
                    let len = r.u8()? as usize;
                    options.push((arch, text(r.take(len)?)));
                }
                info.choices.push(Choice { key, description, options });
            }
            _ => break, // a newer field we do not know: keep what we have
        }
    }
    Ok(info)
}

/// `startingmap` data: entries of `<type u8><l2string>`; type 1 (arch) starts a map.
pub fn parse_starting_maps(data: &[u8]) -> Result<Vec<StartMap>, ParseError> {
    let mut r = Reader(data);
    let mut maps: Vec<StartMap> = Vec::new();
    while !r.0.is_empty() {
        let kind = r.u8()?;
        let len = r.u16()? as usize;
        let value = String::from_utf8_lossy(r.take(len)?).into_owned();
        match (kind, maps.last_mut()) {
            (1, _) => maps.push(StartMap { arch: value, name: String::new(), description: String::new() }),
            (2, Some(m)) => m.name = value,
            (3, Some(m)) => m.description = value,
            _ => {}
        }
    }
    Ok(maps)
}

/// `newcharinfo` data: lstring lines like "V points 85", "V statrange 3 18".
pub fn parse_newcharinfo(data: &[u8]) -> StatLimits {
    let mut limits = StatLimits::default();
    let mut r = Reader(data);
    while let Ok(len) = r.u8() {
        let Ok(line) = r.take(len as usize) else { break };
        let line = String::from_utf8_lossy(line);
        let words: Vec<&str> = line.trim_end_matches('\0').split(' ').collect();
        match words[..] {
            ["V", "points", n] => limits.points = n.parse().unwrap_or(limits.points),
            ["V", "statrange", a, b] => {
                limits.min = a.parse().unwrap_or(limits.min);
                limits.max = b.parse().unwrap_or(limits.max);
            }
            _ => {}
        }
    }
    limits
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lstr(b: &mut Vec<u8>, s: &str) {
        b.push(s.len() as u8);
        b.extend_from_slice(s.as_bytes());
    }

    fn l2str(b: &mut Vec<u8>, s: &str) {
        b.extend_from_slice(&(s.len() as u16).to_be_bytes());
        b.extend_from_slice(s.as_bytes());
    }

    fn race_bytes(with_choice: bool) -> Vec<u8> {
        let mut b = b"dwarf_player\nname ".to_vec();
        lstr(&mut b, "dwarf");
        b.extend_from_slice(b"msg ");
        l2str(&mut b, "Short and strong.");
        b.extend_from_slice(b"stats ");
        b.extend_from_slice(&[5, 0, 2]); // Str +2
        b.extend_from_slice(&[8, 0xff, 0xff]); // Dex -1
        b.push(0);
        if with_choice {
            b.extend_from_slice(b"choice ");
            lstr(&mut b, "race_choice_1");
            lstr(&mut b, "Choose a god");
            lstr(&mut b, "Gaea");
            lstr(&mut b, "Gaea the earth mother");
            lstr(&mut b, "Valriel");
            lstr(&mut b, "Valriel the angel");
            b.push(0);
        }
        b
    }

    #[test]
    fn arch_info_name_msg_stats() {
        let info = parse_arch_info(&race_bytes(false)).unwrap();
        assert_eq!((info.arch.as_str(), info.name.as_str()), ("dwarf_player", "dwarf"));
        assert_eq!(info.message, "Short and strong.");
        assert_eq!(info.stats, vec![(5, 2), (8, -1)]);
        assert!(info.choices.is_empty());
    }

    #[test]
    fn arch_info_with_choice() {
        let info = parse_arch_info(&race_bytes(true)).unwrap();
        assert_eq!(info.choices.len(), 1);
        let c = &info.choices[0];
        assert_eq!((c.key.as_str(), c.description.as_str()), ("race_choice_1", "Choose a god"));
        assert_eq!(c.options, vec![("Gaea".into(), "Gaea the earth mother".into()), ("Valriel".into(), "Valriel the angel".into())]);
    }

    #[test]
    fn arch_info_unknown_field_stops_cleanly() {
        let mut b = race_bytes(false);
        b.extend_from_slice(b"future \x01\x02\x03");
        let info = parse_arch_info(&b).unwrap();
        assert_eq!(info.stats.len(), 2);
    }

    #[test]
    fn arch_info_without_newline_is_error() {
        assert!(parse_arch_info(b"dwarf_player").is_err());
    }

    #[test]
    fn starting_maps_entries() {
        let mut b = Vec::new();
        for (arch, name, desc) in [("map_tutorial", "Tutorial", "Learn."), ("map_scorn", "Scorn", "A town.")] {
            b.push(1);
            l2str(&mut b, arch);
            b.push(2);
            l2str(&mut b, name);
            b.push(9); // unknown type, skipped
            l2str(&mut b, "x");
            b.push(3);
            l2str(&mut b, desc);
        }
        let maps = parse_starting_maps(&b).unwrap();
        assert_eq!(maps.len(), 2);
        assert_eq!(maps[1], StartMap { arch: "map_scorn".into(), name: "Scorn".into(), description: "A town.".into() });
    }

    #[test]
    fn newcharinfo_points_and_range() {
        let mut b = Vec::new();
        for line in ["V points 85\0", "V statrange 3 18\0", "V statname Str Dex Con Wis Cha Int Pow\0", "R race requestinfo\0"] {
            lstr(&mut b, line);
        }
        assert_eq!(parse_newcharinfo(&b), StatLimits { points: 85, min: 3, max: 18 });
    }

    #[test]
    fn newcharinfo_missing_uses_defaults() {
        assert_eq!(parse_newcharinfo(b""), StatLimits::default());
        assert_eq!(parse_newcharinfo(b"\x05junk"), StatLimits::default());
    }
}
