use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use constants::TargetFormat;
use crossbeam_channel::Receiver;
use errors::AudioRecorderError;
use get_default_device::get_default_input_device;

/// Module for handling constants used in the audio recorder.
mod constants;

/// Module for error handling in the audio recorder.
mod errors;
/// Module for handling the default device i/o selection.
mod get_default_device;

/// Helper functions for the recorder module.
mod helpers;

/// Module for handling recording with a resampler.
mod multiple_w_resampler;

/// Module for handling recording without a resampler.
mod multiple_wo_resampler;

/// Module for spawning multiple recording threads.
mod record_multiple_spawner;

/// Module for recording from a single device.
mod record_single_device;

/// A recorder for recording audio - It should be consumed using a singleton pattern
#[derive(Debug)]
pub struct Recorder {
    /// recording signal, safe to share across threads
    recording_signal: Arc<AtomicBool>,
    /// The target sample rate for recording.
    target_sample_rate: Option<u32>,
    /// The number of channels for recording.
    channels: Option<u16>,
    /// The sample size for recording.
    sample_size: Option<u32>,
}

impl Recorder {
    /// Creates a new instance of the Recorder.
    pub fn new() -> Self {
        Recorder {
            recording_signal: Arc::new(AtomicBool::new(false)),
            target_sample_rate: None,
            channels: None,
            sample_size: None,
        }
    }

    #[tracing::instrument]
    pub fn stop(&mut self) {
        tracing::info!("Stopping the recorder");

        tracing::debug!("Checking if recording is in progress");
        if !self.recording_signal.load(Ordering::SeqCst) {
            tracing::info!("Recording is not in progress");
            return;
        }

        tracing::debug!("Resetting recording signal");
        self.recording_signal.store(false, Ordering::SeqCst);
        tracing::info!("Recorder stopped successfully");
    }

    #[tracing::instrument]
    pub fn start(
        &mut self,
        input_only: bool,
    ) -> Result<Receiver<Vec<TargetFormat>>, AudioRecorderError> {
        tracing::info!("Starting audio recording");

        tracing::debug!("Checking if recording is already in progress");
        if self.recording_signal.load(Ordering::SeqCst) {
            tracing::warn!("Recording is already in progress");
            return Err(AudioRecorderError::RecordingInProgress);
        }

        tracing::debug!("Initializing flag for recording");
        self.recording_signal.store(true, Ordering::SeqCst);

        let input_device = match get_default_input_device() {
            Ok(device) => device,
            Err(e) => {
                tracing::error!("{}", e);
                self.stop();
                return Err(e);
            }
        };

        if input_only {
            tracing::info!("Recording from a single device");
            return self.record_single_device(input_device);
        }

        todo!()
    }
}

impl Default for Recorder {
    fn default() -> Self {
        Self::new()
    }
}
