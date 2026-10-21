//! Video and sound settings and the settings file (`key=value` lines).

use std::io;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameRate {
    MatchScreen,
    Cap30,
    Cap60,
    Cap120,
    Unlimited,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Quality {
    Off,
    Low,
    Medium,
    High,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AntiAlias {
    Off,
    Msaa4,
    Taa,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindowPlace {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub fullscreen: bool,
    pub window: Option<WindowPlace>,
    pub render_scale: f32,
    pub frame_rate: FrameRate,
    pub ambient_occlusion: Quality,
    pub shadows: Quality,
    pub anti_alias: AntiAlias,
    pub brightness: f32,
    pub contrast: f32,
    pub gamma: f32,
    /// Background music on (sound effects are not affected).
    pub music: bool,
    /// Open the parchment box with what is under the player on every new square
    /// (otherwise only when B is pressed).
    pub ground_popup: bool,
    /// Text size of the log in the bottom-left corner; the log box grows with it.
    pub log_font_size: f32,
    /// How solid the log's dark background is, 0 (clear) to 100 (black) per cent.
    pub log_opacity: f32,
}

/// Smallest and largest log text size.
pub const LOG_FONT_SIZES: (f32, f32) = (8.0, 32.0);

impl Default for Settings {
    fn default() -> Self {
        Settings {
            fullscreen: false,
            window: None,
            render_scale: 1.0,
            frame_rate: FrameRate::MatchScreen,
            ambient_occlusion: Quality::Medium,
            shadows: Quality::Medium,
            anti_alias: AntiAlias::Taa,
            brightness: 0.0,
            contrast: 1.0,
            gamma: 1.0,
            music: true,
            ground_popup: false,
            log_font_size: 10.0,
            log_opacity: 47.0,
        }
    }
}

pub const RENDER_SCALES: [f32; 3] = [0.5, 0.75, 1.0];

impl Settings {
    /// Snaps and clamps values, and never allows MSAA with ambient occlusion (a Bevy limit).
    pub fn normalized(self) -> Settings {
        let render_scale = RENDER_SCALES
            .iter()
            .copied()
            .min_by(|a, b| (a - self.render_scale).abs().total_cmp(&(b - self.render_scale).abs()))
            .unwrap_or(1.0);
        let anti_alias = match (self.anti_alias, self.ambient_occlusion) {
            (AntiAlias::Msaa4, q) if q != Quality::Off => AntiAlias::Taa,
            (aa, _) => aa,
        };
        Settings {
            render_scale,
            anti_alias,
            brightness: self.brightness.clamp(-1.0, 1.0),
            contrast: self.contrast.clamp(0.5, 1.5),
            gamma: self.gamma.clamp(0.5, 2.0),
            ..self
        }
    }

    pub fn to_text(&self) -> String {
        let mut out = format!("fullscreen={}\n", self.fullscreen);
        if let Some(w) = self.window {
            out += &format!("window={},{},{},{}\n", w.x, w.y, w.width, w.height);
        }
        out += &format!("render_scale={}\n", self.render_scale);
        out += &format!("frame_rate={}\n", frame_rate_name(self.frame_rate));
        out += &format!("ambient_occlusion={}\n", quality_name(self.ambient_occlusion));
        out += &format!("shadows={}\n", quality_name(self.shadows));
        out += &format!("anti_alias={}\n", anti_alias_name(self.anti_alias));
        out += &format!("brightness={}\ncontrast={}\ngamma={}\n", self.brightness, self.contrast, self.gamma);
        out += &format!("music={}\n", self.music);
        out += &format!("ground_popup={}\n", self.ground_popup);
        out += &format!("log_font_size={}\nlog_opacity={}\n", self.log_font_size, self.log_opacity);
        out
    }

    /// Unknown keys and bad values are ignored (defaults stay).
    pub fn from_text(text: &str) -> Settings {
        let mut s = Settings::default();
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else { continue };
            let value = value.trim();
            match key.trim() {
                "fullscreen" => s.fullscreen = value.parse().unwrap_or(s.fullscreen),
                "window" => s.window = parse_window(value).or(s.window),
                "render_scale" => s.render_scale = finite(value).unwrap_or(s.render_scale),
                "frame_rate" => s.frame_rate = parse_name(value, FRAME_RATES, frame_rate_name).unwrap_or(s.frame_rate),
                "ambient_occlusion" => {
                    s.ambient_occlusion = parse_name(value, QUALITIES, quality_name).unwrap_or(s.ambient_occlusion)
                }
                "shadows" => s.shadows = parse_name(value, QUALITIES, quality_name).unwrap_or(s.shadows),
                "anti_alias" => s.anti_alias = parse_name(value, ANTI_ALIASES, anti_alias_name).unwrap_or(s.anti_alias),
                "brightness" => s.brightness = finite(value).unwrap_or(s.brightness),
                "contrast" => s.contrast = finite(value).unwrap_or(s.contrast),
                "gamma" => s.gamma = finite(value).unwrap_or(s.gamma),
                "music" => s.music = value.parse().unwrap_or(s.music),
                "ground_popup" => s.ground_popup = value.parse().unwrap_or(s.ground_popup),
                "log_font_size" => {
                    let v = finite(value).unwrap_or(s.log_font_size);
                    s.log_font_size = v.clamp(LOG_FONT_SIZES.0, LOG_FONT_SIZES.1);
                }
                "log_opacity" => s.log_opacity = finite(value).unwrap_or(s.log_opacity).clamp(0.0, 100.0),
                _ => {}
            }
        }
        s
    }
}

const FRAME_RATES: &[FrameRate] =
    &[FrameRate::MatchScreen, FrameRate::Cap30, FrameRate::Cap60, FrameRate::Cap120, FrameRate::Unlimited];
const QUALITIES: &[Quality] = &[Quality::Off, Quality::Low, Quality::Medium, Quality::High];
const ANTI_ALIASES: &[AntiAlias] = &[AntiAlias::Off, AntiAlias::Msaa4, AntiAlias::Taa];

pub fn frame_rate_name(f: FrameRate) -> &'static str {
    match f {
        FrameRate::MatchScreen => "match",
        FrameRate::Cap30 => "30",
        FrameRate::Cap60 => "60",
        FrameRate::Cap120 => "120",
        FrameRate::Unlimited => "unlimited",
    }
}

pub fn quality_name(q: Quality) -> &'static str {
    match q {
        Quality::Off => "off",
        Quality::Low => "low",
        Quality::Medium => "medium",
        Quality::High => "high",
    }
}

