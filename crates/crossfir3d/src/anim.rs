//! Which animation the player character plays. The server does not say what a player
//! is doing, so the client works it out from events.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Anim {
    Idle,
    Walk,
    Attack,
    Cast,
    Death,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnimEvent {
    StepStarted,
    StepEnded,
    Attacked,
    Cast,
    Died,
    Revived,
}

/// Attack and Cast play once for this long.
pub const ONE_SHOT_SECS: f32 = 0.8;
/// Walking continues this long after the last step ends (smooth between steps).
pub const WALK_LINGER_SECS: f32 = 0.3;

#[derive(Default)]
pub struct AnimState {
    one_shot: Option<(Anim, f32)>,
    walking_until: Option<f32>,
    stepping: bool,
    dead: bool,
}

impl AnimState {
    pub fn on_event(&mut self, ev: AnimEvent, now: f32) {
        match ev {
            AnimEvent::StepStarted => {
                // The server only moves a living player (an arena death keeps the map).
                self.dead = false;
                self.stepping = true;
                self.walking_until = None;
            }
            AnimEvent::StepEnded => {
                self.stepping = false;
                self.walking_until = Some(now + WALK_LINGER_SECS);
            }
            AnimEvent::Attacked => self.one_shot = Some((Anim::Attack, now + ONE_SHOT_SECS)),
            AnimEvent::Cast => self.one_shot = Some((Anim::Cast, now + ONE_SHOT_SECS)),
            AnimEvent::Died => self.dead = true,
            AnimEvent::Revived => *self = AnimState::default(),
        }
    }

    pub fn current(&self, now: f32) -> Anim {
        if self.dead {
            return Anim::Death;
        }
        if let Some((anim, until)) = self.one_shot {
            if now < until {
                return anim;
            }
        }
        if self.stepping || self.walking_until.is_some_and(|t| now < t) {
            Anim::Walk
        } else {
            Anim::Idle
        }
    }
}

/// MSG_TYPE 14 (attack) subtypes 1 hit, 3 fumble, 4 kill, 9 miss are the player's own
/// swing (not a pet's, not a refused attack); 19/5 (victim, died) is death.
pub fn event_for_message(kind: u8, subtype: u8) -> Option<AnimEvent> {
    match (kind, subtype) {
        (14, 1 | 3 | 4 | 9) => Some(AnimEvent::Attacked),
        (19, 5) => Some(AnimEvent::Died),
        _ => None,
    }
}

pub fn event_for_command(command: &str) -> Option<AnimEvent> {
    command.starts_with("cast ").then_some(AnimEvent::Cast)
}

/// The Universal Animation Library clip for each animation.
pub fn clip_name(anim: Anim) -> &'static str {
    match anim {
        Anim::Idle => "Idle_Loop",
        Anim::Walk => "Walk_Loop",
        Anim::Attack => "Sword_Attack",
        Anim::Cast => "Spell_Simple_Shoot",
        Anim::Death => "Death01",
    }
}

pub fn looping(anim: Anim) -> bool {
    matches!(anim, Anim::Idle | Anim::Walk)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_and_commands_start_animations() {
        assert_eq!(event_for_message(19, 5), Some(AnimEvent::Died));
        assert_eq!(event_for_message(19, 1), None);
        assert_eq!(event_for_message(1, 0), None);
        assert_eq!(event_for_command("cast 77"), Some(AnimEvent::Cast));
        assert_eq!(event_for_command("fire 3"), None);
    }

    #[test]
    fn only_the_players_own_attacks_swing() {
        for sub in [1, 3, 4, 9] {
            assert_eq!(event_for_message(14, sub), Some(AnimEvent::Attacked), "subtype {sub}");
        }
        for sub in [2, 5, 6, 7, 8] {
            assert_eq!(event_for_message(14, sub), None, "pet or non-attack subtype {sub}");
        }
    }

    #[test]
    fn a_step_after_death_means_alive_again() {
        let mut a = AnimState::default();
        a.on_event(AnimEvent::Died, 0.0);
        a.on_event(AnimEvent::StepStarted, 5.0);
        assert_eq!(a.current(5.1), Anim::Walk);
    }

    #[test]
    fn every_anim_has_a_clip() {
        for a in [Anim::Idle, Anim::Walk, Anim::Attack, Anim::Cast, Anim::Death] {
            assert!(!clip_name(a).is_empty());
        }
        assert!(looping(Anim::Walk) && !looping(Anim::Death));
    }

    #[test]
    fn idle_by_default() {
        assert_eq!(AnimState::default().current(0.0), Anim::Idle);
    }

    #[test]
    fn attack_then_back_to_idle() {
        let mut a = AnimState::default();
        a.on_event(AnimEvent::Attacked, 1.0);
        assert_eq!(a.current(1.5), Anim::Attack);
        assert_eq!(a.current(1.0 + ONE_SHOT_SECS + 0.01), Anim::Idle);
    }

    #[test]
    fn cast_returns_to_walk_while_stepping() {
        let mut a = AnimState::default();
        a.on_event(AnimEvent::StepStarted, 0.0);
        a.on_event(AnimEvent::Cast, 0.1);
        assert_eq!(a.current(0.2), Anim::Cast);
        assert_eq!(a.current(1.0), Anim::Walk);
    }

    #[test]
    fn walk_lingers_briefly() {
        let mut a = AnimState::default();
        a.on_event(AnimEvent::StepStarted, 0.0);
        a.on_event(AnimEvent::StepEnded, 0.3);
        assert_eq!(a.current(0.5), Anim::Walk);
        assert_eq!(a.current(0.3 + WALK_LINGER_SECS + 0.01), Anim::Idle);
    }

    #[test]
    fn death_holds_until_revived() {
        let mut a = AnimState::default();
        a.on_event(AnimEvent::Died, 0.0);
        a.on_event(AnimEvent::Attacked, 1.0);
        assert_eq!(a.current(5.0), Anim::Death);
        a.on_event(AnimEvent::Revived, 6.0);
        assert_eq!(a.current(6.1), Anim::Idle);
    }
}
