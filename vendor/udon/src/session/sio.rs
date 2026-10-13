use crate::{error::Error, session, source::{ChannelCount, SampleRate, Source}};
use std::{sync::Mutex, os::raw::c_int};
use libsoundio_sys::*;

use super::DeviceImpl;

pub struct Device {
    sio: *mut SoundIo,
    handle: *mut SoundIoDevice,
    channel_count: c_int,
    sample_rate: c_int,
    format: SoundIoFormat,
    layout: SoundIoChannelLayout,
}
unsafe impl Send for Device {}
unsafe impl Sync for Device {}

pub struct OutputStream {
    sio: *mut SoundIo,
    device: *mut SoundIoDevice,
    stream: Mutex<*mut SoundIoOutStream>,
}
unsafe impl Send for OutputStream {}
unsafe impl Sync for OutputStream {}

pub struct Session(*mut SoundIo);
unsafe impl Send for Session {}
unsafe impl Sync for Session {}

impl Device {
    pub fn channel_count(&self) -> ChannelCount {
        unsafe { ChannelCount::new_unchecked(self.channel_count as _) }
    }

    pub fn sample_rate(&self) -> SampleRate {
        unsafe { SampleRate::new_unchecked(self.sample_rate as _) }
    }
}

impl std::ops::Drop for Device {
    fn drop(&mut self) {
        unsafe {
            let Self { handle, .. } = *self;
            soundio_device_unref(handle);
        }
    }
}

impl Session {
    pub fn new() -> Result<Self, Error> {
        unsafe {
            let sio = soundio_create();
            if sio.is_null() {
                return Err(Error::OutOfMemory);
            }
            let err = soundio_connect(sio);
            if err != 0 {
                panic!("failed to connect: {}", std::ffi::CStr::from_ptr(soundio_strerror(err)).to_str().unwrap());
            }
            soundio_flush_events(sio);
            Ok(Self(sio))
        }
    }

    pub fn default_output_device(&self) -> Result<session::Device, Error> {
        unsafe {
            let Self(sio) = *self;
            let index = soundio_default_output_device_index(sio);
            if index < 0 {
                return Err(Error::NoOutputDevice);
            }
            let device = soundio_get_output_device(sio, index);
            if device.is_null() {
                return Err(Error::OutOfMemory);
            }
            if (*device).probe_error != SoundIoError::SoundIoErrorNone as _ {
                return Err(Error::DeviceNotUsable);
            }
            let mut i = 0;
            soundio_sort_channel_layouts((*device).layouts, (*device).layout_count);
            let mut channel_count: Option<c_int> = None;
            let mut lly: Option<SoundIoChannelLayout> = None;
            let mut ly = (*device).layouts;
            while i < (*device).layout_count {
                let lyt = &*ly;
                if lyt.channel_count == 2 {
                    channel_count = Some(2);
                    lly = Some(*lyt);
                    break;
                } else if let Some(c) = channel_count {
                    if c < lyt.channel_count {
                        channel_count = Some(lyt.channel_count);
                        lly = Some(*lyt);
                    }
                } else {
                    channel_count = Some(lyt.channel_count);
                    lly = Some(*lyt);
                }
                ly = ly.offset(1);
                i += 1;
            }
            let channel_count = match channel_count {
                Some(n) => n,
                None => return Err(Error::DeviceNotUsable),
            };
            let layout = lly.unwrap();
            if (*device).sample_rates.is_null() || (*device).sample_rate_count == 0 {
                return Err(Error::DeviceNotUsable);
            }
            i = 0;
            let mut sample_rate: Option<c_int> = None;
            let mut sr = (*device).sample_rates;
            while i < (*device).sample_rate_count {
                let srr = &*sr;
                if let Some(x) = sample_rate {
                    sample_rate = Some(srr.max.min(48000).max(x));
                } else {
                    sample_rate = Some(srr.max.min(48000));
                }
                sr = sr.offset(1);
                i += 1;
            }
            let sample_rate = match sample_rate {
                Some(n) => n,
                None => return Err(Error::DeviceNotUsable),
            };
            let mut format: Option<SoundIoFormat> = None;
            let mut ff = (*device).formats;
            i = 0;
            while i < (*device).format_count {
                let f = *ff;
                if let Some(x) = format {
                    if (x as u32) < f as u32 && (f as u32) < SoundIoFormat::SoundIoFormatFloat64LE as u32 {
                        format = Some(f);
                    }
                } else {
                    format = Some(f);
                }
                ff = ff.offset(1);
                i += 1;
            }
            let format = match format {
                Some(n) => n,
                None => return Err(Error::DeviceNotUsable),
            };
            session_wrap!(Ok(Device {
                sio,
                handle: device,
                channel_count,
                sample_rate,
                layout,
                format,
            }), Device(DeviceImpl), SoundIo)
        }
    }

