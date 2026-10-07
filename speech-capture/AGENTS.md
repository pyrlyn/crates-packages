# AGENTS.md

If an `AGENTS.md` or `CLAUDE.md` exists higher in the tree, follow it too. If it
conflicts with this file, ask the creator.

## What this crate is

The audio front end for speech-to-text, shared by cox, runa and aulo: the default
microphone (`Recorder`), audio file decode (`decode_audio`), resampling to 16 kHz
mono, an energy VAD (`energy_vad`) and, behind the `whisper` cargo feature
(off by default), a whisper.cpp `Transcriber`. Audio stays in memory; nothing
here opens a socket or writes a file.

Audio callbacks must never block or allocate, and audio buffers are bounded. The
`cpal` callback in `src/capture.rs` still locks a `Mutex` and allocates a `Vec`
per callback (carried over from cox as is); the cap bounds the buffer, but
the callback rule is not met yet. A change there should move to a fixed-size
ring buffer with a counted overflow, not make the callback do more.

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
