//! Crossfire's game clock (from the server's `time` reply and `tick`s) and where the
//! sun or moon is at each hour. Constants from the server's include/tod.h and
//! common/time.cpp.

use std::f32::consts::PI;

const HOURS_PER_DAY: f32 = 28.0;
const TICKS_PER_HOUR: f32 = 1500.0;
const TICKS_PER_MINUTE: f32 = 25.0; // 1500 / 58 in integer maths
const SUNRISE: f32 = 5.0; // dawn starts
const SUNSET: f32 = 23.0; // night starts

/// Used until the clock is synced (morning).
pub const FALLBACK_HOUR: f32 = 10.0;

/// "It is 12 minutes past 3 o'clock pm, on ..." -> (hour 0..27, minute 0..58).
pub fn parse_time_reply(text: &str) -> Option<(u8, u8)> {
    let rest = text.strip_prefix("It is ")?;
    let (minute, rest) = rest.split_once(' ')?;
    let (_, rest) = rest.split_once("past ")?;
    let (hour, rest) = rest.split_once(" o'clock ")?;
    let (minute, hour): (u8, u8) = (minute.parse().ok()?, hour.parse().ok()?);
    // Shown minute = (pticks % 1500) / 25 + 1, so 1..=59.
    if !(1..=59).contains(&minute) || !(1..=14).contains(&hour) {
        return None;
    }
    let half = if rest.starts_with("pm") {
        14
    } else if rest.starts_with("am") {
        0
    } else {
        return None;
    };
    Some((hour % 14 + half, minute - 1))
}

pub fn is_time_reply(text: &str) -> bool {
    text.starts_with("It is ") && text.contains(" o'clock ")
}

/// Any line of the server's reply to `time` ("It is ...", "The 3rd Day of the ...",
/// "Time of Year: ..."), so our own request can be hidden from the player.
pub fn is_time_reply_line(text: &str) -> bool {
    is_time_reply(text)
        || (text.starts_with("The ") && text.contains(" Day of the ") && text.contains(", Year "))
        || text.starts_with("Time of Year: ")
        || is_dm_timing_line(text)
}

/// A DM's reply to `time` goes on after the season with the server's tick timings.
fn is_dm_timing_line(text: &str) -> bool {
    ["Statistics for last ", "\tticks longer than ", "Time last ", "avg time=", "ticks longer than max time "].iter().any(|s| text.starts_with(s))
}

/// The last line of the server's reply to `time` (common/time.cpp `time_info`): the
/// season for a player, the last timing line for a DM. After it, a `time` the player
/// types is theirs to see.
pub fn ends_time_reply(text: &str) -> bool {
    text.starts_with("Time of Year: ") || text.starts_with("ticks longer than max time ")
}

#[derive(Default)]
pub struct GameClock {
    /// (hour at sync, tick at sync)
    synced: Option<(f32, u32)>,
    last_tick: u32,
}

impl GameClock {
    pub fn on_tick(&mut self, tick: u32) {
        self.last_tick = tick;
    }

    pub fn sync(&mut self, hour: u8, minute: u8) {
        let h = hour as f32 + minute as f32 * TICKS_PER_MINUTE / TICKS_PER_HOUR;
        self.synced = Some((h, self.last_tick));
    }

    /// 0.0..28.0, or None before the first sync.
    pub fn hour(&self) -> Option<f32> {
        let (h, t) = self.synced?;
        let passed = self.last_tick.wrapping_sub(t) as f32 / TICKS_PER_HOUR;
        Some((h + passed).rem_euclid(HOURS_PER_DAY))
    }
}

#[derive(Debug, PartialEq)]
pub struct SunPose {
    /// Radians above the horizon.
    pub elevation: f32,
    /// Radians: 0 = east, PI/2 = south, PI = west.
    pub azimuth: f32,
    pub moon: bool,
}

