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

/// A message that ends play while in game: on a permadeath server the server sends the
/// character list again after death.
pub fn in_game_reset(msg: &cfproto::ServerMsg) -> Option<&'static str> {
    matches!(msg, cfproto::ServerMsg::AccountPlayers(_)).then_some("Your character died")
}

#[cfg(test)]
mod tests {
    use super::*;

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
