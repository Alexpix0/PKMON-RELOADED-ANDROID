// GameMaker 8 stores "BackgroundMusic" and "Multimedia" sounds as MIDI on Windows, where they
// were played through the system media player. Since they're commonly used for background music,
// they are synthesized to PCM here, which lets them go through the same mixer as everything else.
//
// The whole song used to be rendered up front, the moment the game changed the music. A song is
// minutes long, so that froze the game for seconds every time the player crossed into another
// zone or a battle started. Nothing is rendered ahead of time any more: [MidiSource] synthesizes
// one mixer block at a time, as playback advances, which is a fraction of a millisecond per block.

use rustysynth::{MidiFile, MidiFileSequencer, SoundFont, Synthesizer, SynthesizerSettings};
use serde::{Deserialize, Serialize};
use std::{
    io::Cursor,
    panic::AssertUnwindSafe,
    path::PathBuf,
    sync::{atomic::AtomicU32, Arc},
};
use udon::source::{ChannelCount, Sample, SampleRate, Source};

use super::{make_volume, SoundParams};

const SAMPLE_RATE: u32 = 44100;
const CHANNELS: u16 = 2;
/// Scratch space for one mixer block, so the first callback after a song change does not have to
/// grow the buffers.
const INITIAL_BLOCK: usize = 4096;

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

/// The parsed SoundFont, shared by every synthesizer.
///
/// Parsing it costs real time (the file is several megabytes) and it was being parsed again for
/// every single song, on top of the synthesis. It never changes, so it is parsed once.
fn shared_soundfont() -> Option<Arc<SoundFont>> {
    static SOUNDFONT: std::sync::OnceLock<Option<Arc<SoundFont>>> = std::sync::OnceLock::new();
    SOUNDFONT
        .get_or_init(|| {
            let bytes = load_soundfont()?;
            Some(Arc::new(SoundFont::new(&mut Cursor::new(&bytes)).ok()?))
        })
        .clone()
}

/// Parses a MIDI file. A few of the game's files are RIFF/RMID (a bare MIDI wrapped in a RIFF
/// container), which the parser rejects, so skip to the embedded MThd chunk when there is one.
fn parse(data: &[u8]) -> Result<MidiFile, rustysynth::MidiFileError> {
    let data = match data.windows(4).position(|window| window == b"MThd") {
        Some(offset) => &data[offset..],
        None => data,
    };
    MidiFile::new(&mut Cursor::new(data))
}

/// How long a parsed file lasts, in interleaved samples - the unit the sound handles measure
/// themselves in.
fn length_in_samples(midi: &MidiFile) -> usize {
    // `get_length` unwraps the time of the last event, so a file whose only events are tempo
    // changes panics there. It is worth no more than the silence it would play anyway.
    let seconds = std::panic::catch_unwind(AssertUnwindSafe(|| midi.get_length())).unwrap_or(0.0);
    let frames = (seconds.max(0.0) * f64::from(SAMPLE_RATE)) as usize;
    frames * usize::from(CHANNELS)
}

/// Validates a MIDI file without rendering it. Rendering takes seconds per song, which is far too
/// slow for the `--scan-assets` diagnostic.
pub(crate) fn check(data: &[u8]) -> Result<(), String> {
    parse(data).map(|_| ()).map_err(|e| format!("{:?}", e))
}

/// A MIDI file that is ready to play, and that can be played more than once.
///
/// The parsed file is deliberately not kept: the raw bytes are what get cloned and serialized
/// (savestates carry them like any other sound), and parsing again costs a fraction of a
/// millisecond, while keeping live synthesis state around cannot be serialized at all.
#[derive(Clone, Serialize, Deserialize)]
pub struct MidiHandle {
    data: Box<[u8]>,
    pub(super) params: Arc<SoundParams>,
    pub(super) id: i32,
    pub(super) exclusive: bool,
    length: usize,
}

impl MidiHandle {
    /// Loads a MIDI file, checking that it can be played and measuring how long it lasts.
    pub fn new(data: Box<[u8]>, sound_id: i32, volume: f64, exclusive: bool) -> Option<Self> {
        let length = length_in_samples(&parse(&data).ok()?);
        Some(Self {
            data,
            params: Arc::new(SoundParams { volume: AtomicU32::new(make_volume(volume).to_bits()) }),
            id: sound_id,
            exclusive,
            length,
        })
    }

    pub fn sample_rate(&self) -> u32 {
        SAMPLE_RATE
    }

    pub fn channel_count(&self) -> u16 {
        CHANNELS
    }

    /// The length of the file in interleaved samples, used to work out when playback ends.
    pub fn length(&self) -> usize {
        self.length
    }

    pub fn set_volume(&self, vol: f64) {
        self.params.volume.store(make_volume(vol).to_bits(), std::sync::atomic::Ordering::Release);
    }

    /// A fresh playback of this file. Every play gets its own sequencer, so the same sound can be
    /// played again while an earlier copy is still going.
    pub fn source(&self, looping: bool) -> Option<MidiSource> {
        MidiSource::new(&self.data, looping)
    }
}

/// Synthesizes a MIDI file block by block, as the mixer pulls samples.
pub struct MidiSource {
    sequencer: MidiFileSequencer,
    midi: Arc<MidiFile>,
    /// The file was started with looping on, so it never reaches the end.
    looping: bool,
    /// The sequence finished and the last block has already been handed over; the next call
    /// reports the end so the mixer drops this source.
    finished: bool,
    left: Vec<f32>,
    right: Vec<f32>,
}

impl MidiSource {
    fn new(data: &[u8], looping: bool) -> Option<Self> {
        let soundfont = shared_soundfont()?;
        let settings = SynthesizerSettings::new(i32::try_from(SAMPLE_RATE).ok()?);
        let synthesizer = Synthesizer::new(&soundfont, &settings).ok()?;
        let midi = Arc::new(parse(data).ok()?);

        let mut sequencer = MidiFileSequencer::new(synthesizer);
        sequencer.play(&midi, looping);

        Some(Self {
            sequencer,
            midi,
            looping,
            finished: false,
            left: vec![0.0; INITIAL_BLOCK],
            right: vec![0.0; INITIAL_BLOCK],
        })
    }
}

impl Source for MidiSource {
    fn channel_count(&self) -> ChannelCount {
        ChannelCount::new(CHANNELS).unwrap()
    }

    fn sample_rate(&self) -> SampleRate {
        SampleRate::new(SAMPLE_RATE).unwrap()
    }

    fn write_samples(&mut self, buffer: &mut [Sample]) -> usize {
        let channels = usize::from(CHANNELS);
        let frames = buffer.len() / channels;
        if frames == 0 || self.finished {
            return 0
        }

        self.left.resize(frames, 0.0);
        self.right.resize(frames, 0.0);
        self.left.fill(0.0);
        self.right.fill(0.0);
        self.sequencer.render(&mut self.left, &mut self.right);

        for (frame, (left, right)) in self.left.iter().zip(self.right.iter()).enumerate() {
            buffer[frame * channels] = *left;
            buffer[frame * channels + 1] = *right;
        }
        // The mixer asks for whole frames, but never leave anything it did not ask for unwritten.
        for sample in &mut buffer[frames * channels..] {
            *sample = 0.0;
        }

        if !self.looping && self.sequencer.end_of_sequence() {
            // This block still holds the tail of the song, so it is handed over and the end is
            // reported on the following call, once it has been heard.
            self.finished = true;
        }

        buffer.len()
    }

    fn reset(&mut self) {
        self.sequencer.play(&self.midi, self.looping);
        self.finished = false;
    }
}
