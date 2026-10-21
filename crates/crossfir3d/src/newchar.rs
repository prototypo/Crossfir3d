//! The new-character form: race, class, choices, rolled stats, starting place, name.

use std::collections::HashMap;

use cfproto::newchar::{parse_arch_info, parse_newcharinfo, parse_starting_maps};
use cfproto::{ArchInfo, ClientMsg, StartMap, StatLimits};

pub const STAT_NAMES: [&str; 7] = ["Str", "Dex", "Con", "Wis", "Cha", "Int", "Pow"];

/// The server's stat codes in race/class info, in `STAT_NAMES` order.
pub fn stat_code(i: usize) -> u8 {
    [5, 8, 9, 7, 10, 6, 22][i]
}

pub struct NewChar {
    pub races: Vec<String>,
    pub classes: Vec<String>,
    /// Race and class info by arch name (replies can arrive in any order).
    pub infos: HashMap<String, ArchInfo>,
    pub maps: Vec<StartMap>,
    pub limits: StatLimits,
    pub race: Option<String>,
    pub class: Option<String>,
    pub stats: [u8; 7],
    /// choice key -> chosen arch
    pub choices: HashMap<String, String>,
    pub map: Option<String>,
    pub name: String,
    /// The server's reason the last Create failed.
    pub error: Option<String>,
}

impl Default for NewChar {
    fn default() -> Self {
        let limits = StatLimits::default();
        NewChar {
            races: vec![],
            classes: vec![],
            infos: HashMap::new(),
            maps: vec![],
            limits,
            race: None,
            class: None,
            stats: [limits.min; 7],
            choices: HashMap::new(),
            map: None,
            name: String::new(),
            error: None,
        }
    }
}

impl NewChar {
    pub fn requests() -> Vec<ClientMsg> {
        ["newcharinfo", "race_list", "class_list", "startingmap"].map(|q| ClientMsg::RequestInfo(q.into())).to_vec()
    }

    /// Takes a `replyinfo`; returns follow-up requests (race/class info).
    pub fn on_reply(&mut self, kind: &str, data: &[u8]) -> Vec<ClientMsg> {
        let list = || String::from_utf8_lossy(data).split('|').filter(|s| !s.is_empty()).map(String::from).collect::<Vec<_>>();
        let mut out = Vec::new();
        match kind {
            "newcharinfo" => {
                self.limits = parse_newcharinfo(data);
                self.stats = [self.limits.min; 7];
            }
            "race_list" => {
                self.races = list();
                out = self.races.iter().map(|r| ClientMsg::RequestInfo(format!("race_info {r}"))).collect();
                self.race = self.race.take().or_else(|| self.races.first().cloned());
            }
            "class_list" => {
                self.classes = list();
                out = self.classes.iter().map(|c| ClientMsg::RequestInfo(format!("class_info {c}"))).collect();
                self.class = self.class.take().or_else(|| self.classes.first().cloned());
            }
            "race_info" | "class_info" => match parse_arch_info(data) {
                Ok(info) => {
                    self.infos.insert(info.arch.clone(), info);
                }
                Err(e) => eprintln!("{kind}: {}", e.0),
            },
            "startingmap" => match parse_starting_maps(data) {
                Ok(maps) => {
                    self.maps = maps;
                    let scorn = self.maps.iter().find(|m| m.arch == "map_scorn");
                    self.map = scorn.or(self.maps.first()).map(|m| m.arch.clone());
                }
                Err(e) => eprintln!("startingmap: {}", e.0),
            },
            _ => {}
        }
        self.refresh_choices();
        out
    }

    /// The choices the selected race and class offer.
    pub fn offered_choices(&self) -> Vec<&cfproto::Choice> {
        [&self.race, &self.class]
            .into_iter()
            .flatten()
            .filter_map(|a| self.infos.get(a))
            .flat_map(|i| i.choices.iter())
            .collect()
    }

    /// Preselects the first option of every offered choice not chosen yet, or chosen for
    /// an earlier race or class and not offered now.
    pub fn refresh_choices(&mut self) {
        let fixes: Vec<(String, String)> = self
            .offered_choices()
            .iter()
            .filter(|c| !self.choices.get(&c.key).is_some_and(|v| c.options.iter().any(|(arch, _)| arch == v)))
            .filter_map(|c| c.options.first().map(|(arch, _)| (c.key.clone(), arch.clone())))
            .collect();
        self.choices.extend(fixes);
    }

    /// Random stats within the limits: all start at min, then points go one at a time
    /// to random stats below max until the budget is used.
    pub fn roll(&mut self, seed: u64) {
        let mut x = seed.max(1);
        let mut next = || {
            // xorshift64
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x
        };
        let (min, max) = (self.limits.min, self.limits.max);
        self.stats = [min; 7];
        while self.used() < self.limits.points {
            let open: Vec<usize> = (0..7).filter(|&i| self.stats[i] < max).collect();
            if open.is_empty() {
                break;
            }
            self.stats[open[(next() % open.len() as u64) as usize]] += 1;
        }
    }

