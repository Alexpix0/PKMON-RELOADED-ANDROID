// Temporary diagnostic: decodes a .gmspr and prints the first frame's first pixels, so the
// channel order (RGBA vs BGRA) can be checked against a known colour.
use gm8exe::{asset::Asset, asset::Sprite};
use std::io::Read;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let bytes = std::fs::read(&args[1]).expect("read .gmspr");
    let magic = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    println!("magic = {:#x} (esperado 0x12d591)", magic);

    let mut inflated = Vec::new();
    flate2::read::ZlibDecoder::new(&bytes[8..]).read_to_end(&mut inflated).expect("inflate");

    let mut with_name = Vec::new();
    with_name.extend_from_slice(&0u32.to_le_bytes());
    with_name.extend_from_slice(&inflated);
    let sprite = Sprite::deserialize_exe(&with_name[..], gm8exe::GameVersion::GameMaker8_0, false).expect("parse");

    println!("origin = ({}, {}), frames = {}", sprite.origin_x, sprite.origin_y, sprite.frames.len());
    if let Some(f) = sprite.frames.first() {
        println!("frame0 = {}x{}, {} bytes", f.width, f.height, f.data.len());
        let mut shown = 0;
        for p in 0..f.data.len() / 4 {
            let o = p * 4;
            let a = f.data[o + 3];
            if a > 0 {
                println!("  pixel[{}] = [{}, {}, {}, {}]", p, f.data[o], f.data[o + 1], f.data[o + 2], a);
                shown += 1;
                if shown >= 8 {
                    break
                }
            }
        }
    }
}
