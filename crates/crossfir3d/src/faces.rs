//! Face (image) table, animations, and the on-disk PNG cache.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use cfproto::{ClientMsg, ANIM_MASK, FACE_IS_ANIM};

pub struct FaceStore {
    dir: PathBuf,
    names: HashMap<u16, (String, u32)>,
    pngs: HashMap<u16, Vec<u8>>,
    anims: HashMap<u16, Vec<u16>>,
    new: Vec<u16>,
    /// Faces whose PNG came from the disk cache, not the server.
    from_cache: HashSet<u16>,
}

pub fn default_cache_dir() -> PathBuf {
    crate::settings::cache_dir().join("faces")
}

impl FaceStore {
    pub fn new(cache_dir: PathBuf) -> Self {
        FaceStore {
            dir: cache_dir,
            names: HashMap::new(),
            pngs: HashMap::new(),
            anims: HashMap::new(),
            new: vec![],
            from_cache: HashSet::new(),
        }
    }

    /// Returns `AskFace` when the image is not in the cache.
    pub fn on_face2(&mut self, num: u16, checksum: u32, name: &str) -> Option<ClientMsg> {
        self.names.insert(num, (name.to_string(), checksum));
        match std::fs::read(self.cache_path(name, checksum)) {
            Ok(png) => {
                self.pngs.insert(num, png);
                self.from_cache.insert(num);
                self.new.push(num);
                None
            }
            Err(_) => Some(ClientMsg::AskFace(num as u32)),
        }
    }

    /// Stores a picture the server sent and saves it to the face cache.
    pub fn on_image2(&mut self, face: u32, png: Vec<u8>) {
        let Ok(num) = u16::try_from(face) else { return };
        if let Some((name, checksum)) = self.names.get(&num) {
            let path = self.cache_path(name, *checksum);
            // Write then rename, so a crash or full disk never leaves a half-written PNG.
            let tmp = path.with_extension("tmp");
            let written = std::fs::create_dir_all(&self.dir)
                .and_then(|_| std::fs::write(&tmp, &png))
                .and_then(|_| std::fs::rename(&tmp, &path));
            if written.is_err() {
                let _ = std::fs::remove_file(&tmp);
            }
            if let Err(e) = written {
                eprintln!("face cache write failed for {}: {e}", path.display());
            }
        }
        self.from_cache.remove(&num);
        self.pngs.insert(num, png);
        self.new.push(num);
    }

    pub fn on_anim(&mut self, id: u16, faces: Vec<u16>) {
        self.anims.insert(id, faces);
    }

    pub fn name(&self, num: u16) -> Option<&str> {
        self.names.get(&num).map(|(n, _)| n.as_str())
    }

    pub fn png(&self, num: u16) -> Option<&[u8]> {
        self.pngs.get(&num).map(Vec::as_slice)
    }

    /// The PNG for `num` would not decode. Drops it; if it came from the disk cache,
    /// deletes the file and asks the server again.
    pub fn on_bad_png(&mut self, num: u16) -> Option<ClientMsg> {
        self.pngs.remove(&num);
        if !self.from_cache.remove(&num) {
            return None;
        }
        let (name, checksum) = self.names.get(&num)?;
        let _ = std::fs::remove_file(self.cache_path(name, *checksum));
        Some(ClientMsg::AskFace(num as u32))
    }

    /// A map layer value to a face number. Animations show their first frame.
    /// ponytail: first frame only; tick-driven animation in M2.
    pub fn resolve(&self, raw: u16) -> u16 {
        if raw & FACE_IS_ANIM == 0 {
            return raw;
        }
        self.anims.get(&(raw & ANIM_MASK)).and_then(|f| f.first().copied()).unwrap_or(0)
    }

    /// The name comes from the server, so keep only safe characters.
    fn cache_path(&self, name: &str, checksum: u32) -> PathBuf {
        let safe: String = name
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.' { c } else { '_' })
            .collect();
        self.dir.join(format!("{}.{checksum:08x}.png", safe.trim_start_matches('.')))
    }

