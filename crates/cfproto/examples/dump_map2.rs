//! Debug tool: prints the map2 packets in a capture folder.
fn main() {
    let dir = std::env::args().nth(1).expect("capture dir");
    let mut files: Vec<_> = std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().path()).collect();
    files.sort();
    for f in files.iter().filter(|f| f.to_string_lossy().ends_with("-map2.bin")) {
        let cfproto::ServerMsg::Map2(items) = cfproto::parse(&std::fs::read(f).unwrap()).unwrap() else { continue };
        println!("== {}", f.file_name().unwrap().to_string_lossy());
        for i in items {
            match i {
                cfproto::Map2Item::Scroll { dx, dy } => println!("scroll {dx},{dy}"),
                cfproto::Map2Item::Tile { x, y, ops } if (8..=16).contains(&x) && (8..=16).contains(&y) => println!("{x},{y} {ops:?}"),
                _ => {}
            }
        }
    }
}
