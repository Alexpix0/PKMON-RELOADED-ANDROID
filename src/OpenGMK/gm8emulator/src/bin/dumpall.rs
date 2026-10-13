// Temporary diagnostic tool: dumps every script's GML source from a GameMaker 8 exe.
use gm8exe::reader;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut file = std::fs::read(&args[1]).expect("read exe");
    let assets = reader::from_exe(&mut file, None::<fn(&str)>, false, false).expect("load exe");
    for (i, script) in assets.scripts.iter().enumerate() {
        let Some(script) = script.as_ref() else { continue };
        let name = String::from_utf8_lossy(script.name.0.as_ref());
        println!("\n===== script #{} '{}' =====\n{}", i, name, String::from_utf8_lossy(script.source.0.as_ref()));
    }
}