pub fn anti_alias_name(a: AntiAlias) -> &'static str {
    match a {
        AntiAlias::Off => "off",
        AntiAlias::Msaa4 => "msaa4",
        AntiAlias::Taa => "taa",
    }
}

fn parse_name<T: Copy>(value: &str, all: &[T], name: fn(T) -> &'static str) -> Option<T> {
    all.iter().copied().find(|v| name(*v) == value)
}

fn parse_window(value: &str) -> Option<WindowPlace> {
    let parts: Vec<&str> = value.split(',').map(str::trim).collect();
    let [x, y, w, h] = parts[..] else { return None };
    let place = WindowPlace { x: x.parse().ok()?, y: y.parse().ok()?, width: w.parse().ok()?, height: h.parse().ok()? };
    (place.width > 0 && place.height > 0).then_some(place)
}

/// A number, but not NaN or infinity (a hand edit could hold either).
fn finite(value: &str) -> Option<f32> {
    value.parse::<f32>().ok().filter(|v| v.is_finite())
}

/// The game was called MacCrossfire: its data and cache folders move to the new name
/// once, unless a Crossfir3d folder already exists.
pub fn move_old_folders(home: &Path) {
    for dir in ["Library/Application Support", "Library/Caches"] {
        let (old, new) = (home.join(dir).join("MacCrossfire"), home.join(dir).join("Crossfir3d"));
        if old.is_dir() && !new.exists() {
            let _ = std::fs::rename(&old, &new);
        }
    }
}

/// CROSSFIR3D_SETTINGS overrides it (tests, screenshots).
/// ponytail: Mac path; the `dirs` crate in M7.
pub fn default_settings_path() -> PathBuf {
    if let Some(p) = std::env::var_os("CROSSFIR3D_SETTINGS") {
        return PathBuf::from(p);
    }
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    home.join("Library/Application Support/Crossfir3d/settings.txt")
}

pub fn load_settings(path: &Path) -> Settings {
    Settings::from_text(&std::fs::read_to_string(path).unwrap_or_default()).normalized()
}

/// Writes a temp file and renames it, so a crash never leaves half a file.
pub fn save_settings(path: &Path, s: &Settings) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, s.to_text())?;
    std::fs::rename(&tmp, path)
}

/// Window positions are saved in logical points (`physical / scale` of the screen the
/// window was on); at start they are placed using the main screen's scale, because
/// that is what winit uses to read a window's starting position.
pub fn to_logical(physical: (i32, i32), scale: f64) -> (i32, i32) {
    ((physical.0 as f64 / scale).round() as i32, (physical.1 as f64 / scale).round() as i32)
}

