//! The `stats` command and the player's stats. Value sizes follow `esrv_update_stats`
//! in the server's socket/request.cpp.

use crate::server::ParseError;

#[derive(Clone, Debug, PartialEq)]
pub enum StatValue {
    Int(i64),
    Text(String),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Stats {
    pub hp: i16,
    pub max_hp: i16,
    pub sp: i16,
    pub max_sp: i16,
    pub grace: i16,
    pub max_grace: i16,
    pub food: i16,
    pub level: i16,
    pub exp: i64,
    pub str_: i16,
    pub dex: i16,
    pub con: i16,
    pub wis: i16,
    pub cha: i16,
    pub int: i16,
    pub pow: i16,
    pub range: String,
    pub title: String,
}

/// Parses until the end or the first unknown stat code. Returns what was read and,
/// for an unknown code or short data, the error.
pub fn parse_stats(data: &[u8]) -> (Vec<(u8, StatValue)>, Option<ParseError>) {
    let mut out = Vec::new();
    let mut i = 0;
    let int = |d: &[u8], i: usize, n: usize| -> Option<i64> {
        let b = d.get(i..i + n)?;
        Some(match n {
            2 => i16::from_be_bytes([b[0], b[1]]) as i64,
            4 => i32::from_be_bytes(b.try_into().ok()?) as i64,
            _ => i64::from_be_bytes(b.try_into().ok()?),
        })
    };
    while i < data.len() {
        let code = data[i];
        i += 1;
        let short = || Some(ParseError(format!("stats: value for code {code} is cut short")));
        let (value, size) = match code {
            17 | 19 | 26 | 29 | 30 | 31 | 55 | 57 => (int(data, i, 4).map(StatValue::Int), 4),
            28 => (int(data, i, 8).map(StatValue::Int), 8),
            20 | 21 | 56 => {
                let len = data.get(i).map_or(0, |&l| l as usize);
                let text = data.get(i + 1..i + 1 + len).map(|t| StatValue::Text(String::from_utf8_lossy(t).into()));
                (if data.get(i).is_some() { text } else { None }, 1 + len)
            }
            140..=189 => (int(data, i + 1, 8).filter(|_| i < data.len()).map(StatValue::Int), 9),
            1..=10 | 12..=16 | 18 | 22..=25 | 32..=54 | 58 | 100..=117 => (int(data, i, 2).map(StatValue::Int), 2),
            _ => return (out, Some(ParseError(format!("stats: unknown code {code}")))),
        };
        let Some(value) = value else { return (out, short()) };
        out.push((code, value));
        i += size;
    }
    (out, None)
}

impl Stats {
    pub fn apply(&mut self, updates: &[(u8, StatValue)]) {
        for (code, value) in updates {
            match value {
                StatValue::Text(t) => match code {
                    20 => self.range = t.clone(),
                    21 => self.title = t.clone(),
                    _ => {}
                },
                StatValue::Int(v) => {
                    let short = *v as i16;
                    match code {
                        1 => self.hp = short,
                        2 => self.max_hp = short,
                        3 => self.sp = short,
                        4 => self.max_sp = short,
                        5 => self.str_ = short,
                        6 => self.int = short,
                        7 => self.wis = short,
                        8 => self.dex = short,
                        9 => self.con = short,
                        10 => self.cha = short,
                        12 => self.level = short,
                        18 => self.food = short,
                        22 => self.pow = short,
                        23 => self.grace = short,
                        24 => self.max_grace = short,
                        28 => self.exp = *v,
                        _ => {}
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stats_hp_and_max() {
        let (u, err) = parse_stats(&[1, 0, 50, 2, 0, 80]);
        assert_eq!(u, vec![(1, StatValue::Int(50)), (2, StatValue::Int(80))]);
        assert!(err.is_none());
    }

    #[test]
    fn stats_negative_short() {
        let (u, _) = parse_stats(&[14, 0xff, 0xfe]); // AC -2
        assert_eq!(u, vec![(14, StatValue::Int(-2))]);
    }

    #[test]
    fn stats_exp64() {
        let mut d = vec![28];
        d.extend_from_slice(&1_000_000i64.to_be_bytes());
        assert_eq!(parse_stats(&d).0, vec![(28, StatValue::Int(1_000_000))]);
    }

    #[test]
    fn stats_int32_and_skill() {
        let mut d = vec![26];
        d.extend_from_slice(&70_000i32.to_be_bytes());
        d.push(141); // a skill: level byte + exp64
        d.push(3);
        d.extend_from_slice(&500i64.to_be_bytes());
        d.extend_from_slice(&[18, 0, 200]); // food after it still parses
        let (u, err) = parse_stats(&d);
        assert!(err.is_none());
        assert_eq!(u[0], (26, StatValue::Int(70_000)));
        assert_eq!(u.last().unwrap(), &(18, StatValue::Int(200)));
    }

    #[test]
    fn stats_title_lstring() {
        let mut d = vec![21, 7];
        d.extend_from_slice(b"Fighter");
        assert_eq!(parse_stats(&d).0, vec![(21, StatValue::Text("Fighter".into()))]);
    }

    #[test]
    fn stats_unknown_code_keeps_earlier() {
        let (u, err) = parse_stats(&[1, 0, 50, 250, 1, 2, 3]);
        assert_eq!(u, vec![(1, StatValue::Int(50))]);
        assert!(err.is_some());
    }

    #[test]
    fn stats_short_value_is_error() {
        let (u, err) = parse_stats(&[1, 0]);
        assert!(u.is_empty() && err.is_some());
    }

    #[test]
    fn stats_apply_updates_model() {
        let mut s = Stats::default();
        s.apply(&[
            (1, StatValue::Int(50)),
            (3, StatValue::Int(7)),
            (23, StatValue::Int(9)),
            (18, StatValue::Int(999)),
            (28, StatValue::Int(1234)),
            (12, StatValue::Int(2)),
            (21, StatValue::Text("Fighter".into())),
            (100, StatValue::Int(30)), // resistance: ignored
        ]);
        assert_eq!((s.hp, s.sp, s.grace, s.food, s.exp, s.level), (50, 7, 9, 999, 1234, 2));
        assert_eq!(s.title, "Fighter");
    }
}
