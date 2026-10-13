//! Desktop entry point. The shared logic lives in the library so the Android entry point
//! can call exactly the same `run_game`.

fn main() {
    let args: Vec<String> = std::env::args().collect();
    std::process::exit(gm8emulator::run_game(args));
}
