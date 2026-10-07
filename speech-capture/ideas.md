# Ideas

- Make the `cpal` callback real-time safe: copy into a fixed-size ring buffer (an SPSC queue) and count overflow, with no `Mutex` and no `Vec` per callback. aulo's audio rules require it.
- One resampler instead of two: `resample_to_16k` (FFT) and `resample_mono` (sinc) differ only in quality and speed, but runa's stored PCM hashes depend on the sinc output.
- A streaming `Recorder` that hands out 16 kHz frames while recording, for live dictation.