pub fn sun_pose(hour: f32) -> SunPose {
    let hour = hour.rem_euclid(HOURS_PER_DAY);
    if (SUNRISE..SUNSET).contains(&hour) {
        let t = (hour - SUNRISE) / (SUNSET - SUNRISE);
        SunPose { elevation: (PI * t).sin() * 70f32.to_radians(), azimuth: PI * t, moon: false }
    } else {
        SunPose { elevation: 60f32.to_radians(), azimuth: PI / 2.0, moon: true }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dms_timing_lines_are_part_of_the_reply() {
        for line in [
            "Statistics for last 1305757 ticks:\n\tmin/avg/max = 0/59/1309 ms per tick",
            "\tticks longer than 120 ms = 9 (0%)",
            "Time last 100 ticks:",
            "avg time=5ms  max time=57ms  min time=0ms",
            "ticks longer than max time (120ms) = 0 (0%)",
        ] {
            assert!(is_time_reply_line(line), "{line}");
        }
        assert!(ends_time_reply("ticks longer than max time (120ms) = 0 (0%)"), "a DM's reply ends here");
        assert!(ends_time_reply("Time of Year: Season of the Blizzard"), "a player's ends here");
    }

    #[test]
    fn parses_real_reply() {
        assert_eq!(parse_time_reply("It is 12 minutes past 3 o'clock pm, on the Day of the Bull"), Some((17, 11)));
    }

    #[test]
    fn parses_one_minute_and_14_am() {
        assert_eq!(parse_time_reply("It is 1 minute past 14 o'clock am, on the Day of the Moon"), Some((0, 0)));
    }

    #[test]
    fn fourteen_pm_is_hour_14() {
        assert_eq!(parse_time_reply("It is 58 minutes past 14 o'clock pm, on x"), Some((14, 57)));
    }

    #[test]
    fn fifty_nine_minutes_is_valid() {
        // The server's minute is (pticks % 1500) / 25, which reaches 58; shown as 59.
        assert_eq!(parse_time_reply("It is 59 minutes past 3 o'clock pm, on x"), Some((17, 58)));
    }

    #[test]
    fn unparseable_reply_gives_none() {
        assert_eq!(parse_time_reply("It is late."), None);
        assert_eq!(parse_time_reply("It is 99 minutes past 3 o'clock pm"), None);
        assert_eq!(parse_time_reply("It is 5 minutes past 15 o'clock am"), None);
    }

    #[test]
    fn all_time_reply_lines_recognised() {
        assert!(is_time_reply_line("It is 1 minute past 2 o'clock am, on the Day of the Bull"));
        assert!(is_time_reply_line("The 3rd Day of the Month of Winter, Year 1"));
        assert!(is_time_reply_line("Time of Year: The Season of New Year"));
        assert!(!is_time_reply_line("The house is being over run with monsters!"));
        assert!(!is_time_reply_line("You miss bird!"));
    }

    #[test]
    fn season_line_ends_the_reply() {
        assert!(ends_time_reply("Time of Year: The Season of New Year"));
        assert!(!ends_time_reply("It is 1 minute past 2 o'clock am, on the Day of the Bull"));
    }

    #[test]
    fn is_time_reply_only_for_time() {
        assert!(is_time_reply("It is 1 minute past 2 o'clock am, on the Day of the Bull"));
        assert!(!is_time_reply("It is dark."));
        assert!(!is_time_reply("Scorn is the best place"));
    }

    #[test]
    fn clock_advances_by_ticks() {
        let mut c = GameClock::default();
        c.on_tick(1000);
        c.sync(6, 0);
        c.on_tick(1750);
        assert!((c.hour().unwrap() - 6.5).abs() < 1e-4);
    }

    #[test]
    fn clock_wraps_at_28() {
        let mut c = GameClock::default();
        c.sync(27, 0);
        c.on_tick(3000);
        assert!((c.hour().unwrap() - 1.0).abs() < 1e-4);
    }

    #[test]
    fn not_synced_is_none() {
        let mut c = GameClock::default();
        c.on_tick(5);
        assert_eq!(c.hour(), None);
    }

    #[test]
    fn sun_high_at_14_low_at_dawn() {
        let (dawn, noon) = (sun_pose(6.0), sun_pose(14.0));
        assert!(!dawn.moon && !noon.moon);
        assert!(noon.elevation > dawn.elevation && dawn.elevation > 0.0);
    }

    #[test]
    fn sun_east_morning_west_evening() {
        assert!(sun_pose(6.0).azimuth < PI / 2.0);
        assert!(sun_pose(20.0).azimuth > PI / 2.0);
    }

    #[test]
    fn night_is_moon() {
        assert!(sun_pose(2.0).moon);
        assert!(sun_pose(23.5).moon);
        assert!(sun_pose(4.9).moon);
    }
}
