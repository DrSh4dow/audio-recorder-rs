/// Common utilities and constants used across the recorder module.
mod common;

/// Helper functions for the recorder module.
mod helpers;

/// Module for handling recording with a resampler.
mod multiple_w_resampler;

/// Module for handling recording without a resampler.
mod multiple_wo_resampler;

/// Module for spawning multiple recording threads.
mod record_multiple_spawner;

#[cfg(target_os = "macos")]
mod macos_utils;

/// Module for recording from a single device.
mod record_single_device;

use std::sync::{Arc, Mutex};

use common::TargetFormat;
use cpal::traits::{DeviceTrait, HostTrait};
use crossbeam_channel::Receiver;

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
                    sf => Err(format!("Unsupported sample format '{sf:?}'")),
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
            sf => Err(format!("Unsupported sample format '{sf:?}'")),
        }
    }};
}

/// A recorder for recording audio - It should be consumed using a singleton pattern
#[derive(Debug)]
pub struct Recorder {
    /// A mutex for the recording signal. This is used to stop the recording threads.
    recording_signal_mutex: Arc<Mutex<bool>>,
    /// The target sample rate for recording.
    target_rate: Option<u32>,
    /// The number of channels for recording.
    channels: Option<u16>,
    /// The sample size for recording.
    sample_size: Option<u32>,
    /// Original output device
    #[cfg(target_os = "macos")]
    original_output_device_name: Option<String>,
}

impl Recorder {
    /// Creates a new instance of the Recorder.
    pub fn new() -> Self {
        Recorder {
            recording_signal_mutex: Arc::new(Mutex::new(false)),
            target_rate: None,
            channels: None,
            sample_size: None,
            #[cfg(target_os = "macos")]
            original_output_device_name: None,
        }
    }

    /// Stops the recorder by setting the recording signal to false.
    ///
    /// # Errors
    /// Returns an error if the recording signal mutex cannot be locked.
    #[tracing::instrument]
    pub fn stop(&mut self) -> Result<(), String> {
        tracing::info!("Stopping the recorder");

        let mut recording_signal = match self.recording_signal_mutex.lock() {
            Ok(recording_signal) => recording_signal,
            Err(error) => {
                tracing::error!("Failed to lock the recording signal mutex: {}", error);
                return Err("Failed to lock the recording signal mutex".to_string());
            }
        };
        *recording_signal = false;
        self.target_rate = None;

        #[cfg(target_os = "macos")]
        if let Some(orig_device_name) = &self.original_output_device_name {
            tracing::info!("Switching back to the original output device");
            if let Err(e) = macos_utils::switch_device(orig_device_name) {
                tracing::error!("Error switching back to the original output device: {}", e);
            };
        }

        Ok(())
    }

    /// Starts the recorder. If `input_only` is true, it records from a single device.
    /// Otherwise, it records from both input and output devices.
    ///
    /// # Errors
    /// Returns an error if the recorder cannot be prepared or if no input/output device is available.
    #[tracing::instrument]
    pub fn start(&mut self, input_only: bool) -> Result<Receiver<Vec<TargetFormat>>, String> {
        tracing::info!("Starting the recorder");

        match self.prepare() {
            Ok(_) => {
                tracing::debug!("Recorder prepared");
            }
            Err(_) => {
                tracing::error!("Failed to prepare recorder");
                return Err("Failed to prepare recorder".to_string());
            }
        };

        // decide the recording mode
        let host = cpal::default_host();
        let input_device = match host.default_input_device() {
            Some(input_device) => input_device,
            None => {
                tracing::error!("No input device available");
                return Err("No input device available".to_string());
            }
        };

        if input_only {
            tracing::info!("Recording from a single device");
            return self.record_single_device(input_device);
        }

        let output_device = match host.id() {
            #[cfg(target_os = "macos")]
            cpal::HostId::CoreAudio => match macos_utils::find_macos_monitor_source(&host) {
                Ok(device) => device,
                Err(e) => {
                    tracing::error!("Error finding monitor source: {:?}", e);
                    return Err(String::from("Error finding monitor source"));
                }
            },
            _ => match host.default_output_device() {
                Some(device) => device,
                None => {
                    tracing::error!("No output device found");
                    return Err(String::from("No output device found"));
                }
            },
        };

        tracing::debug!(
            "Using input device: {:?}",
            input_device.name().unwrap_or(String::from("Unknown"))
        );
        tracing::debug!(
            "Using output device: {:?}",
            output_device.name().unwrap_or(String::from("Unknown"))
        );

        tracing::info!("Recording from multiple devices");

        let input_config = match input_device.default_input_config() {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Failed to get input config: {}", e);
                return Err("Failed to get input config".to_string());
            }
        };

        let output_config = match output_device.default_output_config() {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Failed to get output config: {}", e);
                return Err("Failed to get output config".to_string());
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
