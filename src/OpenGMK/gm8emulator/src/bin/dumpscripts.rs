// Temporary diagnostic tool: dumps the GML source of scripts read from a GameMaker 8 exe.
// Usage: cargo run --bin dumpscripts -- <exe> <name1, name2, ...>
use gm8exe::reader;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = &args[1];
    let wanted: Vec<&str> = args[2].split(',').collect();

    let mut file = std::fs::read(path).expect("read exe");
    let assets = reader::from_exe(&mut file, None::<fn(&str)>, false, false).expect("load exe");

    for (i, script) in assets.scripts.iter().enumerate() {
        let Some(script) = script.as_ref() else { continue };
        let name = String::from_utf8_lossy(script.name.0.as_ref()).to_string();
        if wanted.iter().any(|w| w == &name) {
            println!("\n===== script #{} '{}' =====\n", i, name);
            println!("{}", String::from_utf8_lossy(script.source.0.as_ref()));
        }
    }
}
