# AGENTS.md

If an `AGENTS.md` or `CLAUDE.md` exists higher in the tree, follow it too. If it
conflicts with this file, ask the creator.

## What this crate is

The audio front end for speech-to-text, shared by cox, runa and aulo: the default
microphone (`Recorder`), audio file decode (`decode_audio`), resampling to 16 kHz
mono, an energy VAD (`energy_vad`) and, behind the `whisper` cargo feature
(off by default), a whisper.cpp `Transcriber`. Audio stays in memory; nothing
here opens a socket or writes a file.

Audio callbacks never block or allocate, and audio buffers are bounded. The
`cpal` callback in `src/capture.rs` is the function `feed`: it downmixes into the
producer half of a fixed-size `ringbuf` ring that `open` allocates before the
stream starts, and counts the samples a full ring drops in an `AtomicU64`
(`Recorder::overflowed_samples`). Keep it that way: no `Mutex`, no `Vec`, no
logging and no `?` that allocates in the callback, and a test for any change.

Tests must not need a microphone or a model: device-free steps are pure
functions, and the model test is `#[ignore]`d.

## Commands

```bash
cargo test
cargo clippy --all-targets
cargo test --features whisper   # builds whisper.cpp: needs cmake and a C++ compiler
cargo fmt
```

On Linux, `cpal` needs the ALSA headers (`libasound2-dev`).

`just test` runs the same tests and finishes with a lossless `swarfr` cleanup of the target dir.
