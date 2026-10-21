//! Where each server message goes: the parchment box, a short pop-up, or only the log.

use std::collections::VecDeque;

#[derive(Debug, PartialEq)]
pub enum Route {
    Box,
    Popup,
    Log,
}

/// A message within this long after a player action counts as caused by the player.
pub const CAUSE_WINDOW_SECS: f32 = 2.0;
const POPUP_SECS: f32 = 3.0;
const LOG_LINES: usize = 500;

/// Kinds are MSG_TYPE values: 1 book .. 6 dialog, 14 attack, 19 victim (subtype 5 = died).
pub fn route(kind: u8, subtype: u8, caused_by_player: bool) -> Route {
    match (kind, subtype) {
        (19, 5) => Route::Box,
        (1..=6, _) if caused_by_player => Route::Box,
        (14 | 19, _) => Route::Popup,
        _ => Route::Log,
    }
}

#[derive(Default)]
pub struct Messages {
    pub boxed: Option<String>,
    /// (text, time it appeared)
    pub popups: Vec<(String, f32)>,
    pub log: VecDeque<String>,
}

impl Messages {
    pub fn push(&mut self, kind: u8, subtype: u8, text: &str, now: f32, last_action: Option<f32>) {
        let caused = last_action.is_some_and(|t| now - t <= CAUSE_WINDOW_SECS);
        match route(kind, subtype, caused) {
            Route::Box => match &mut self.boxed {
                Some(b) => {
                    b.push('\n');
                    b.push_str(text);
                }
                None => self.boxed = Some(text.to_string()),
            },
            Route::Popup => self.popups.push((text.to_string(), now)),
            Route::Log => {}
        }
        if self.log.len() == LOG_LINES {
            self.log.pop_front();
        }
        self.log.push_back(text.to_string());
    }

    pub fn close_box(&mut self) {
        self.boxed = None;
    }

    pub fn expire(&mut self, now: f32) {
        self.popups.retain(|(_, t)| now - t <= POPUP_SECS);
    }
}

/// How many things the log line names before "... and N more".
const STANDING_ON_NAMES: usize = 5;

fn thing_name(item: &cfproto::Item) -> String {
    if item.nrof > 1 {
        return format!("{} {}", item.nrof, item.plural);
    }
    let article = if item.name.starts_with(['a', 'e', 'i', 'o', 'u', 'A', 'E', 'I', 'O', 'U']) { "an" } else { "a" };
    format!("{article} {}", item.name)
}

/// What lies under the player worth naming: not the floor or other fixed things the
/// server marks "no pick", and not the player.
pub fn ground_things(inv: &cfproto::Inventory) -> Vec<&cfproto::Item> {
    const F_NOPICK: u32 = 0x4000;
    inv.ground().into_iter().filter(|i| i.flags & F_NOPICK == 0 && i.tag != inv.player).collect()
}

/// The log line for the things under the player; None when there are none.
pub fn standing_on(items: &[&cfproto::Item]) -> Option<String> {
    if items.is_empty() {
        return None;
    }
    let names: Vec<String> = items.iter().take(STANDING_ON_NAMES).map(|i| thing_name(i)).collect();
    let more = items.len().saturating_sub(STANDING_ON_NAMES);
    let tail = if more > 0 { format!(" ... and {more} more (I to see all)") } else { String::new() };
    Some(format!("You are standing on: {}{tail}.", names.join(", ")))
}

/// The parchment box text: every thing under the player, one per line.
pub fn ground_box(items: &[&cfproto::Item]) -> Option<String> {
    (!items.is_empty()).then(|| {
        let lines: Vec<String> = items.iter().map(|i| thing_name(i)).collect();
        format!("You are standing on:\n{}", lines.join("\n"))
    })
}

/// The ground list changed to a new square's things (none in common with before), not
/// just a drop or a pick-up here.
/// ponytail: by item tags; the client is not told when the player moves onto a square.
pub fn new_square(old: &[u32], new: &[u32]) -> bool {
    !new.is_empty() && !new.iter().any(|t| old.contains(t))
}