    pub fn adjust(&mut self, i: usize, delta: i8) {
        let Some(&s) = self.stats.get(i) else { return };
        let new = s as i16 + delta as i16;
        let fits_budget = self.used() as i16 + delta as i16 <= self.limits.points as i16 || delta < 0;
        if (self.limits.min as i16..=self.limits.max as i16).contains(&new) && fits_budget {
            self.stats[i] = new as u8;
        }
    }

    pub fn used(&self) -> u16 {
        self.stats.iter().map(|&s| s as u16).sum()
    }

    pub fn final_stats(&self) -> [i16; 7] {
        let change = |arch: &Option<String>, i: usize| {
            arch.as_ref()
                .and_then(|a| self.infos.get(a))
                .map_or(0, |info| info.stats.iter().filter(|(c, _)| *c == stat_code(i)).map(|(_, v)| *v).sum())
        };
        std::array::from_fn(|i| self.stats[i] as i16 + change(&self.race, i) + change(&self.class, i))
    }

    /// Why Create would fail, if it would.
    pub fn problem(&self) -> Option<String> {
        if self.name.trim().is_empty() {
            return Some("Choose a name.".into());
        }
        if self.race.is_none() || self.class.is_none() {
            return Some("Choose a race and a class.".into());
        }
        if self.used() > self.limits.points {
            return Some(format!("Too many points: {} of {}.", self.used(), self.limits.points));
        }
        let low = self.final_stats().iter().position(|&v| v < 1);
        low.map(|i| format!("{} would be below 1 with this race and class.", STAT_NAMES[i]))
    }