pub fn to_main_screen_physical(logical: (i32, i32), main_scale: f64) -> (i32, i32) {
    ((logical.0 as f64 * main_scale).round() as i32, (logical.1 as f64 * main_scale).round() as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_place_survives_mixed_density_screens() {
        // Left at physical x=2000 on a 1x external screen; the main screen is Retina 2x.
        let saved = to_logical((2000, 100), 1.0);
        assert_eq!(saved, (2000, 100));
        assert_eq!(to_main_screen_physical(saved, 2.0), (4000, 200));
        // Left on the 2x screen.
        assert_eq!(to_logical((800, 300), 2.0), (400, 150));
        assert_eq!(to_main_screen_physical((-600, 50), 2.0), (-1200, 100));
    }

    #[test]
    fn old_folders_move_to_the_new_name_once() {
        let home = std::env::temp_dir().join(format!("c3d-home-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        let old = home.join("Library/Application Support/MacCrossfire");
        std::fs::create_dir_all(&old).unwrap();
        std::fs::write(old.join("keys.txt"), "x").unwrap();
        move_old_folders(&home);
        assert!(!old.exists());
        assert_eq!(std::fs::read_to_string(home.join("Library/Application Support/Crossfir3d/keys.txt")).unwrap(), "x");
        // A second old folder never replaces the new one.
        std::fs::create_dir_all(&old).unwrap();
        move_old_folders(&home);
        assert!(old.exists());
        let _ = std::fs::remove_dir_all(&home);
    }

    fn temp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("mcf-settings-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d.join("settings.txt")
    }

    #[test]
    fn defaults_when_file_missing() {
        assert_eq!(load_settings(&temp("missing")), Settings::default());
    }

    #[test]
    fn round_trip_all_fields() {
        let s = Settings {
            fullscreen: true,
            window: Some(WindowPlace { x: -1200, y: 40, width: 1600, height: 900 }),
            render_scale: 0.75,
            frame_rate: FrameRate::Cap60,
            ambient_occlusion: Quality::Off,
            shadows: Quality::High,
            anti_alias: AntiAlias::Msaa4,
            brightness: 0.25,
            contrast: 1.2,
            gamma: 0.8,
            music: false,
            ground_popup: true,
            log_font_size: 16.0,
            log_opacity: 85.0,
        };
        assert_eq!(Settings::from_text(&s.to_text()), s);
        let path = temp("roundtrip");
        save_settings(&path, &s).unwrap();
        assert_eq!(load_settings(&path), s);
    }

    #[test]
    fn bad_values_use_defaults() {
        let s = Settings::from_text("render_scale=banana\nshadows=ultra\nno equals sign\nframe_rate=60\n");
        assert_eq!(s.render_scale, 1.0);
        assert_eq!(s.shadows, Quality::Medium);
        assert_eq!(s.frame_rate, FrameRate::Cap60);
    }

    #[test]
    fn nan_and_empty_window_are_ignored() {
        let s = Settings::from_text("render_scale=NaN\nbrightness=nan\ncontrast=inf\ngamma=NaN\nlog_font_size=NaN\nwindow=0,0,0,600\n");
        let d = Settings::default();
        assert_eq!((s.render_scale, s.brightness, s.contrast, s.gamma), (d.render_scale, d.brightness, d.contrast, d.gamma));
        assert_eq!(s.log_font_size, d.log_font_size);
        assert_eq!(s.window, None);
    }

    #[test]
    fn log_values_stay_in_range() {
        let s = Settings::from_text("log_font_size=200\nlog_opacity=-5\n");
        assert_eq!((s.log_font_size, s.log_opacity), (LOG_FONT_SIZES.1, 0.0));
        let s = Settings::from_text("log_font_size=1\nlog_opacity=300\n");
        assert_eq!((s.log_font_size, s.log_opacity), (LOG_FONT_SIZES.0, 100.0));
    }

    #[test]
    fn unknown_keys_ignored() {
        assert_eq!(Settings::from_text("volume=11\nfov=90\n"), Settings::default());
    }

    #[test]
    fn msaa_with_ao_becomes_taa() {
        let s = Settings { anti_alias: AntiAlias::Msaa4, ambient_occlusion: Quality::Low, ..Default::default() };
        assert_eq!(s.normalized().anti_alias, AntiAlias::Taa);
    }

    #[test]
    fn msaa_without_ao_stays() {
        let s = Settings { anti_alias: AntiAlias::Msaa4, ambient_occlusion: Quality::Off, ..Default::default() };
        assert_eq!(s.normalized().anti_alias, AntiAlias::Msaa4);
    }

    #[test]
    fn render_scale_snaps() {
        let snap = |v: f32| Settings { render_scale: v, ..Default::default() }.normalized().render_scale;
        assert_eq!(snap(0.6), 0.5);
        assert_eq!(snap(0.8), 0.75);
        assert_eq!(snap(3.0), 1.0);
    }

    #[test]
    fn values_clamped() {
        let s = Settings { brightness: 5.0, contrast: 0.0, gamma: 9.0, ..Default::default() }.normalized();
        assert_eq!((s.brightness, s.contrast, s.gamma), (1.0, 0.5, 2.0));
    }

    #[test]
    fn load_normalizes_hand_edited_file() {
        let path = temp("hand");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "anti_alias=msaa4\nambient_occlusion=high\n").unwrap();
        assert_eq!(load_settings(&path).anti_alias, AntiAlias::Taa);
    }

    #[test]
    fn save_is_atomic_no_temp_left() {
        let path = temp("atomic");
        save_settings(&path, &Settings::default()).unwrap();
        let names: Vec<_> = std::fs::read_dir(path.parent().unwrap()).unwrap().map(|e| e.unwrap().file_name()).collect();
        assert_eq!(names, vec![std::ffi::OsString::from("settings.txt")]);
    }
}
