use crate::{source::{ChannelCount, SampleRate, Sample, Source}};
use std::{sync::Arc, sync::{atomic::{AtomicBool, Ordering}, mpsc::{self, Receiver, Sender}}};

const INIT_CAPACITY: usize = 16;

/// A simple additive mixer. Construct with `Mixer::new()`. This will return a Mixer and a MixerHandle.
/// The Mixer is a Source object, and intended to be attached (directly or indirectly) to an OutputStream
/// or any other place where a Source is expected.
/// The MixerHandle is kept and used for dynamically adding Sources to the Mixer.
pub struct Mixer {
    channels: ChannelCount,
    sample_rate: SampleRate,
    sources: Vec<(Box<dyn Source + Send + 'static>, Arc<SoundInfo>)>,
    input_buffer: Vec<Sample>,
    receiver: Receiver<(Box<dyn Source + Send + 'static>, Arc<SoundInfo>)>,
}

/// Returned from Mixer::new(), and permanently associated with the Mixer created alongside it.
/// Used for dynamically adding sounds to the Mixer with `handle.add()`
pub struct MixerHandle(Sender<(Box<dyn Source + Send + 'static>, Arc<SoundInfo>)>);

/// Error type for Mixer calls
#[derive(Debug, Clone, Copy)]
pub enum Error {
    /// Indicates that something could not be sent to the Mixer via a MixerHandle.
    /// This usually happens because the Mixer no longer exists.
    SendError,
}

impl Mixer {
    /// Constructs a new Mixer and MixerHandle.
    ///
    /// `Mixer` does not make any changes to the channel count or sample rate of its Sources. As such, it needs to know
    /// its expected output rate and channel count on construction.
    ///
    /// If Sources with a different sample rate or channel count than this are subsequently added to the Mixer,
    /// they will sound wrong. For resampling and rechanneling, see `Resampler` and `Rechanneler`.
    pub fn new(sample_rate: SampleRate, channels: ChannelCount) -> (Self, MixerHandle) {
        let (sender, receiver) = mpsc::channel();
        (
            Self { channels, sample_rate, sources: Vec::with_capacity(INIT_CAPACITY), input_buffer: Vec::new(), receiver },
            MixerHandle(sender),
        )
    }
}

impl Source for Mixer {
    fn write_samples(&mut self, buffer: &mut [Sample]) -> usize {
        buffer.iter_mut().for_each(|x| *x = 0.0);

        // Check for new sources...
        while let Ok(source) = self.receiver.try_recv() {
            self.sources.push(source);
        }

        let input_buffer = &mut self.input_buffer;

        self.sources.retain_mut(|(source, info)| {
            if info.stop.load(Ordering::Acquire) {
                info.running.store(false, Ordering::Release);
                false
            } else {
                input_buffer.resize_with(buffer.len(), Default::default);
                let count = source.write_samples(input_buffer);

                for (in_sample, out_sample) in input_buffer.iter().take(count).copied().zip(buffer.iter_mut()) {
                    *out_sample += in_sample;
                }

                let running = count == input_buffer.len();
                info.running.store(running, Ordering::Release);
                running
            }
        });

        buffer.len()
    }

    fn channel_count(&self) -> ChannelCount {
        self.channels
    }

    fn sample_rate(&self) -> SampleRate {
        self.sample_rate
    }

    fn reset(&mut self) {
        self.sources.clear()
    }
}

impl MixerHandle {
    /// Adds a Source to the Mixer associated with this handle. The Mixer will play the Source until it ends,
    /// then discard it.
    pub fn add(&self, source: impl Source + Send + 'static) -> Result<SoundHandle, Error> {
        let arc = Arc::new(SoundInfo {
            running: AtomicBool::new(true),
            stop: AtomicBool::new(false),
        });

        self.0.send((Box::new(source), arc.clone())).map_err(|_| Error::SendError)?;
        Ok(SoundHandle(arc))
    }
}

struct SoundInfo {
    running: AtomicBool,
    stop: AtomicBool,
}

pub struct SoundHandle(Arc<SoundInfo>);

impl SoundHandle {
    #[inline(always)]
    pub fn is_running(&self) -> bool {
        self.0.running.load(Ordering::Acquire)
    }

    #[inline(always)]
    pub fn stop(&self) {
        self.0.stop.store(true, Ordering::Release)
    }
}

trait RetainMut<T> {
    fn retain_mut(&mut self, f: impl FnMut(&mut T) -> bool);
}

impl<T> RetainMut<T> for Vec<T> {
    fn retain_mut(&mut self, mut f: impl FnMut(&mut T) -> bool) {
        let len = self.len();
        let mut del = 0;
        {
            let v = &mut **self;

            for i in 0..len {
                if !f(&mut v[i]) {
                    del += 1;
                } else if del > 0 {
                    v.swap(i - del, i);
                }
            }
        }
        if del > 0 {
            self.truncate(len - del);
        }
    }
}
