//! # Recorder Core Library
//!
//! The `recorder-core` library provides functionality for recording audio using various configurations.
//! It supports recording from multiple devices, with or without resampling, and is designed to be used
//! in a singleton pattern to ensure only one instance of the recorder is active at any time.
//!
//! ## Usage
//!
//! To use the recorder, create an instance of the `Recorder` struct and call its `start` method to begin recording.
//! This will start a background thread that will record audio from the default input device. the
//! start function will return a receiver which acts as a stream to receive the audio data in
//! `TargetFormat`.
//! Call the `stop` method to stop recording.

//mod recorder;

//pub use recorder::Recorder;
mod recorder_v2;
