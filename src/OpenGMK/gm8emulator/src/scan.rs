//! `--scan-assets`: validates every external asset the game can load, using the same decoders it
//! uses at runtime, so a format problem shows up here instead of as a missing sprite or a crash in
//! the middle of a battle.

use std::collections::BTreeMap;
use std::path::Path;

pub fn run(dir: &Path) -> i32 {
    let mut totals: BTreeMap<&'static str, (u32, u32)> = BTreeMap::new();
    let mut failures: Vec<String> = Vec::new();
    walk(dir, &mut totals, &mut failures);

    println!("{:<8} {:>8} {:>8}", "tipo", "ok", "fallan");
    for (kind, (ok, bad)) in &totals {
        println!("{:<8} {:>8} {:>8}", kind, ok, bad);
    }

    if failures.is_empty() {
        println!("\nSin fallos.");
        return 0
    }
    println!("\n{} fallos (primeros 40):", failures.len());
    for failure in failures.iter().take(40) {
        println!("  {failure}");
    }
    1
}

fn walk(dir: &Path, totals: &mut BTreeMap<&'static str, (u32, u32)>, failures: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, totals, failures);
            continue
        }
        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
        let (kind, result): (&'static str, Result<(), String>) = match extension.as_str() {
            "gmspr" => ("gmspr", check_gmspr(&path)),
            "gif" | "png" | "bmp" | "jpg" | "jpeg" => ("imagen", check_image(&path)),
            "mid" | "midi" => ("midi", check_midi(&path)),
            "wav" => ("wav", check_sound(&path, false)),
            "mp3" => ("mp3", check_sound(&path, true)),
            _ => continue,
        };
        let slot = totals.entry(kind).or_insert((0, 0));
        match result {
            Ok(()) => slot.0 += 1,
            Err(error) => {
                slot.1 += 1;
                failures.push(format!("{}: {}", path.display(), error));
            },
        }
    }
}

fn read(path: &Path) -> Result<Box<[u8]>, String> {
    std::fs::read(path).map(Vec::into_boxed_slice).map_err(|e| e.to_string())
}

fn check_gmspr(path: &Path) -> Result<(), String> {
    let bytes = read(path)?;
    crate::gml::kernel::parse_gmspr(&bytes).map(|_| ())
}

fn check_image(path: &Path) -> Result<(), String> {
    crate::gml::file::load_animation(&path.to_string_lossy(), 1).map(|_| ()).map_err(|e| e.to_string())
}

fn check_midi(path: &Path) -> Result<(), String> {
    let bytes = read(path)?;
    crate::game::audio::midi::check(&bytes)
}

fn check_sound(path: &Path, mp3: bool) -> Result<(), String> {
    let data = read(path)?;
    if mp3 {
        crate::game::audio::check_mp3(data)
    } else {
        crate::game::audio::check_wav(data)
    }
}
