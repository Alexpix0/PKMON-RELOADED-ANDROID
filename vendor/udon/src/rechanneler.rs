use crate::source::{ChannelCount, Sample, SampleRate, Source};

/// Converts the number of channels in a Source to the target channel count.
///
/// Channel mixing strategy is as follows:
/// - if input and output channel count is the same, pass straight through
/// - otherwise: take the average input sample from each frame and duplicate it across each output frame
///
/// This suffices for most use-cases, but a more specialized strategy would be needed for mixing a multi-channel input
/// to a >2 channel output (5-channel multimedia, for example) non-naively.
pub struct Rechanneler<S>
where
    S: Source,
{
    source: S,
    source_channels: ChannelCount,
    target_channels: ChannelCount,
    buffer: Vec<Sample>,
}

impl<S> Rechanneler<S>
where
    S: Source,
{
    pub fn new(source: S, target_channels: ChannelCount) -> Self {
        let source_channels = source.channel_count();
        Self { source, source_channels, target_channels, buffer: Vec::new() }
    }
}

impl<S> Source for Rechanneler<S>
where
    S: Source,
{
    #[inline(always)]
    fn sample_rate(&self) -> SampleRate {
        self.source.sample_rate()
    }

    #[inline(always)]
    fn channel_count(&self) -> ChannelCount {
        self.target_channels
    }

    fn write_samples(&mut self, buffer: &mut [Sample]) -> usize {
        if self.source_channels == self.target_channels {
            self.source.write_samples(buffer)
        } else {
            let from: usize = self.source_channels.get().into();
            let to: usize = self.target_channels.get().into();

            self.buffer.resize_with(buffer.len() * from / to, Default::default);
            let written_count = self.source.write_samples(&mut self.buffer);
            let iter = self.buffer.chunks_exact(from).take(written_count / from).zip(buffer.chunks_exact_mut(to));
            let frame_count = iter.len();

            for (in_samples, out_samples) in iter {
                let sample = in_samples.iter().sum::<Sample>() / in_samples.len() as Sample;
                out_samples.iter_mut().for_each(|x| *x = sample);
            }

            frame_count * to
        }
    }

    fn reset(&mut self) {
        self.source.reset()
    }
}
