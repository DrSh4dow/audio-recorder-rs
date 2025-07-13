# Rust System Audio & Mic audio Recorder (audio-recorder-rs)

<!--toc:start-->
- [Rust System Audio & Mic audio Recorder (audio-recorder-rs)](#rust-system-audio-mic-audio-recorder-audio-recorder-rs)
  - [Overview](#overview)
  - [Features](#features)
  - [Usage](#usage)
  - [API](#api)
    - [`Recorder`](#recorder)
<!--toc:end-->

Core library for cross-OS system + input audio recording.

## Overview

The `audio-recorder-rs` library provides functionality for recording audio using
various configurations. It supports recording from multiple devices, with or
without resampling, and is designed to be used in a singleton pattern to
ensure only one instance of the recorder is active at any time.

## Features

- Cross-platform support (Windows, macOS, Linux)
- Auto Resampling
- Background thread for non-blocking recording

## Usage

To use the recorder, create an instance of the `Recorder` struct and
call its `start` method to begin recording. This will start a background
thread that will record audio from the default input device. The `start`
function will return a receiver which acts as a stream to receive the
audio data in `f32`. Call the `stop` method to stop recording.

```rust
use audio_recorder_rs::Recorder;

fn main() {
    let mut recorder = Recorder::new();
    let receiver = recorder.start().expect("Failed to start recording");

    thread::spawn(move || {
      while let Ok(d) = receiver.recv() {
        for sample in d {
          writer.write_sample(sample).ok();
        }
      }
    });

    sleep(Duration::from_secs(5));
    recorder.stop().expect("Failed to stop recording");
}
```

## API

### `Recorder`

- `new() -> Recorder`

  - Creates a new instance of the `Recorder`.

- `start() -> Result<Receiver<TargetFormat>, RecorderError>`

  - Starts the recording process and returns a receiver for the audio data stream.

- `stop() -> Result<(), RecorderError>`

  - Stops the recording process.

- `get_is_recording() -> bool`

  - Returns whether the recorder is currently recording.

- `get_config() -> RecorderConfig`
  - Returns the current configuration of the audio output stream.
