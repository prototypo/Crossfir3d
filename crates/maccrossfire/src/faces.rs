//! Face (image) table, animations, and the on-disk PNG cache.

use std::collections::HashMap;
use std::path::PathBuf;

use cfproto::{ClientMsg, ANIM_MASK, FACE_IS_ANIM};

pub struct FaceStore {
    dir: PathBuf,
    names: HashMap<u16, (String, u32)>,
    pngs: HashMap<u16, Vec<u8>>,
    anims: HashMap<u16, Vec<u16>>,
    new: Vec<u16>,
}

/// ponytail: Mac only, swap in the `dirs` crate in M7.
pub fn default_cache_dir() -> PathBuf {
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    home.join("Library/Caches/MacCrossfire/faces")
}

impl FaceStore {
    pub fn new(cache_dir: PathBuf) -> Self {
        FaceStore { dir: cache_dir, names: HashMap::new(), pngs: HashMap::new(), anims: HashMap::new(), new: vec![] }
    }

    /// Returns `AskFace` when the image is not in the cache.
    pub fn on_face2(&mut self, num: u16, checksum: u32, name: &str) -> Option<ClientMsg> {
        self.names.insert(num, (name.to_string(), checksum));
        match std::fs::read(self.cache_path(name, checksum)) {
            Ok(png) => {
                self.pngs.insert(num, png);
                self.new.push(num);
                None
            }
            Err(_) => Some(ClientMsg::AskFace(num as u32)),
        }
    }

    pub fn on_image2(&mut self, face: u32, png: Vec<u8>) {
        let Ok(num) = u16::try_from(face) else { return };
        if let Some((name, checksum)) = self.names.get(&num) {
            let path = self.cache_path(name, *checksum);
            let written = std::fs::create_dir_all(&self.dir).and_then(|_| std::fs::write(&path, &png));
            if let Err(e) = written {
                eprintln!("face cache write failed for {}: {e}", path.display());
            }
        }
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
        let d = std::env::temp_dir().join(format!("maccrossfire-test-{tag}-{}", std::process::id()));
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
    fn resolve_anim() {
        let mut s = FaceStore::new(temp_dir("anim"));
        s.on_anim(5, vec![11, 12]);
        assert_eq!(s.resolve(42), 42);
        assert_eq!(s.resolve(FACE_IS_ANIM | 5), 11);
        assert_eq!(s.resolve(FACE_IS_ANIM | 0x4000 | 5), 11); // animation type bits ignored
        assert_eq!(s.resolve(FACE_IS_ANIM | 6), 0);
    }
}
