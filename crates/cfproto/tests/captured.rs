//! Parses real packets saved from a server with CROSSFIR3D_DUMP=captures.

use std::path::Path;

#[test]
fn captured_packets_parse() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../captures");
    let Ok(entries) = std::fs::read_dir(&dir) else {
        println!("no captures");
        return;
    };
    let mut count = 0;
    for entry in entries {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if !name.ends_with(".bin") {
            continue;
        }
        let msg = cfproto::parse(&std::fs::read(&path).unwrap())
            .unwrap_or_else(|e| panic!("{name}: {}", e.0));
        if name.ends_with("-map2.bin") {
            assert!(matches!(&msg, cfproto::ServerMsg::Map2(items) if !items.is_empty()), "{name}: empty map2");
        }
        count += 1;
    }
    println!("{count} captured packets parsed");
}
