use crate::{source::{ChannelCount, SampleRate, Sample, Source}};

/// A Source which endlessly cycles another Source, calling reset() each time it ends.
///
/// Note that this struct is not necessarily endless - it will exit if given a Source containing 0 samples.
pub struct Cycle<S: Source>(S);

impl<S: Source> Cycle<S> {
    #[inline(always)]
    pub fn new(source: S) -> Self {
        Self(source)
    }
}

impl<S:Source> Source for Cycle<S> {
    #[inline]
    fn channel_count(&self) -> ChannelCount {
        self.0.channel_count()
    }

    #[inline]
    fn sample_rate(&self) -> SampleRate {
        self.0.sample_rate()
    }

    fn write_samples(&mut self, buffer: &mut [Sample]) -> usize {
        let mut written = self.0.write_samples(buffer);
        while written != buffer.len() {
            self.0.reset();
            let newly_written = self.0.write_samples(&mut buffer[written..]);
            if newly_written == 0 {
                return written;
            }
            written += newly_written;
        }
        buffer.len()
    }

    fn reset(&mut self) {
        self.0.reset()
    }
}
