//! Maps server sound actions to files, from crossfire-sounds' sounds.conf.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Default)]
pub struct SoundTable {
    sounds: HashMap<String, (String, u8)>,
}

/// Bundled with the program, else the source checkout's. CROSSFIR3D_SOUNDS overrides it.
pub fn sounds_dir() -> PathBuf {
    let dev = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../upstream/crossfire-sounds");
    std::env::var_os("CROSSFIR3D_SOUNDS")
        .map(PathBuf::from)
        .unwrap_or_else(|| crate::settings::bundled(&std::env::current_exe().unwrap_or_default(), "sounds", &dev))
}

impl SoundTable {
    /// Lines are `action:volume:file`; `#` lines, blank lines, and bad lines are skipped.
    pub fn parse(conf: &str) -> SoundTable {
        let sounds = conf
            .lines()
            .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
            .filter_map(|l| {
                let mut parts = l.rsplitn(3, ':');
                let (file, volume, action) = (parts.next()?, parts.next()?, parts.next()?);
                Some((action.to_string(), (file.trim().to_string(), volume.trim().parse().ok()?)))
            })
            .collect();
        SoundTable { sounds }
    }

    /// Reads `dir/sounds.conf`. A missing file gives an empty table.
    pub fn load(dir: &Path) -> SoundTable {
        match std::fs::read_to_string(dir.join("sounds.conf")) {
            Ok(conf) => SoundTable::parse(&conf),
            Err(e) => {
                eprintln!("no sounds: {}: {e}", dir.display());
                SoundTable::default()
            }
        }
    }

    pub fn lookup(&self, action: &str) -> Option<(&str, u8)> {
        self.sounds.get(action).map(|(f, v)| (f.as_str(), *v))
    }
}

/// The music file for a server song name, if it exists. Names that could leave the
/// music folder are rejected (the name comes from the server).
pub fn music_file(dir: &Path, name: &str) -> Option<PathBuf> {
    let safe = !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    if !safe {
        return None;
    }
    ["ogg", "mp3"].iter().map(|ext| dir.join("music").join(format!("{name}.{ext}"))).find(|p| p.exists())
}

/// The server resends the song on every map change; only a different song restarts.
pub fn music_changes(current: Option<&str>, new: &str) -> bool {
    match current {
        Some(c) => c != new,
        None => new != "NONE",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn music_name_cannot_leave_folder() {
        let dir = std::env::temp_dir().join(format!("mcf-music-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("music")).unwrap();
        std::fs::write(dir.join("music/scorn.mp3"), b"x").unwrap();
        std::fs::write(dir.join("secret.ogg"), b"x").unwrap();
        assert_eq!(music_file(&dir, "scorn"), Some(dir.join("music/scorn.mp3")));
        assert_eq!(music_file(&dir, "../secret"), None);
        assert_eq!(music_file(&dir, "/etc/x"), None);
        assert_eq!(music_file(&dir, ""), None);
        assert_eq!(music_file(&dir, "navar"), None);
    }

    #[test]
    fn same_song_does_not_restart() {
        assert!(!music_changes(Some("scorn"), "scorn"));
        assert!(music_changes(Some("scorn"), "navar"));
        assert!(music_changes(None, "scorn"));
        assert!(music_changes(Some("scorn"), "NONE"));
        assert!(!music_changes(None, "NONE"));
    }

    #[test]
    fn parse_conf_lines() {
        let t = SoundTable::parse("miss:50:miss-1.ogg\nturn handle:80:boink2.wav\n");
        assert_eq!(t.lookup("miss"), Some(("miss-1.ogg", 50)));
        assert_eq!(t.lookup("turn handle"), Some(("boink2.wav", 80)));
    }

    #[test]
    fn comments_and_bad_lines_skipped() {
        let t = SoundTable::parse("# comment\n\nbroken line\nloud:x:a.wav\nok:200:b.wav\n");
        assert_eq!(t.lookup("ok"), Some(("b.wav", 200)));
        assert_eq!(t.lookup("loud"), None);
    }

    #[test]
    fn unknown_action_none() {
        assert_eq!(SoundTable::parse("a:1:b.wav").lookup("nope"), None);
    }

    #[test]
    fn load_missing_dir_is_empty() {
        assert_eq!(SoundTable::load(Path::new("/no/such/dir")).lookup("miss"), None);
    }

    #[test]
    fn real_conf_parses() {
        let dir = sounds_dir();
        if !dir.join("sounds.conf").exists() {
            println!("no sounds checkout");
            return;
        }
        assert_eq!(SoundTable::load(&dir).lookup("miss"), Some(("miss-1.ogg", 50)));
    }
}
