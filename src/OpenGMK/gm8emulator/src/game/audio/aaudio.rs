//! Native Android audio output via AAudio.
//!
//! The rest of the engine feeds a `Mixer` (an `udon::Source`). On desktop, `udon`'s SoundIo
//! session pulls from it; there is no such backend for Android, so here we open an AAudio output
//! stream and drive the very same `Mixer` straight from its data callback.
//!
//! AAudio lives in `libaaudio.so` (linked from the NDK sysroot), so this is plain FFI — no extra
//! crates and no C++ toolchain.

use std::{
    ffi::c_void,
    sync::{
        atomic::{AtomicU32, Ordering},
        Mutex,
    },
    sync::Arc,
};

use udon::source::{ChannelCount, Sample, SampleRate, Source};

use super::mixer::{Mixer, MixerHandle};

// aaudio/AAudio.h
const AAUDIO_OK: i32 = 0;
const AAUDIO_DIRECTION_OUTPUT: i32 = 0;
const AAUDIO_FORMAT_PCM_FLOAT: i32 = 2;
const AAUDIO_PERFORMANCE_MODE_LOW_LATENCY: i32 = 12;
const AAUDIO_CALLBACK_RESULT_CONTINUE: i32 = 0;

/// The mixer currently being pulled by the AAudio callback, if any.
static MIXER: Mutex<Option<Mixer>> = Mutex::new(None);

/// The output stream, so it can be stopped and resumed when the app goes to the background and
/// comes back. Set by [open].
static STREAM: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

const RING_BUFFER_CAPACITY: usize = 16384; // ~170 ms at 48kHz stereo (power of two)
const RING_BUFFER_MASK: usize = RING_BUFFER_CAPACITY - 1;

/// Lock-free circular audio ring buffer storing f32 sample bits.
static RING_BUF: [AtomicU32; RING_BUFFER_CAPACITY] = {
    const INIT: AtomicU32 = AtomicU32::new(0);
    [INIT; RING_BUFFER_CAPACITY]
};

static READ_POS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
static WRITE_POS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
static STREAM_PAUSED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static FEEDER_ACTIVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Stops or resumes the output. Without this the music keeps playing behind the home screen, which
/// is both wrong and how a game that never really paused gives itself away.
pub fn set_paused(paused: bool) {
    STREAM_PAUSED.store(paused, Ordering::Release);
    let stream = STREAM.load(Ordering::Relaxed) as *mut c_void;
    if stream.is_null() {
        return
    }
    unsafe {
        if paused {
            AAudioStream_requestPause(stream);
            // Drop what is already queued, so the silence arrives now instead of draining first.
            AAudioStream_requestFlush(stream);
            // Reset ring buffer positions so resuming starts with fresh audio
            READ_POS.store(0, Ordering::Release);
            WRITE_POS.store(0, Ordering::Release);
        } else {
            let _ = AAudioStream_requestStart(stream);
        }
    }
}

extern "C" {
    fn AAudio_createStreamBuilder(builder: *mut *mut c_void) -> i32;
    fn AAudioStreamBuilder_setDirection(builder: *mut c_void, direction: i32);
    fn AAudioStreamBuilder_setFormat(builder: *mut c_void, format: i32);
    fn AAudioStreamBuilder_setChannelCount(builder: *mut c_void, channel_count: i32);
    fn AAudioStreamBuilder_setSampleRate(builder: *mut c_void, sample_rate: i32);
    fn AAudioStreamBuilder_setPerformanceMode(builder: *mut c_void, mode: i32);
    fn AAudioStreamBuilder_setDataCallback(
        builder: *mut c_void,
        callback: extern "C" fn(*mut c_void, *mut c_void, *mut c_void, i32) -> i32,
        user_data: *mut c_void,
    );
    fn AAudioStreamBuilder_openStream(builder: *mut c_void, stream: *mut *mut c_void) -> i32;
    fn AAudioStreamBuilder_delete(builder: *mut c_void) -> i32;
    fn AAudioStream_requestStart(stream: *mut c_void) -> i32;
    fn AAudioStream_requestPause(stream: *mut c_void) -> i32;
    fn AAudioStream_requestFlush(stream: *mut c_void) -> i32;
    fn AAudioStream_getChannelCount(stream: *mut c_void) -> i32;
    fn AAudioStream_getSampleRate(stream: *mut c_void) -> i32;
}

/// Set once the mixer has produced something audible. The launcher waits for this before handing the
/// screen over: the game paints a small "Loading…" screen well before its first sound.
static SOUND: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Whether anything has actually been heard yet.
pub fn sound_started() -> bool {
    SOUND.load(Ordering::Relaxed)
}