    /// Faces whose PNG arrived since the last call.
    pub fn take_new(&mut self) -> Vec<u16> {
        std::mem::take(&mut self.new)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("crossfir3d-test-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn uncached_face_asks() {
        let mut s = FaceStore::new(temp_dir("uncached"));
        assert_eq!(s.on_face2(7, 0xabc, "grass.111"), Some(ClientMsg::AskFace(7)));
        assert_eq!(s.name(7), Some("grass.111"));
        assert_eq!(s.png(7), None);
    }

    #[test]
    fn image_then_face2_hits_cache() {
        let dir = temp_dir("hit");
        let mut s = FaceStore::new(dir.clone());
        s.on_face2(7, 0xabc, "grass.111");
        s.on_image2(7, vec![1, 2, 3]);
        assert_eq!(s.png(7), Some(&[1u8, 2, 3][..]));
        assert_eq!(s.take_new(), vec![7]);
        assert!(s.take_new().is_empty());

        let mut s2 = FaceStore::new(dir);
        assert_eq!(s2.on_face2(9, 0xabc, "grass.111"), None);
        assert_eq!(s2.png(9), Some(&[1u8, 2, 3][..]));
        assert_eq!(s2.take_new(), vec![9]);
        // A different checksum is a different image.
        assert_eq!(s2.on_face2(10, 0xdef, "grass.111"), Some(ClientMsg::AskFace(10)));
    }

    #[test]
    fn unwritable_cache_still_works() {
        let file = temp_dir("file");
        std::fs::write(&file, b"x").unwrap();
        let mut s = FaceStore::new(file.join("faces"));
        assert_eq!(s.on_face2(7, 1, "grass.111"), Some(ClientMsg::AskFace(7)));
        s.on_image2(7, vec![1]);
        assert_eq!(s.png(7), Some(&[1u8][..]));
        assert_eq!(s.on_face2(8, 1, "grass.111"), Some(ClientMsg::AskFace(8)));
    }

    #[test]
    fn server_name_cannot_escape_cache_dir() {
        let dir = temp_dir("escape");
        let mut s = FaceStore::new(dir.join("faces"));
        s.on_face2(7, 1, "../../evil");
        s.on_image2(7, vec![1]);
        assert!(!dir.join("evil.00000001.png").exists());
        assert!(std::fs::read_dir(dir.join("faces")).unwrap().count() == 1);
    }

    #[test]
    fn bad_cached_png_is_dropped_and_asked_again() {
        let dir = temp_dir("badcache");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("grass.111.00000001.png"), b"trunc").unwrap();
        let mut s = FaceStore::new(dir.clone());
        assert_eq!(s.on_face2(7, 1, "grass.111"), None);
        assert_eq!(s.on_bad_png(7), Some(ClientMsg::AskFace(7)));
        assert!(!dir.join("grass.111.00000001.png").exists());
        assert_eq!(s.png(7), None);
        // A bad image straight from the server is not asked for again (no loop).
        s.on_image2(7, b"still bad".to_vec());
        assert_eq!(s.on_bad_png(7), None);
    }

    #[test]
    fn cache_write_leaves_no_temp_file() {
        let dir = temp_dir("tmpfile");
        let mut s = FaceStore::new(dir.clone());
        s.on_face2(7, 1, "grass.111");
        s.on_image2(7, vec![1, 2]);
        let names: Vec<_> = std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().file_name()).collect();
        assert_eq!(names, vec![std::ffi::OsString::from("grass.111.00000001.png")]);
    }

    #[test]
    fn resolve_anim() {
        let mut s = FaceStore::new(temp_dir("anim"));
        s.on_anim(5, vec![11, 12]);
        assert_eq!(s.resolve(42), 42);
        assert_eq!(s.resolve(FACE_IS_ANIM | 5), 11);
        assert_eq!(s.resolve(FACE_IS_ANIM | 0x4000 | 5), 11); // animation type bits ignored
        assert_eq!(s.resolve(FACE_IS_ANIM | 6), 0);
    }
}
