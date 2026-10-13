use crate::{error::Error, source::{ChannelCount, SampleRate, Source}, session};
use std::num::{NonZeroU16, NonZeroU32};

const BUFFER_FRAMES: i64 = 256;

pub struct Device {
    pcm: alsa_rs::pcm::PCM,
    sample_rate: SampleRate,
    channel_count: ChannelCount,
    mmap: bool,
}

impl Device {
    pub fn channel_count(&self) -> ChannelCount {
        self.channel_count
    }

    pub fn sample_rate(&self) -> SampleRate {
        self.sample_rate
    }

    pub fn default_output() -> Result<Self, Error> {
        let req_samplerate = 48000;

        // Open the device
        let pcm = match alsa_rs::PCM::new("default", alsa_rs::Direction::Playback, false) {
            Ok(p) => p,
            Err(_) => return Err(Error::NoOutputDevice),
        };

        // Hardware parameters
        let hwp = alsa_rs::pcm::HwParams::any(&pcm).map_err(|_| Error::Unknown)?;
        hwp.set_channels(2).map_err(|_| Error::DeviceNotUsable)?;
        hwp.set_rate(req_samplerate, alsa_rs::ValueOr::Nearest).map_err(|_| Error::DeviceNotUsable)?;
        hwp.set_format(alsa_rs::pcm::Format::float()).map_err(|_| Error::DeviceNotUsable)?;
        // This next call fails with EINVAL if the device doesn't support mmap
        let mmap = match hwp.set_access(alsa_rs::pcm::Access::MMapInterleaved) {
            Ok(()) => true,
            Err(_) => {
                hwp.set_access(alsa_rs::pcm::Access::RWInterleaved).map_err(|_| Error::DeviceNotUsable)?;
                false
            },
        };
        hwp.set_buffer_size(BUFFER_FRAMES).map_err(|_| Error::DeviceNotUsable)?;
        hwp.set_period_size(BUFFER_FRAMES / 4, alsa_rs::ValueOr::Nearest).map_err(|_| Error::DeviceNotUsable)?;
        pcm.hw_params(&hwp).map_err(|_| Error::DeviceNotUsable)?;
        std::mem::drop(hwp); // because rust

        // Software parameters
        let hwp = pcm.hw_params_current().map_err(|_| Error::Unknown)?;
        let swp = pcm.sw_params_current().map_err(|_| Error::Unknown)?;
        swp.set_start_threshold(hwp.get_buffer_size().map_err(|_| Error::Unknown)?).map_err(|_| Error::DeviceNotUsable)?;
        swp.set_avail_min(hwp.get_period_size().map_err(|_| Error::Unknown)?).map_err(|_| Error::DeviceNotUsable)?;
        pcm.sw_params(&swp).map_err(|_| Error::DeviceNotUsable)?;
        let rate = hwp.get_rate().ok().and_then(NonZeroU32::new).ok_or(Error::DeviceNotUsable)?;
        std::mem::drop(swp); // because rust 2 electric boolgaoo
        std::mem::drop(hwp); // because rust (2016) (remastered for ps5)

        Ok(Self {
            pcm,
            sample_rate: rate,
            channel_count: unsafe { NonZeroU16::new_unchecked(2) },
            mmap,
        })
    }
}

pub struct OutputStream(Device);

impl OutputStream {
    pub fn new(device: session::Device) -> Result<session::OutputStream, Error> {
        match device {
            session::Device(session::DeviceImpl::Alsa(device)) => session_wrap!(Ok(Self(device)), OutputStream(OutputStreamImpl), Alsa),
            _ => todo!(), // TODO: what?
        }
    }

    pub fn play(&self, source: impl Source + Send + 'static) -> Result<(), Error> {
        if self.0.mmap {
            self.play_mmap(source)
        } else {
            self.play_writei(source)
        }
    }

    fn play_writei(&self, mut source: impl Source + Send + 'static) -> Result<(), Error> {
        let channel_count = usize::from(u16::from(self.0.channel_count));
        let buf_len = (BUFFER_FRAMES as usize) * channel_count;
        let mut buf: Vec<f32> = Vec::with_capacity(buf_len);
        unsafe { buf.set_len(buf_len); }
        let mut buf_start = 0;
        let mut buf_end = source.write_samples(&mut buf);
        let io = self.0.pcm.io_f32().map_err(|_| Error::Unknown)?;

        loop {
            match io.writei(&buf[buf_start..buf_end]) {
                Ok(n) if n * channel_count == buf_end - buf_start => {
                    if buf_end != buf_len {
                        break;
                    }
                    buf_start = 0;
                    buf_end = source.write_samples(&mut buf);
                },
                Ok(n) => {
                    // "The returned number of frames can be less only if a signal or underrun occurred." - alsa docs
                    buf_start = n * channel_count;
                },
                Err(e) => {
                    // Underruns can and do happen here, so we need to try to recover from them...
                    // TODO: handle this next error properly, could be unplug etc
                    self.0.pcm.try_recover(e, true).map_err(|_| Error::Unknown)?;
                },
            }
        }

        self.0.pcm.drain().map_err(|_| Error::Unknown)?;
        self.0.pcm.reset().map_err(|_| Error::Unknown)?;
        Ok(())
    }

    fn play_mmap(&self, _source: impl Source + Send + 'static) -> Result<(), Error> {
        todo!()
    }
}

pub struct Session;

impl Session {
    pub fn new() -> Result<Self, Error> {
        Ok(Self)
    }

    pub fn default_output_device(&self) -> Result<session::Device, Error> {
        session_wrap!(Device::default_output(), Device(DeviceImpl), Alsa)
    }

    pub fn open_output_stream(
        &self,
        device: session::Device,
    ) -> Result<session::OutputStream, Error> {
        OutputStream::new(device)
    }
}
