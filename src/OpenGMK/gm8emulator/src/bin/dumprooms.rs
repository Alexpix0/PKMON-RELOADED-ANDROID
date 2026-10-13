// Temporary diagnostic tool: dumps room ids and names from a GameMaker 8 exe.
use gm8exe::reader;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut file = std::fs::read(&args[1]).expect("read exe");
    let assets = reader::from_exe(&mut file, None::<fn(&str)>, false, false).expect("load exe");
    let filter = args.get(2).map(|s| s.to_lowercase());
    for (id, room) in assets.rooms.iter().enumerate() {
        let Some(room) = room else { continue };
        let name = String::from_utf8_lossy(room.name.0.as_ref()).to_string();
        if let Some(filter) = &filter {
            if !name.to_lowercase().contains(filter) {
                continue
            }
        }
        println!("{} = {:?} ({}x{})", id, name, room.width, room.height);
    }
}
