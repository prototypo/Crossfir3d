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

#[cfg(test)]
mod tests {
    use super::*;

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
        a.on_event(AnimEvent::StepStarted, 1.0);
        a.on_event(AnimEvent::Attacked, 1.0);
        assert_eq!(a.current(5.0), Anim::Death);
        a.on_event(AnimEvent::Revived, 6.0);
        assert_eq!(a.current(6.1), Anim::Idle);
    }
}