    pub fn open_output_stream(
        &self,
        device: session::Device,
    ) -> Result<session::OutputStream, Error> {
        unsafe {
            // Rust!
            let device = match device {
                session::Device(DeviceImpl::SoundIo(device)) => device,
                _ => unreachable!(),
            };
            let outstream = soundio_outstream_create(device.handle);
            if outstream.is_null() {
                return Err(Error::OutOfMemory);
            }
            (*outstream).format = if cfg!(target_endian = "little") {
                SoundIoFormat::SoundIoFormatFloat32LE
            } else {
                SoundIoFormat::SoundIoFormatFloat32BE
            };
            (*outstream).write_callback = udon_callback;
            (*outstream).name = "OpenGMK\0".as_ptr().cast();
            (*outstream).sample_rate = device.sample_rate;
            (*outstream).layout = device.layout;
            (*outstream).format = device.format;
            (*outstream).software_latency = 0.01;
            let err = soundio_outstream_open(outstream);
            if err != 0 {
                panic!("failed to open outstream: {}", std::ffi::CStr::from_ptr(soundio_strerror(err)).to_str().unwrap());
            }
            if (*outstream).layout_error != 0 {
                panic!("unable to set channel layout: {}", std::ffi::CStr::from_ptr(soundio_strerror((*outstream).layout_error)).to_str().unwrap());
            }
            assert_eq!((*outstream).sample_rate, device.sample_rate);
            assert_eq!((*outstream).layout.channel_count, device.layout.channel_count);
            assert_eq!((*outstream).format as u32, device.format as u32);
            soundio_device_ref(device.handle);
            session_wrap!(Ok(OutputStream {
                sio: device.sio.clone(),
                device: device.handle,
                stream: Mutex::new(outstream),
            }), OutputStream(OutputStreamImpl), SoundIo)
        }
    }
}

impl std::ops::Drop for Session {
    fn drop(&mut self) {
        unsafe {
            let Self(sio) = *self;
            soundio_destroy(sio);
        }
    }
}

impl OutputStream {
    pub fn play(
        &self,
        source: impl Source + Send + 'static
    ) -> Result<(), Error> {
        unsafe {
            let mut source = Box::new(source) as Box<dyn Source>;
            let mut guard = self.stream.lock().unwrap();
            let mut extra = Vec::with_capacity(32768);
            let mut exit = false;
            let mut param = UdonCallbackParam {
                source: &mut source as *mut Box<dyn Source> as _,
                extra: &mut extra,
                exit: &mut exit,
                err: Ok(()),
            };
            (**guard).userdata = &mut param as *mut _ as _;
            let err = soundio_outstream_start(*guard);
            if err != 0 {
                panic!("failed to start outstream: {}", std::ffi::CStr::from_ptr(soundio_strerror(err)).to_str().unwrap());
            }
            while !exit {
                soundio_wait_events(self.sio);
            }
            param.err
        }
    }
}

impl std::ops::Drop for OutputStream {
    fn drop(&mut self) {
        unsafe {
            let guard = self.stream.lock().unwrap();
            soundio_outstream_pause(*guard, 1);
            soundio_outstream_destroy(*guard);
            soundio_device_unref(self.device);
        }
    }
}

struct UdonCallbackParam {
    source: *mut Box<dyn Source>,
    extra: *mut Vec<f32>,
    exit: *mut bool,
    err: Result<(), Error>,
}

unsafe extern "C" fn udon_callback(
    outstream: *mut SoundIoOutStream,
    _frame_count_min: c_int,
    frame_count_max: c_int,
) {
    let param = (*outstream).userdata as *mut UdonCallbackParam;
    let format = (*outstream).format;
    let channel_count = (*outstream).layout.channel_count as usize;
    let mut areas: *mut SoundIoChannelArea = std::ptr::null_mut();
    let mut frames_left: c_int = frame_count_max;
    let mut err: c_int;
    while frames_left > 0 {
        let mut frame_count = frames_left;
        err = soundio_outstream_begin_write(outstream, &mut areas, &mut frame_count);
        if err != 0 {
            (*param).err = Err(Error::Unknown);
            *(*param).exit = true;
            return;
        }
        if frame_count == 0 {
            break;
        }
        let units = frame_count as usize * channel_count;
        let extra = &mut *(*param).extra;
        extra.clear();
        extra.reserve(units);
        extra.set_len(units);
        let total = (*(*param).source).write_samples(extra.as_mut_slice());
        extra.set_len(total);
        match format {
            SoundIoFormat::SoundIoFormatFloat32LE | SoundIoFormat::SoundIoFormatFloat32BE => {
                for ch in 0..channel_count {
                    let area = *areas.offset(ch as _);
                    let p = area.ptr;
                    if cfg!(target_endian = "little") == (matches!(format, SoundIoFormat::SoundIoFormatFloat32LE)) {
                        for (i, sample) in extra.iter().copied().skip(ch as _).step_by(channel_count).enumerate() {
                            *p.add(area.step as usize * i).cast::<f32>() = sample;
                        }
                    } else {
                        for (i, sample) in extra.iter().copied().skip(ch as _).step_by(channel_count).enumerate() {
                            *p.add(area.step as usize * i).cast::<u32>() = sample.to_bits().swap_bytes();
                        }
                    }
                    for i in (total / channel_count)..frame_count as usize {
                        *p.add(area.step as usize * i).cast::<u32>() = 0;
                    }
                }
            },
            _ => (),
        }
        err = soundio_outstream_end_write(outstream);
        if err != 0 {
            (*param).err = Err(Error::Unknown);
            *(*param).exit = true;
            return;
        }
        if total < units {
            *(*param).exit = true;
            return;
        }
        frames_left -= frame_count;
    }
}
