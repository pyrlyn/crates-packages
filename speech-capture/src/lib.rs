// Copyright (c) 2026 Ivan Tugay
// SPDX-License-Identifier: GPL-3.0-or-later OR LicenseRef-Royalty-Free
// Licensed under GPL-3.0 or later, or under the royalty-free licence in LICENSE-ROYALTY-FREE.md

//! Everything between a sound source and a speech-to-text engine: the default
//! microphone (`Recorder`), audio files (`decode_audio`), resampling to 16 kHz
//! mono (`resample_to_16k`, `resample_mono`) and an energy voice-activity
//! detector (`energy_vad`). The optional `whisper` feature adds `Transcriber`,
//! whisper.cpp over a 16 kHz mono buffer. Audio stays in memory: nothing here
//! opens a socket or writes a file.

// Unit tests assert with `expect`/`unwrap`; the deny list is for shipped code.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

mod capture;
mod decode;
mod error;
mod resample;
#[cfg(feature = "whisper")]
mod transcribe;
mod vad;

pub use capture::{Recorder, input_device};
pub use decode::{AudioProbe, DecodedAudio, decode_audio, pcm_sha256, pcm_to_wav_bytes};
pub use error::Error;
pub use resample::{TARGET_SAMPLE_RATE, resample_mono, resample_to_16k};
#[cfg(feature = "whisper")]
pub use transcribe::{TranscribeError, Transcriber};
pub use vad::{VadSpan, energy_vad};
