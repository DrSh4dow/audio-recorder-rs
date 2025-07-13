use std::{
    thread::{self},
    time::Duration,
};

use cpal::{
    Sample,
    traits::{DeviceTrait, StreamTrait},
};
use crossbeam_channel::Receiver;

use super::{
    constants::{CLOCK_DELAY, TargetFormat},
    errors::AudioRecorderError,
};

use super::Recorder;

/// Macro: build an input stream for every numeric CPAL `SampleFormat` you list.
///
/// * `$device`   – the `cpal::Device`.
/// * `$config`   – the (mutable/owned) `cpal::StreamConfig`.
/// * `$fmt`      – the **runtime** sample-format you want to match on.
/// * `$tx`       – an `mpsc::Sender<Vec<TargetFormat>>`.
///
/// After those four, give the *compile-time* mapping from enum variant → Rust
/// primitive type (`I16 => i16`, etc.).  
/// It expands to an **expression** that evaluates to `Result<cpal::Stream, String>`.
///
macro_rules! build_input_stream_for {
    (
        $device:expr,            // input  CPAL device
        $config:expr,            // config
        $fmt:expr,               // runtime SampleFormat
        $tx:expr,                // mpsc::Sender<Vec<TargetFormat>>
        $( $variant:ident => $ty:ty ),+ $(,)?   // mapping table
    ) => {{
        match $fmt {
            $(
                cpal::SampleFormat::$variant => {
                    // Each branch has the right slice type automatically.
                    let tx_clone = $tx.clone();
                    $device.build_input_stream(
                        &($config).clone().into(),
                        move |data: &[$ty], _| {
                            // fast, idiomatic conversion
                            let parsed: Vec<TargetFormat> =
                                data.iter().map(|s| s.to_sample::<TargetFormat>()).collect();
                            if let Err(e) = tx_clone.send(parsed) {
                                tracing::error!("Failed to send data: {}", e);
                            }
                        },
                        Recorder::err_fn,
                        None,
                    )
                    .map_err(|e| {
                        tracing::error!("Failed to build input stream: {}", e);
                        "Failed to build input stream".to_string()
                    })
                }
            )+
            other => {
                tracing::error!("Unsupported sample format: {:?}", other);
                Err("Unsupported sample format".into())
            }
        }
    }};
}

impl Recorder {
    pub fn record_single_device(
        &mut self,
        device: cpal::Device,
    ) -> Result<Receiver<Vec<TargetFormat>>, AudioRecorderError> {
        tracing::info!("Record single device started");

        tracing::debug!(
            "Using input device: {:?}",
            device.name().unwrap_or(String::from("Unknown"))
        );

        let config = match device.default_input_config() {
            Ok(config) => config,
            Err(error) => {
                tracing::error!("Failed to get default input config: {}", error);
                return Err(AudioRecorderError::DeviceError(
                    "Failed to get default input config",
                ));
            }
        };

        tracing::debug!("Setting up the recorder");
        self.target_sample_rate = Some(config.sample_rate().0);
        self.channels = Some(config.channels());
        self.sample_size = Some(config.sample_format().sample_size() as u32);
        tracing::debug!("Config: {:?}", self);

        // Run the input stream on a separate thread.
        tracing::debug!("Clone recording signal mutex");
        let recording_signal = self.recording_signal.clone();

        // A signal to pass on the stream
        tracing::debug!("Create channel for passing data");
        let (sync_tx, sync_rx) = crossbeam_channel::unbounded::<Vec<TargetFormat>>();

        tracing::debug!("Begin recording...");
        thread::spawn(move || {
            let stream = match build_input_stream_for!(
                device,
                config,
                config.sample_format(),
                sync_tx,
                I8  => i8,
                I16 => i16,
                I32 => i32,
                I64 => i64,
                U8  => u8,
                U16 => u16,
                U32 => u32,
                U64 => u64,
                F32 => f32,
                F64 => f64
            ) {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!("Failed to build input stream: {}", e);
                    recording_signal.store(false, std::sync::atomic::Ordering::SeqCst);
                    return Err("Failed to build input stream".to_string());
                }
            };

            tracing::info!("Stream started");
            if let Err(e) = stream.play() {
                tracing::error!("Failed to play stream: {}", e);
                recording_signal.store(false, std::sync::atomic::Ordering::SeqCst);
                return Err("Failed to play stream".to_string());
            };

            while recording_signal.load(std::sync::atomic::Ordering::SeqCst) {
                thread::sleep(Duration::from_millis(CLOCK_DELAY as _));
            }

            tracing::debug!("Dropping stream");
            drop(stream);

            tracing::info!("Recording stopped");
            Ok(())
        });

        Ok(sync_rx)
    }
}