    pub fn create_msg(&self, password: &str) -> Option<ClientMsg> {
        if self.problem().is_some() {
            return None;
        }
        let offered: Vec<&str> = self.offered_choices().iter().map(|c| c.key.as_str()).collect();
        let mut choices: Vec<(String, String)> =
            self.choices.iter().filter(|(k, _)| offered.contains(&k.as_str())).map(|(k, v)| (k.clone(), v.clone())).collect();
        choices.sort();
        Some(ClientMsg::CreatePlayer {
            name: self.name.trim().to_string(),
            password: password.to_string(),
            race: self.race.clone()?,
            class: self.class.clone()?,
            stats: (0..7).map(|i| (STAT_NAMES[i].to_string(), self.stats[i])).collect(),
            starting_map: self.map.clone(),
            choices,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_kept_choice_the_new_race_does_not_offer_is_replaced() {
        let mut n = ready();
        let god = |opts: &[&str]| Choice { key: "race_choice_1".into(), description: "God".into(), options: opts.iter().map(|o| (o.to_string(), o.to_string())).collect() };
        n.infos.insert("dwarf_player".into(), info("dwarf_player", vec![], vec![god(&["Gaea", "Valriel"])]));
        n.choices.insert("race_choice_1".into(), "Sorig".into()); // from an earlier race
        n.refresh_choices();
        assert_eq!(n.choices.get("race_choice_1").map(String::as_str), Some("Gaea"));
        n.choices.insert("race_choice_1".into(), "Valriel".into());
        n.refresh_choices();
        assert_eq!(n.choices.get("race_choice_1").map(String::as_str), Some("Valriel"));
    }
    use cfproto::Choice;

    fn info(arch: &str, stats: Vec<(u8, i16)>, choices: Vec<Choice>) -> ArchInfo {
        ArchInfo { arch: arch.into(), name: arch.into(), message: "m".into(), stats, choices }
    }

    fn ready() -> NewChar {
        let mut n = NewChar::default();
        n.on_reply("race_list", b"|dwarf_player|elf_player");
        n.on_reply("class_list", b"|wizard_class|warrior_class");
        n.infos.insert("dwarf_player".into(), info("dwarf_player", vec![(5, 2), (8, -1)], vec![]));
        n.infos.insert("wizard_class".into(), info("wizard_class", vec![(6, 3)], vec![]));
        n.name = "Bob".into();
        n
    }

    #[test]
    fn requests_ask_four_things() {
        let r = NewChar::requests();
        for q in ["newcharinfo", "race_list", "class_list", "startingmap"] {
            assert!(r.contains(&ClientMsg::RequestInfo(q.into())), "{q}");
        }
    }

    #[test]
    fn lists_trigger_info_requests() {
        let mut n = NewChar::default();
        let out = n.on_reply("race_list", b"|dwarf_player|elf_player");
        assert_eq!(out, vec![ClientMsg::RequestInfo("race_info dwarf_player".into()), ClientMsg::RequestInfo("race_info elf_player".into())]);
        assert_eq!(n.race.as_deref(), Some("dwarf_player"));
        let out = n.on_reply("class_list", b"|wizard_class");
        assert_eq!(out, vec![ClientMsg::RequestInfo("class_info wizard_class".into())]);
    }

    fn maps_bytes(archs: &[&str]) -> Vec<u8> {
        let mut b = Vec::new();
        for a in archs {
            b.push(1);
            b.extend_from_slice(&(a.len() as u16).to_be_bytes());
            b.extend_from_slice(a.as_bytes());
        }
        b
    }

    #[test]
    fn scorn_preselected() {
        let mut n = NewChar::default();
        n.on_reply("startingmap", &maps_bytes(&["map_tutorial", "map_scorn", "map_navar_city"]));
        assert_eq!(n.map.as_deref(), Some("map_scorn"));
    }

    #[test]
    fn first_map_when_no_scorn() {
        let mut n = NewChar::default();
        n.on_reply("startingmap", &maps_bytes(&["map_a", "map_b"]));
        assert_eq!(n.map.as_deref(), Some("map_a"));
    }

    #[test]
    fn infos_keyed_by_name() {
        let mut n = ready();
        n.race = Some("elf_player".into());
        let mut b = b"dwarf_player\nname ".to_vec();
        b.extend_from_slice(&[5]);
        b.extend_from_slice(b"dwarf");
        n.on_reply("race_info", &b);
        assert_eq!(n.race.as_deref(), Some("elf_player"));
        assert_eq!(n.infos["dwarf_player"].name, "dwarf");
    }

    #[test]
    fn choices_preselect_first_option() {
        let mut n = ready();
        let choice = Choice { key: "race_choice_1".into(), description: "God".into(), options: vec![("Gaea".into(), "Gaea".into()), ("Valriel".into(), "Valriel".into())] };
        n.infos.insert("dwarf_player".into(), info("dwarf_player", vec![], vec![choice]));
        n.on_reply("class_list", b"|wizard_class"); // any reply re-checks choices
        assert_eq!(n.choices.get("race_choice_1").map(String::as_str), Some("Gaea"));
    }

    #[test]
    fn roll_obeys_limits_1000_times() {
        let mut n = ready();
        n.limits = StatLimits { points: 84, min: 3, max: 18 };
        let mut shapes = std::collections::HashSet::new();
        for seed in 1..=1000u64 {
            n.roll(seed);
            assert!(n.stats.iter().all(|&s| (3..=18).contains(&s)), "{:?}", n.stats);
            assert!(n.used() <= 84);
            shapes.insert(n.stats);
        }
        assert!(shapes.len() > 100, "rolls should vary");
    }

    #[test]
    fn adjust_respects_limits() {
        let mut n = ready();
        n.stats = [18, 3, 12, 12, 12, 12, 15];
        n.adjust(0, 1);
        assert_eq!(n.stats[0], 18);
        n.adjust(1, -1);
        assert_eq!(n.stats[1], 3);
        assert_eq!(n.used(), 84);
        n.adjust(2, 1); // budget full
        assert_eq!(n.stats[2], 12);
        n.adjust(2, -1);
        assert_eq!(n.stats[2], 11);
    }

    #[test]
    fn final_stats_add_race_and_class() {
        let mut n = ready();
        n.stats = [10; 7];
        // dwarf: Str +2, Dex -1; wizard: Int +3
        assert_eq!(n.final_stats(), [12, 9, 10, 10, 10, 13, 10]);
    }

    #[test]
    fn stat_below_one_is_a_problem() {
        let mut n = ready();
        n.limits.min = 1;
        n.stats = [12, 1, 12, 12, 12, 12, 12];
        assert!(n.problem().unwrap().contains("Dex"));
        n.stats[1] = 2;
        assert!(n.problem().is_none());
        n.name.clear();
        assert!(n.problem().is_some());
    }

    #[test]
    fn create_msg_carries_everything() {
        let mut n = ready();
        n.stats = [12; 7];
        n.map = Some("map_scorn".into());
        let choice = Choice { key: "race_choice_1".into(), description: "God".into(), options: vec![("Gaea".into(), "Gaea".into())] };
        n.infos.insert("dwarf_player".into(), info("dwarf_player", vec![], vec![choice]));
        n.choices.insert("race_choice_1".into(), "Gaea".into());
        n.choices.insert("class_choice_9".into(), "Stale".into()); // not offered by the selection: dropped
        let Some(ClientMsg::CreatePlayer { name, password, race, class, stats, starting_map, choices }) = n.create_msg("pw") else {
            panic!()
        };
        assert_eq!((name.as_str(), password.as_str(), race.as_str(), class.as_str()), ("Bob", "pw", "dwarf_player", "wizard_class"));
        assert_eq!(stats.len(), 7);
        assert_eq!(stats[0], ("Str".to_string(), 12));
        assert_eq!(starting_map.as_deref(), Some("map_scorn"));
        assert_eq!(choices, vec![("race_choice_1".to_string(), "Gaea".to_string())]);
    }
}
