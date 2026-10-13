use crate::source::{ChannelCount, Sample, SampleRate, Source};
use std::{
    marker::PhantomData,
    sync::{Arc, Condvar, Mutex},
};

const DEF_BUFFER_SIZE: usize = 4800;

/// Adds threaded buffering to any Source object using an internal ring buffer.
///
/// It should be noted that a Buffer will induce filter delay proportional to the buffer size.
/// For example, the default size of 4800 samples will cause a 50 millisecond filter delay, assuming it's connected to
/// a 48000 Hz and 2-channel output. Doubling the buffer size will double the resulting filter delay.
///
/// For consistent filter delay across systems, you may want to use a buffer size calculated from output variables,
/// for example: `Buffer::with_capacity(sample_rate * channel_count / 20)`
pub struct Buffer<S>
where
    S: Source + Send + 'static,
{
    _source: PhantomData<S>,
    channel_count: ChannelCount,
    sample_rate: SampleRate,
    buffer: Arc<(Mutex<RingBuffer>, Condvar)>,
}

struct RingBuffer {
    data: Box<[Sample]>,
    index: usize,
    len: usize,
    samples_remaining: Option<usize>,
    dropped: bool,
}

impl<S> Buffer<S>
where
    S: Source + Send + 'static,
{
    /// Creates a new Buffer with the given source and the default buffer size (currently 4800, but this may change.)
    /// Use with_capacity() instead to specify a custom buffer size. Buffer size cannot be changed after creation.
    pub fn new(source: S) -> Self {
        Self::with_capacity(source, DEF_BUFFER_SIZE)
    }

    /// Creates a new Buffer with the given source and internal buffer capacity.
    /// Buffer capacity will never change (and thus, cannot be changed) after creation.
    pub fn with_capacity(mut source: S, capacity: usize) -> Self {
        let channel_count = source.channel_count();
        let sample_rate = source.sample_rate();
        let mut buffer = Vec::with_capacity(capacity);
        unsafe {
            buffer.set_len(capacity);
        }

        let ring_buffer = Arc::new((
            Mutex::new(RingBuffer {
                data: buffer.into_boxed_slice(),
                index: 0,
                len: 0,
                samples_remaining: None,
                dropped: false,
            }),
            Condvar::new(),
        ));
        let ring_buffer_clone = ring_buffer.clone();

        std::thread::spawn(move || {
            let mut back_buffer = Vec::with_capacity(capacity);

            let (b, cvar) = &*ring_buffer_clone;
            let mut ring_buffer = b.lock().unwrap();

            loop {
                // Wait until we're notified that we need to fill the buffer
                while ring_buffer.len >= ring_buffer.data.len() {
                    ring_buffer = cvar.wait(ring_buffer).unwrap();

                    // Stop if the Buffer was dropped
                    if ring_buffer.dropped {
                        break
                    }
                }

                // Fill ring buffer from source
                let samples_missing = ring_buffer.data.len() - ring_buffer.len;
                unsafe {
                    // SAFETY: back_buffer is created with the same capacity as ring_buffer.data's length,
                    // and here we are setting length to ring_buffer.data.len() at most
                    back_buffer.set_len(samples_missing);
                }
                let written_count = source.write_samples(&mut back_buffer);
                if written_count < samples_missing {
                    unsafe {
                        // SAFETY: samples_missing is a safe length as explained above, and we've also verified that
                        // written_count is lower than samples_missing
                        back_buffer.set_len(written_count);
                    }
                    ring_buffer.samples_remaining = Some(ring_buffer.len + written_count);
                }

                let write_index = (ring_buffer.index + ring_buffer.len) % ring_buffer.data.len();
                if let Some(data) = ring_buffer.data.get_mut(write_index..(write_index + written_count)) {
                    data.copy_from_slice(&back_buffer);
                } else {
                    let split_index = ring_buffer.data.len() - write_index;
                    let (back1, back2) = back_buffer.split_at(split_index);
                    ring_buffer.data[write_index..].copy_from_slice(back1);
                    ring_buffer.data[..back2.len()].copy_from_slice(back2);
                }
                ring_buffer.len = ring_buffer.data.len();

                if ring_buffer.samples_remaining.is_some() {
                    break
                }
            }
        });

        Self { _source: PhantomData, channel_count, sample_rate, buffer: ring_buffer }
    }
}

impl<S> Source for Buffer<S>
where
    S: Source + Send + 'static,
{
    #[inline]
    fn channel_count(&self) -> ChannelCount {
        self.channel_count
    }

    #[inline]
    fn sample_rate(&self) -> SampleRate {
        self.sample_rate
    }

    fn write_samples(&mut self, mut output_buffer: &mut [Sample]) -> usize {
        loop {
            // Lock access to ring buffer
            let mut ring_buffer = self.buffer.0.lock().unwrap();

            // Check if there's enough data in the ring buffer to totally fill the output
            let output = if let Some(s) = &mut ring_buffer.samples_remaining {
                let samples = output_buffer.len().min(*s);
                *s -= samples;
                let (out1, out2) = output_buffer.split_at_mut(samples);
                output_buffer = out2;
                out1
            } else {
                &mut *output_buffer
            };

            // Short circuit if there are 0 samples left to play
            if output.len() == 0 {
                break 0
            }

            if ring_buffer.len >= output.len() {
                // There's enough data to complete this request, so write and return
                ring_buffer.write_to(output);
                self.buffer.1.notify_one();
                break output.len()
            } else {
                // There's some data in the buffer, but not enough.
                // This will most likely happen if the buffer is smaller than the read being performed.
                // Write it to the output, truncate the output slice, then continue looping to wait for more data
                let sample_count = ring_buffer.len;
                ring_buffer.write_to(&mut output[..sample_count]);
                self.buffer.1.notify_one();
                output_buffer = &mut output_buffer[sample_count..];
            }
        }
    }
}

impl<S> Drop for Buffer<S>
where
    S: Source + Send + 'static,
{
    fn drop(&mut self) {
        let (b, cvar) = &*self.buffer;
        let mut ring_buffer = b.lock().unwrap();
        ring_buffer.dropped = true;
        cvar.notify_one();
    }
}

impl RingBuffer {
    // Only meant as a helper fn. Assumes there's enough data in the ring buffer to fill the output.
    fn write_to(&mut self, output: &mut [Sample]) {
        // Check if we need 1 or 2 memcpy's due to the ring buffer looping round
        if let Some(data) = self.data.get(self.index..(self.index + output.len())) {
            // Just one memcpy
            output.copy_from_slice(data);

            // Remove data from the front of the ring buffer
            self.index = (self.index + output.len()) % self.data.len();
            self.len -= output.len();
        } else {
            // Two memcpy's
            let new_index = self.index + output.len() - self.data.len();
            let (output1, output2) = output.split_at_mut(self.data.len() - self.index);
            output1.copy_from_slice(&self.data[self.index..]);
            output2.copy_from_slice(&self.data[..new_index]);

            // Remove data from the front of the ring buffer
            self.index = new_index;
            self.len -= output.len();
        }
    }
}
