// Temporary diagnostic: parses every .gmspr under a directory with the real parser and counts
// how many succeed, so a format fix can be checked against the whole game.
use gm8exe::{asset::Asset, asset::Sprite};
use std::io::Read;

fn walk(dir: &std::path::Path, ok: &mut u32, bad: &mut u32, failures: &mut Vec<String>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, ok, bad, failures);
            continue
        }
        if path.extension().map(|e| e != "gmspr").unwrap_or(true) {
            continue
        }
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(_) => continue,
        };
        let good = bytes.len() >= 8
            && u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) == 0x0012_D591
            && {
                let mut inflated = Vec::new();
                flate2::read::ZlibDecoder::new(&bytes[8..]).read_to_end(&mut inflated).is_ok() && {
                    let mut with_name = Vec::with_capacity(4 + inflated.len());
                    with_name.extend_from_slice(&0u32.to_le_bytes());
                    with_name.extend_from_slice(&inflated);
                    Sprite::deserialize_exe(&with_name[..], gm8exe::GameVersion::GameMaker8_0, false).is_ok()
                }
            };
        if good {
            *ok += 1;
        } else {
            *bad += 1;
            if failures.len() < 10 {
                failures.push(path.display().to_string());
            }
        }
    }
}

fn main() {
    let dir = std::env::args().nth(1).expect("dir");
    let (mut ok, mut bad) = (0u32, 0u32);
    let mut failures = Vec::new();
    walk(std::path::Path::new(&dir), &mut ok, &mut bad, &mut failures);
    println!("ok={} fallan={}", ok, bad);
    for failure in failures {
        println!("  {failure}")
    }
}
