// Copyright (c) 2026 Ivan Tugay
// SPDX-License-Identifier: GPL-3.0-or-later OR LicenseRef-Royalty-Free
// Licensed under GPL-3.0 or later, or under the royalty-free licence in LICENSE-ROYALTY-FREE.md

//! `Recorder`: the default microphone as mono `f32` at 16 kHz, held in
//! memory only and capped in length. The `cpal` callback never blocks or
//! allocates: it downmixes straight into the producer half of a fixed-size
//! ring buffer allocated before the stream starts, and counts the samples the
//! full buffer had no room for. The device-free steps (`feed`, sample
//! conversion, downmix, resampling) are plain functions so they test without
//! a microphone; only `open` touches `cpal`.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, mpsc};
use std::thread::JoinHandle;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SampleFormat, SizedSample};
use ringbuf::traits::{Consumer, Producer, Split};
use ringbuf::{HeapCons, HeapRb};

use crate::Error;
use crate::resample::resample_to_16k;

/// What the capture thread hands back once the stream plays.
struct Opened {
    rate: u32,
    samples: HeapCons<f32>,
}

/// A press of the key in progress. The stream lives on its own thread, so
/// the recorder is `Send` whatever the platform's stream type is; dropping
/// the recorder (cancel) stops the stream and discards the audio.
pub struct Recorder {
    samples: HeapCons<f32>,
    overflow: Arc<AtomicU64>,
    rate: u32,
    stop: Option<mpsc::Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

impl Recorder {
    /// Opens the default input device and keeps at most `max` of audio.
    pub fn start(max: Duration) -> Result<Self, Error> {
        let overflow = Arc::new(AtomicU64::new(0));
        let dropped = Arc::clone(&overflow);
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let (stop_tx, stop_rx) = mpsc::channel::<()>();
        let thread = std::thread::Builder::new()
            .name("speech-capture".into())
            .spawn(move || match open(dropped, max) {
                Ok((stream, opened)) => {
                    let _ = ready_tx.send(Ok(opened));
                    // Returns once the sender is dropped: stop or cancel.
                    let _ = stop_rx.recv();
                    drop(stream);
                }
                Err(e) => {
                    let _ = ready_tx.send(Err(e));
                }
            })
            .map_err(|e| Error::Stream(e.to_string()))?;
        let opened = ready_rx
            .recv()
            .map_err(|_| Error::Stream("the capture thread ended".into()))??;
        Ok(Self {
            samples: opened.samples,
            overflow,
            rate: opened.rate,
            stop: Some(stop_tx),
            thread: Some(thread),
        })
    }

    /// Samples at the device rate dropped so far because the recording was
    /// already `max` long.
    pub fn overflowed_samples(&self) -> u64 {
        self.overflow.load(Ordering::Relaxed)
    }