/// Pulls `num_frames` of interleaved f32 audio out of the lock-free ring buffer.
/// AAudio calls this from its real-time audio thread: zero locking, zero wait, constant O(1) time.
extern "C" fn data_callback(
    _stream: *mut c_void,
    _user_data: *mut c_void,
    audio_data: *mut c_void,
    num_frames: i32,
) -> i32 {
    let channels = 2;
    let len = (num_frames.max(0) as usize) * channels;
    // SAFETY: AAudio guarantees `audio_data` points to `num_frames * channels` f32 samples.
    let buffer = unsafe { std::slice::from_raw_parts_mut(audio_data as *mut Sample, len) };

    if STREAM_PAUSED.load(Ordering::Relaxed) {
        buffer.fill(0.0);
        return AAUDIO_CALLBACK_RESULT_CONTINUE;
    }

    let r = READ_POS.load(Ordering::Relaxed);
    let w = WRITE_POS.load(Ordering::Acquire);
    let avail = w.wrapping_sub(r);
    let to_read = len.min(avail);

    for (i, slot) in buffer[..to_read].iter_mut().enumerate() {
        let idx = (r.wrapping_add(i)) & RING_BUFFER_MASK;
        *slot = f32::from_bits(RING_BUF[idx].load(Ordering::Relaxed));
    }
    if to_read < len {
        buffer[to_read..].fill(0.0);
    }
    READ_POS.store(r.wrapping_add(to_read), Ordering::Release);

    // On-screen mute button toggle support
    if crate::touch_controls::is_audio_muted() {
        buffer.fill(0.0);
    }

    // Cheap after the first time: the check is skipped once something has been heard.
    if !SOUND.load(Ordering::Relaxed) && buffer.iter().any(|x| *x != 0.0) {
        SOUND.store(true, Ordering::Relaxed);
    }
    AAUDIO_CALLBACK_RESULT_CONTINUE
}

fn check(result: i32, what: &str) -> Result<(), String> {
    if result == AAUDIO_OK {
        Ok(())
    } else {
        Err(format!("{} fallo (AAudio error {})", what, result))
    }
}

/// Opens the output stream and hands the mixer over to its callback. Returns the raw stream
/// pointer (keep it alive for the process lifetime), the device channel count, its sample rate,
/// and the handle used to add sounds.
pub fn open(global_volume: Arc<AtomicU32>) -> Result<(usize, ChannelCount, SampleRate, MixerHandle), String> {
    unsafe {
        let mut builder: *mut c_void = std::ptr::null_mut();
        check(AAudio_createStreamBuilder(&mut builder), "AAudio_createStreamBuilder")?;

        AAudioStreamBuilder_setDirection(builder, AAUDIO_DIRECTION_OUTPUT);
        AAudioStreamBuilder_setFormat(builder, AAUDIO_FORMAT_PCM_FLOAT);
        AAudioStreamBuilder_setChannelCount(builder, 2);
        AAudioStreamBuilder_setSampleRate(builder, 48000);
        AAudioStreamBuilder_setPerformanceMode(builder, AAUDIO_PERFORMANCE_MODE_LOW_LATENCY);
        AAudioStreamBuilder_setDataCallback(builder, data_callback, std::ptr::null_mut());

        let mut stream: *mut c_void = std::ptr::null_mut();
        let result = AAudioStreamBuilder_openStream(builder, &mut stream);
        AAudioStreamBuilder_delete(builder);
        check(result, "AAudioStreamBuilder_openStream")?;

        let channels = AAudioStream_getChannelCount(stream).max(1) as u32;
        let sample_rate = AAudioStream_getSampleRate(stream).max(8000) as u32;
        let channels = ChannelCount::new(channels as u16).ok_or("canal de audio invalido")?;
        let sample_rate = SampleRate::new(sample_rate).ok_or("frecuencia de audio invalida")?;

        let (mixer, handle) = Mixer::new(sample_rate, channels, global_volume);
        *MIXER.lock().unwrap() = Some(mixer);
        STREAM.store(stream as usize, Ordering::Relaxed);

        FEEDER_ACTIVE.store(true, Ordering::Release);
        let _ = std::thread::Builder::new()
            .name("audio_synthesis".into())
            .spawn(|| {
                let mut chunk = [0.0f32; 512];
                while FEEDER_ACTIVE.load(Ordering::Relaxed) {
                    if STREAM_PAUSED.load(Ordering::Relaxed) {
                        std::thread::sleep(std::time::Duration::from_millis(15));
                        continue;
                    }
                    let r = READ_POS.load(Ordering::Acquire);
                    let w = WRITE_POS.load(Ordering::Relaxed);
                    let occupied = w.wrapping_sub(r);
                    let free = RING_BUFFER_CAPACITY.saturating_sub(occupied);

                    if free >= chunk.len() {
                        let count = if let Ok(mut guard) = MIXER.lock() {
                            if let Some(m) = guard.as_mut() {
                                m.write_samples(&mut chunk)
                            } else {
                                0
                            }
                        } else {
                            0
                        };
                        if count < chunk.len() {
                            chunk[count..].fill(0.0);
                        }
                        for (i, &s) in chunk.iter().enumerate() {
                            let idx = (w.wrapping_add(i)) & RING_BUFFER_MASK;
                            RING_BUF[idx].store(s.to_bits(), Ordering::Relaxed);
                        }
                        WRITE_POS.store(w.wrapping_add(chunk.len()), Ordering::Release);
                    } else {
                        // Buffer has plenty of audio (>150ms), sleep 4ms
                        std::thread::sleep(std::time::Duration::from_millis(4));
                    }
                }
            });

        check(AAudioStream_requestStart(stream), "AAudioStream_requestStart")?;
        eprintln!("[aaudio] salida abierta: {} canales @ {} Hz (lock-free ring buffer activo)", channels.get(), sample_rate.get());
        Ok((stream as usize, channels, sample_rate, handle))
    }
}
