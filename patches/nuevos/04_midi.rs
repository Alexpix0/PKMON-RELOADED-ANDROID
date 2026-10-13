// GameMaker 8 stores "BackgroundMusic" and "Multimedia" sounds as MIDI on Windows, where they
// were played through the system media player. Since they're commonly used for background music,
// they are synthesized to PCM here on load, which lets them go through the same WAV path as
// everything else - no changes needed to the mixer, volume, looping or `sound_isplaying`.

use rustysynth::{MidiFile, MidiFileSequencer, SoundFont, Synthesizer, SynthesizerSettings};
use std::{
    io::Cursor,
    path::PathBuf,
    sync::Arc,
};

const SAMPLE_RATE: u32 = 44100;
const CHANNELS: u16 = 2;
/// Safety valve for MIDI files with no reliable length, so a bad file can't hang the game.
const MAX_RENDER_SECONDS: usize = 600;
const RENDER_BLOCK: usize = 4096;

/// Where to look for a GM SoundFont. Users can override with the `OPENGMK_SOUNDFONT` env var.
fn soundfont_candidates() -> Vec<PathBuf> {
    if let Some(path) = std::env::var_os("OPENGMK_SOUNDFONT") {
        return vec![PathBuf::from(path)]
    }
    vec![
        PathBuf::from("/usr/share/sounds/sf2/default-GM.sf2"),
        PathBuf::from("/usr/share/sounds/sf2/TimGM6mb.sf2"),
        PathBuf::from("/usr/share/soundfonts/default-GM.sf2"),
        PathBuf::from("/usr/share/soundfonts/FluidR3_GM.sf2"),
        PathBuf::from("/usr/share/sounds/sf2/FluidR3_GM.sf2"),
    ]
}

fn load_soundfont() -> Option<Vec<u8>> {
    for path in soundfont_candidates() {
        match std::fs::read(&path) {
            Ok(bytes) => return Some(bytes),
            Err(e) => eprintln!("could not load soundfont '{}': {}", path.display(), e),
        }
    }
    eprintln!(
        "no SoundFont (.sf2) found for MIDI playback - install one, e.g. `fluidsynth` or \
         `timgm6mb-soundfont`, or point OPENGMK_SOUNDFONT at one"
    );
    None
}

#[inline]
fn f32_to_i16(sample: f32) -> i16 {
    let clamped = if sample > 1.0 {
        1.0
    } else if sample < -1.0 {
        -1.0
    } else {
        sample
    };
    (clamped * 32767.0) as i16
}

fn build_wav(samples: &[i16], sample_rate: u32, channels: u16) -> Vec<u8> {
    let data_len = samples.len() * 2;
    let mut wav = Vec::with_capacity(44 + data_len);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&((36 + data_len) as u32).to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
    wav.extend_from_slice(&channels.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&(sample_rate * u32::from(channels) * 2).to_le_bytes()); // byte rate
    wav.extend_from_slice(&(channels * 2).to_le_bytes()); // block align
    wav.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&(data_len as u32).to_le_bytes());
    for sample in samples {
        wav.extend_from_slice(&sample.to_le_bytes());
    }
    wav
}

/// Renders a standard MIDI file to a 16-bit PCM WAV held in memory, ready to be handed to the
/// existing WAV player. Returns `None` if the data, a SoundFont, or the synthesizer can't be used.
pub fn render_to_wav(data: &[u8]) -> Option<Vec<u8>> {
    let soundfont = load_soundfont()?;
    let sf = Arc::new(SoundFont::new(&mut Cursor::new(&soundfont)).ok()?);
    let settings = SynthesizerSettings::new(i32::try_from(SAMPLE_RATE).ok()?);
    let synth = Synthesizer::new(&sf, &settings).ok()?;

    let midi = Arc::new(MidiFile::new(&mut Cursor::new(data)).ok()?);
    let mut sequencer = MidiFileSequencer::new(synth);
    sequencer.play(&midi, false);

    let mut samples: Vec<i16> = Vec::new();
    let mut left = vec![0f32; RENDER_BLOCK];
    let mut right = vec![0f32; RENDER_BLOCK];
    let sample_limit = MAX_RENDER_SECONDS * SAMPLE_RATE as usize * usize::from(CHANNELS);
    while !sequencer.end_of_sequence() && samples.len() < sample_limit {
        left.iter_mut().for_each(|x| *x = 0.0);
        right.iter_mut().for_each(|x| *x = 0.0);
        sequencer.render(&mut left, &mut right);
        for (l, r) in left.iter().zip(right.iter()) {
            samples.push(f32_to_i16(*l));
            samples.push(f32_to_i16(*r));
        }
    }

    if samples.is_empty() {
        eprintln!("MIDI rendered zero samples");
        return None
    }
    Some(build_wav(&samples, SAMPLE_RATE, CHANNELS))
}
