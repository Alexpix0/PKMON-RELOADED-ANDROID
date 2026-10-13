// Temporary diagnostic tool: dumps game constants (name = value) from a GameMaker 8 exe.
use gm8exe::reader;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut file = std::fs::read(&args[1]).expect("read exe");
    let assets = reader::from_exe(&mut file, None::<fn(&str)>, false, false).expect("load exe");
    let filter: Option<&str> = args.get(2).map(|s| s.as_str());
    for c in &assets.constants {
        let name = String::from_utf8_lossy(c.name.0.as_ref()).to_string();
        if filter.map(|f| name.contains(f)).unwrap_or(true) {
            println!("{} = {}", name, String::from_utf8_lossy(c.expression.0.as_ref()));
        }
    }
}