/// A message that ends play while in game: on a permadeath server the server sends the
/// character list again after death.
pub fn in_game_reset(msg: &cfproto::ServerMsg) -> Option<&'static str> {
    matches!(msg, cfproto::ServerMsg::AccountPlayers(_)).then_some("Your character died")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn thing(tag: u32, name: &str, plural: &str, nrof: u32) -> cfproto::Item {
        cfproto::Item { tag, location: 0, flags: 0, weight: 0, face: 0, name: name.into(), plural: plural.into(), anim: 0, anim_speed: 0, nrof, kind: 0 }
    }

    #[test]
    fn standing_on_line_names_the_pile() {
        let (a, b, c) = (thing(1, "sword", "swords", 1), thing(2, "scroll", "scrolls", 3), thing(3, "apple", "apples", 1));
        assert_eq!(standing_on(&[]), None);
        assert_eq!(standing_on(&[&a, &b, &c]).unwrap(), "You are standing on: a sword, 3 scrolls, an apple.");
        let many: Vec<cfproto::Item> = (0..8).map(|i| thing(i, "coin", "coins", 1)).collect();
        let refs: Vec<&cfproto::Item> = many.iter().collect();
        assert_eq!(standing_on(&refs).unwrap(), "You are standing on: a coin, a coin, a coin, a coin, a coin ... and 3 more (I to see all).");
        assert_eq!(ground_box(&[&a, &b]).unwrap(), "You are standing on:\na sword\n3 scrolls");
    }

    #[test]
    fn ground_things_skip_the_floor_and_the_player() {
        let mut inv = cfproto::Inventory::default();
        inv.player = 9;
        let mut grass = thing(2, "grass", "grass", 1);
        grass.flags = 0x4000; // F_NOPICK
        inv.apply_item2(0, vec![thing(9, "Showcase", "Showcase", 1), grass, thing(3, "sword", "swords", 1)]);
        let names: Vec<&str> = ground_things(&inv).iter().map(|i| i.name.as_str()).collect();
        assert_eq!(names, ["sword"]);
    }

    #[test]
    fn new_square_when_nothing_is_shared() {
        assert!(new_square(&[], &[1, 2]));
        assert!(new_square(&[5], &[1, 2]));
        assert!(!new_square(&[1, 2], &[1, 2, 3])); // something was dropped here
        assert!(!new_square(&[1], &[]));
    }

    #[test]
    fn accountplayers_in_game_means_death() {
        assert_eq!(in_game_reset(&cfproto::ServerMsg::AccountPlayers(vec![])), Some("Your character died"));
        assert_eq!(in_game_reset(&cfproto::ServerMsg::NewMap), None);
    }

    #[test]
    fn sign_after_apply_boxes() {
        let mut m = Messages::default();
        m.push(4, 1, "Welcome", 10.0, Some(9.0));
        assert_eq!(m.boxed.as_deref(), Some("Welcome"));
        assert_eq!(m.log.back().map(String::as_str), Some("Welcome"));
    }

    #[test]
    fn sign_without_action_logs() {
        let mut m = Messages::default();
        m.push(4, 1, "Welcome", 10.0, Some(5.0));
        m.push(6, 1, "Hello other player", 10.0, None);
        assert!(m.boxed.is_none());
        assert_eq!(m.log.len(), 2);
    }

    #[test]
    fn death_always_boxes() {
        assert_eq!(route(19, 5, false), Route::Box);
    }

    #[test]
    fn attack_pops_up() {
        assert_eq!(route(14, 9, true), Route::Popup);
        assert_eq!(route(19, 1, false), Route::Popup);
        assert_eq!(route(10, 0, true), Route::Log);
    }

    #[test]
    fn popups_expire_after_3s() {
        let mut m = Messages::default();
        m.push(14, 9, "You miss bird!", 10.0, None);
        m.expire(12.9);
        assert_eq!(m.popups.len(), 1);
        m.expire(13.1);
        assert!(m.popups.is_empty());
    }

    #[test]
    fn second_box_message_appends() {
        let mut m = Messages::default();
        m.push(6, 1, "Hello.", 10.0, Some(10.0));
        m.push(6, 1, "Need help?", 10.5, Some(10.0));
        assert_eq!(m.boxed.as_deref(), Some("Hello.\nNeed help?"));
        m.close_box();
        assert!(m.boxed.is_none());
    }

    #[test]
    fn log_capped_at_500() {
        let mut m = Messages::default();
        for i in 0..600 {
            m.push(0, 0, &i.to_string(), 0.0, None);
        }
        assert_eq!(m.log.len(), 500);
        assert_eq!(m.log.front().map(String::as_str), Some("100"));
    }
}
