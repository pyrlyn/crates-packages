// Copyright (c) 2026 Ivan Tugay
// SPDX-License-Identifier: GPL-3.0-or-later OR LicenseRef-Royalty-Free
// Licensed under GPL-3.0 or later, or under the royalty-free licence in LICENSE-ROYALTY-FREE.md

//! `Recorder`: the default microphone as mono `f32` at 16 kHz, held in
//! memory only and capped in length. The device-free steps (sample
//! conversion, downmix, the cap, resampling) are pure functions so they test
//! without a microphone; only `open` touches `cpal`.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError, mpsc};
use std::thread::JoinHandle;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SampleFormat, SizedSample};

use crate::Error;
use crate::resample::resample_to_16k;

/// A press of the key in progress. The stream lives on its own thread, so
/// the recorder is `Send` whatever the platform's stream type is; dropping
/// the recorder (cancel) stops the stream and discards the audio.
pub struct Recorder {
    buffer: Arc<Mutex<Capped>>,
    rate: u32,
    stop: Option<mpsc::Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

impl Recorder {
    /// Opens the default input device and keeps at most `max` of audio.
    pub fn start(max: Duration) -> Result<Self, Error> {
        let buffer = Arc::new(Mutex::new(Capped::new(0)));
        let shared = Arc::clone(&buffer);
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let (stop_tx, stop_rx) = mpsc::channel::<()>();
        let thread = std::thread::Builder::new()
            .name("speech-capture".into())
            .spawn(move || match open(shared, max) {
                Ok((stream, rate)) => {
                    let _ = ready_tx.send(Ok(rate));
                    // Returns once the sender is dropped: stop or cancel.
                    let _ = stop_rx.recv();
                    drop(stream);
                }
                Err(e) => {
                    let _ = ready_tx.send(Err(e));
                }
            })
            .map_err(|e| Error::Stream(e.to_string()))?;
        let mut recorder = Self {
            buffer,
            rate: 0,
            stop: Some(stop_tx),
            thread: Some(thread),
        };
        recorder.rate = ready_rx
            .recv()
            .map_err(|_| Error::Stream("the capture thread ended".into()))??;
        Ok(recorder)
    }

    /// Stops the stream and returns the audio resampled to 16 kHz.
    pub fn stop(mut self) -> Result<Vec<f32>, Error> {
        self.halt();
        let mono = std::mem::take(&mut lock(&self.buffer).samples);
        resample_to_16k(&mono, self.rate)
    }

    fn halt(&mut self) {
        drop(self.stop.take());
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for Recorder {
    fn drop(&mut self) {
        self.halt();
    }
}

/// The default input device's name, `None` without one; opens no stream,
/// so it records nothing and raises no permission prompt (a doctor/health check).
pub fn input_device() -> Option<String> {
    let device = cpal::default_host().default_input_device()?;
    device.description().ok().map(|d| d.name().to_string())
}

fn lock(buffer: &Mutex<Capped>) -> MutexGuard<'_, Capped> {
    buffer.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Mono samples at the device rate, never more than `cap` of them.
struct Capped {
    samples: Vec<f32>,
    cap: usize,
}

impl Capped {
    fn new(cap: usize) -> Self {
        Self {
            samples: Vec::new(),
            cap,
        }
    }

    fn push(&mut self, mono: impl IntoIterator<Item = f32>) {
        let room = self.cap.saturating_sub(self.samples.len());
        self.samples.extend(mono.into_iter().take(room));
    }
}

fn stream_error(e: cpal::Error) -> Error {
    Error::Stream(e.to_string())
}

fn open(buffer: Arc<Mutex<Capped>>, max: Duration) -> Result<(cpal::Stream, u32), Error> {
    let device = cpal::default_host()
        .default_input_device()
        .ok_or(Error::NoInputDevice)?;
    let supported = device.default_input_config().map_err(stream_error)?;
    let rate = supported.sample_rate();
    let channels = usize::from(supported.channels()).max(1);
    lock(&buffer).cap = (f64::from(rate) * max.as_secs_f64()) as usize;
    let format = supported.sample_format();
    let config: cpal::StreamConfig = supported.into();
    let stream = match format {
        SampleFormat::I8 => build::<i8>(&device, config, channels, buffer),
        SampleFormat::I16 => build::<i16>(&device, config, channels, buffer),
        SampleFormat::I32 => build::<i32>(&device, config, channels, buffer),
        SampleFormat::I64 => build::<i64>(&device, config, channels, buffer),
        SampleFormat::U8 => build::<u8>(&device, config, channels, buffer),
        SampleFormat::U16 => build::<u16>(&device, config, channels, buffer),
        SampleFormat::U32 => build::<u32>(&device, config, channels, buffer),
        SampleFormat::U64 => build::<u64>(&device, config, channels, buffer),
        SampleFormat::F32 => build::<f32>(&device, config, channels, buffer),
        SampleFormat::F64 => build::<f64>(&device, config, channels, buffer),
        other => Err(Error::Stream(format!("unsupported sample format {other}"))),
    }?;
    stream.play().map_err(stream_error)?;
    Ok((stream, rate))
}

fn build<T>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    channels: usize,
    buffer: Arc<Mutex<Capped>>,
) -> Result<cpal::Stream, Error>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    device
        .build_input_stream(
            config,
            move |data: &[T], _: &cpal::InputCallbackInfo| {
                let samples: Vec<f32> = data.iter().map(|s| to_f32(*s)).collect();
                lock(&buffer).push(downmix(&samples, channels));
            },
            |e| tracing::warn!(error = %e, "microphone stream error"),
            None,
        )
        .map_err(stream_error)
}

/// Any device sample as `f32` in `-1.0..=1.0`.
fn to_f32<T: Sample>(sample: T) -> f32
where
    f32: FromSample<T>,
{
    sample.to_sample::<f32>()
}

/// Interleaved frames to mono by averaging each frame's channels.
fn downmix(interleaved: &[f32], channels: usize) -> impl Iterator<Item = f32> + '_ {
    let channels = channels.max(1);
    interleaved
        .chunks(channels)
        .map(move |frame| frame.iter().sum::<f32>() / channels as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downmix_averages_the_channels() {
        let stereo = [1.0, 0.0, 0.5, 0.5, -1.0, 1.0];
        assert_eq!(downmix(&stereo, 2).collect::<Vec<_>>(), [0.5, 0.5, 0.0]);
        let mono = [0.25, -0.25];
        assert_eq!(downmix(&mono, 1).collect::<Vec<_>>(), mono);
    }

    #[test]
    fn buffer_stops_growing_at_the_cap() {
        let mut buffer = Capped::new(5);
        buffer.push([0.1; 3]);
        buffer.push([0.2; 3]);
        assert_eq!(buffer.samples, [0.1, 0.1, 0.1, 0.2, 0.2]);
        buffer.push([0.3; 10]);
        assert_eq!(buffer.samples.len(), 5);
    }

    #[test]
    fn i16_and_u16_samples_convert_to_f32() {
        assert_eq!(to_f32(0i16), 0.0);
        assert_eq!(to_f32(i16::MIN), -1.0);
        assert!((to_f32(i16::MAX) - 1.0).abs() < 1e-4);
        assert_eq!(to_f32(32_768u16), 0.0);
        assert_eq!(to_f32(0u16), -1.0);
        assert!((to_f32(u16::MAX) - 1.0).abs() < 1e-4);
    }
}