    /// Stops the stream and returns the audio resampled to 16 kHz.
    pub fn stop(mut self) -> Result<Vec<f32>, Error> {
        self.halt();
        let dropped = self.overflowed_samples();
        if dropped > 0 {
            tracing::warn!(
                dropped,
                "recording hit its length cap; the tail was dropped"
            );
        }
        let mono: Vec<f32> = self.samples.pop_iter().collect();
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

fn stream_error(e: cpal::Error) -> Error {
    Error::Stream(e.to_string())
}

fn open(overflow: Arc<AtomicU64>, max: Duration) -> Result<(cpal::Stream, Opened), Error> {
    let device = cpal::default_host()
        .default_input_device()
        .ok_or(Error::NoInputDevice)?;
    let supported = device.default_input_config().map_err(stream_error)?;
    let rate = supported.sample_rate();
    let channels = usize::from(supported.channels()).max(1);
    // Sized once, here, so the callback never allocates. `HeapRb` rejects a
    // zero capacity.
    let cap = ((f64::from(rate) * max.as_secs_f64()) as usize).max(1);
    let (producer, samples) = HeapRb::<f32>::new(cap).split();
    let format = supported.sample_format();
    let config: cpal::StreamConfig = supported.into();
    let stream = match format {
        SampleFormat::I8 => build::<i8>(&device, config, channels, producer, overflow),
        SampleFormat::I16 => build::<i16>(&device, config, channels, producer, overflow),
        SampleFormat::I32 => build::<i32>(&device, config, channels, producer, overflow),
        SampleFormat::I64 => build::<i64>(&device, config, channels, producer, overflow),
        SampleFormat::U8 => build::<u8>(&device, config, channels, producer, overflow),
        SampleFormat::U16 => build::<u16>(&device, config, channels, producer, overflow),
        SampleFormat::U32 => build::<u32>(&device, config, channels, producer, overflow),
        SampleFormat::U64 => build::<u64>(&device, config, channels, producer, overflow),
        SampleFormat::F32 => build::<f32>(&device, config, channels, producer, overflow),
        SampleFormat::F64 => build::<f64>(&device, config, channels, producer, overflow),
        other => Err(Error::Stream(format!("unsupported sample format {other}"))),
    }?;
    stream.play().map_err(stream_error)?;
    Ok((stream, Opened { rate, samples }))
}

fn build<T>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    channels: usize,
    mut producer: impl Producer<Item = f32> + Send + 'static,
    overflow: Arc<AtomicU64>,
) -> Result<cpal::Stream, Error>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    device
        .build_input_stream(
            config,
            move |data: &[T], _: &cpal::InputCallbackInfo| {
                feed(data, channels, &mut producer, &overflow);
            },
            |e| tracing::warn!(error = %e, "microphone stream error"),
            None,
        )
        .map_err(stream_error)
}

/// The body of the input callback: converts `data` to `f32`, averages each
/// interleaved frame to mono and pushes it. A sample the ring has no room for
/// is counted in `overflow` and dropped, so the recording keeps its first
/// samples, as a cap does. Wait-free: no lock, no allocation, no syscall.
fn feed<T>(
    data: &[T],
    channels: usize,
    producer: &mut impl Producer<Item = f32>,
    overflow: &AtomicU64,
) where
    T: Sample,
    f32: FromSample<T>,
{
    let channels = channels.max(1);
    for frame in data.chunks(channels) {
        let sum: f32 = frame.iter().map(|s| to_f32(*s)).sum();
        if producer.try_push(sum / channels as f32).is_err() {
            overflow.fetch_add(1, Ordering::Relaxed);
        }
    }
}

/// Any device sample as `f32` in `-1.0..=1.0`.
fn to_f32<T: Sample>(sample: T) -> f32
where
    f32: FromSample<T>,
{
    sample.to_sample::<f32>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ringbuf::HeapProd;

    /// A ring of `cap` samples and its two halves, as `open` builds it.
    fn ring(cap: usize) -> (HeapProd<f32>, HeapCons<f32>) {
        HeapRb::<f32>::new(cap).split()
    }

    fn drain(consumer: &mut HeapCons<f32>) -> Vec<f32> {
        consumer.pop_iter().collect()
    }

    #[test]
    fn feed_averages_the_channels() {
        let overflow = AtomicU64::new(0);
        let (mut tx, mut rx) = ring(8);
        feed(&[1.0f32, 0.0, 0.5, 0.5, -1.0, 1.0], 2, &mut tx, &overflow);
        assert_eq!(drain(&mut rx), [0.5, 0.5, 0.0]);
        feed(&[0.25f32, -0.25], 1, &mut tx, &overflow);
        assert_eq!(drain(&mut rx), [0.25, -0.25]);
        assert_eq!(overflow.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn feed_converts_integer_samples_and_a_drained_recording_equals_what_was_pushed() {
        let overflow = AtomicU64::new(0);
        let (mut tx, mut rx) = ring(16);
        let first = [0i16, i16::MIN, 0, i16::MIN];
        feed(&first, 2, &mut tx, &overflow);
        feed(&[0.5f32; 6], 3, &mut tx, &overflow);
        assert_eq!(drain(&mut rx), [-0.5, -0.5, 0.5, 0.5]);
        assert_eq!(overflow.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn a_full_ring_counts_dropped_samples_keeps_the_first_ones_and_never_panics() {
        let overflow = AtomicU64::new(0);
        let (mut tx, mut rx) = ring(5);
        feed(&[0.1f32; 3], 1, &mut tx, &overflow);
        feed(&[0.2f32; 3], 1, &mut tx, &overflow);
        assert_eq!(overflow.load(Ordering::Relaxed), 1);
        feed(&[0.3f32; 10], 1, &mut tx, &overflow);
        assert_eq!(overflow.load(Ordering::Relaxed), 11);
        assert_eq!(drain(&mut rx), [0.1, 0.1, 0.1, 0.2, 0.2]);
        // Draining frees the room again.
        feed(&[0.4f32; 2], 1, &mut tx, &overflow);
        assert_eq!(drain(&mut rx), [0.4, 0.4]);
        assert_eq!(overflow.load(Ordering::Relaxed), 11);
    }

    #[test]
    fn a_partial_last_frame_is_still_one_sample_and_zero_channels_count_as_one() {
        let overflow = AtomicU64::new(0);
        let (mut tx, mut rx) = ring(4);
        feed(&[1.0f32, 1.0, 1.0], 2, &mut tx, &overflow);
        assert_eq!(drain(&mut rx), [1.0, 0.5]);
        feed(&[0.5f32], 0, &mut tx, &overflow);
        assert_eq!(drain(&mut rx), [0.5]);
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
