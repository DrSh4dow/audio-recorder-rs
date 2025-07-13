use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use constants::TargetFormat;
use cpal::traits::DeviceTrait;
use crossbeam_channel::Receiver;
use errors::AudioRecorderError;
use get_default_device::{get_default_input_device, get_default_output_device};

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

/// Expands to the correct `self.record_multiple::<In, Out>(…)` call
/// for every (input, output) sample-format pair.
///
/// Usage:
/// `record_multiple_expansion!(self, input_config, output_config, input_device, output_device)`
macro_rules! record_multiple_expansion {
    // ── entry point ───────────────────────────────────────────────────────────
    ($self_:expr, $in_cfg:expr, $out_cfg:expr, $in_dev:expr, $out_dev:expr) => {{
        // helper: second-level match (output side) for a **fixed** input type
        macro_rules! match_output {
            ($in_ty:ty) => {{
                match $out_cfg.sample_format() {
                    cpal::SampleFormat::I8 => {
                        $self_.record_multiple::<$in_ty, i8>($in_dev, $out_dev)
                    }
                    cpal::SampleFormat::I16 => {
                        $self_.record_multiple::<$in_ty, i16>($in_dev, $out_dev)
                    }
                    cpal::SampleFormat::I32 => {
                        $self_.record_multiple::<$in_ty, i32>($in_dev, $out_dev)
                    }
                    cpal::SampleFormat::I64 => {
                        $self_.record_multiple::<$in_ty, i64>($in_dev, $out_dev)
                    }
                    cpal::SampleFormat::U8 => {
                        $self_.record_multiple::<$in_ty, u8>($in_dev, $out_dev)
                    }
                    cpal::SampleFormat::U16 => {
                        $self_.record_multiple::<$in_ty, u16>($in_dev, $out_dev)
                    }
                    cpal::SampleFormat::U32 => {
                        $self_.record_multiple::<$in_ty, u32>($in_dev, $out_dev)
                    }
                    cpal::SampleFormat::U64 => {
                        $self_.record_multiple::<$in_ty, u64>($in_dev, $out_dev)
                    }
                    cpal::SampleFormat::F32 => {
                        $self_.record_multiple::<$in_ty, f32>($in_dev, $out_dev)
                    }
                    cpal::SampleFormat::F64 => {
                        $self_.record_multiple::<$in_ty, f64>($in_dev, $out_dev)
                    }
                    sf => Err(AudioRecorderError::SignalError(format!(
                        "Unsupported sample format '{sf:?}'"
                    ))),
                }
            }};
        }

        // first-level match (input side)
        match $in_cfg.sample_format() {
            cpal::SampleFormat::I8 => match_output!(i8),
            cpal::SampleFormat::I16 => match_output!(i16),
            cpal::SampleFormat::I32 => match_output!(i32),
            cpal::SampleFormat::I64 => match_output!(i64),
            cpal::SampleFormat::U8 => match_output!(u8),
            cpal::SampleFormat::U16 => match_output!(u16),
            cpal::SampleFormat::U32 => match_output!(u32),
            cpal::SampleFormat::U64 => match_output!(u64),
            cpal::SampleFormat::F32 => match_output!(f32),
            cpal::SampleFormat::F64 => match_output!(f64),
            sf => Err(AudioRecorderError::SignalError(format!(
                "Unsupported sample format '{sf:?}'"
            ))),
        }
    }};
}

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
        self.target_sample_rate = None;
        self.channels = None;
        self.sample_size = None;

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

        let output_device = match get_default_output_device() {
            Ok(device) => device,
            Err(e) => {
                tracing::error!("{}", e);
                self.stop();
                return Err(e);
            }
        };
        tracing::info!("Recording from multiple devices");

        tracing::debug!(
            "Using input device: {:?}",
            input_device.name().unwrap_or(String::from("Unknown"))
        );
        tracing::debug!(
            "Using output device: {:?}",
            output_device.name().unwrap_or(String::from("Unknown"))
        );

        let input_config = match input_device.default_input_config() {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Failed to get input config: {}", e);
                return Err(AudioRecorderError::DeviceError(
                    "Failed to get input config",
                ));
            }
        };

        let output_config = match output_device.default_output_config() {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Failed to get output config: {}", e);
                return Err(AudioRecorderError::DeviceError(
                    "Failed to get output config",
                ));
            }
        };

        record_multiple_expansion!(
            self,
            input_config,
            output_config,
            input_device,
            output_device
        )
    }
}

impl Default for Recorder {
    fn default() -> Self {
        Self::new()
    }
}
