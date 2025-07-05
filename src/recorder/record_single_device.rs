use std::{
    thread::{self},
    time::Duration,
};

use cpal::{
    traits::{DeviceTrait, StreamTrait},
    Sample,
};
use crossbeam_channel::Receiver;

use crate::recorder::common::{TargetFormat, CLOCK_DELAY};

use super::Recorder;

impl Recorder {
    pub fn record_single_device(
        &mut self,
        device: cpal::Device,
    ) -> Result<Receiver<Vec<TargetFormat>>, String> {
        tracing::info!("Record single device started");

        tracing::debug!(
            "Using input device: {:?}",
            device.name().unwrap_or(String::from("Unknown"))
        );

        let config = match device.default_input_config() {
            Ok(config) => config,
            Err(error) => {
                tracing::error!("Failed to get default input config: {}", error);
                return Err("Failed to get default input config".to_string());
            }
        };

        tracing::debug!("Setting up the recorder");
        self.target_rate = Some(config.sample_rate().0);
        self.channels = Some(config.channels());
        self.sample_size = Some(config.sample_format().sample_size() as u32);
        tracing::debug!("Config: {:?}", self);

        // Run the input stream on a separate thread.
        tracing::debug!("Clone recording signal mutex");
        let recording_signal = self.recording_signal_mutex.clone();

        // A signal to pass on the stream
        tracing::debug!("Create channel for passing data");
        let (sync_tx, sync_rx) = crossbeam_channel::unbounded::<Vec<TargetFormat>>();

        tracing::debug!("Begin recording...");
        thread::spawn(move || {
            let stream = match config.sample_format() {
                cpal::SampleFormat::I8 => match device.build_input_stream(
                    &config.into(),
                    move |data: &[i8], _: &_| {
                        let mut parsed_data: Vec<TargetFormat> = Vec::new();

                        for s_i in data {
                            parsed_data.push(s_i.to_sample::<TargetFormat>());
                        }

                        if let Err(e) = sync_tx.send(parsed_data) {
                            tracing::error!("Failed to send data: {}", e);
                        }
                    },
                    Recorder::err_fn,
                    None,
                ) {
                    Ok(stream) => stream,
                    Err(error) => {
                        tracing::error!("Failed to build input stream: {}", error);
                        return Err("Failed to build input stream".to_string());
                    }
                },
                cpal::SampleFormat::I16 => match device.build_input_stream(
                    &config.into(),
                    move |data: &[i16], _: &_| {
                        let mut parsed_data: Vec<TargetFormat> = Vec::new();

                        for s_i in data {
                            parsed_data.push(s_i.to_sample::<TargetFormat>());
                        }

                        if let Err(e) = sync_tx.send(parsed_data) {
                            tracing::error!("Failed to send data: {}", e);
                        }
                    },
                    Recorder::err_fn,
                    None,
                ) {
                    Ok(stream) => stream,
                    Err(error) => {
                        tracing::error!("Failed to build input stream: {}", error);
                        return Err("Failed to build input stream".to_string());
                    }
                },
                cpal::SampleFormat::I32 => match device.build_input_stream(
                    &config.into(),
                    move |data: &[i32], _: &_| {
                        let mut parsed_data: Vec<TargetFormat> = Vec::new();

                        for s_i in data {
                            parsed_data.push(s_i.to_sample::<TargetFormat>());
                        }

                        if let Err(e) = sync_tx.send(parsed_data) {
                            tracing::error!("Failed to send data: {}", e);
                        }
                    },
                    Recorder::err_fn,
                    None,
                ) {
                    Ok(stream) => stream,
                    Err(error) => {
                        tracing::error!("Failed to build input stream: {}", error);
                        return Err("Failed to build input stream".to_string());
                    }
                },
                cpal::SampleFormat::I64 => match device.build_input_stream(
                    &config.into(),
                    move |data: &[i64], _: &_| {
                        let mut parsed_data: Vec<TargetFormat> = Vec::new();

                        for s_i in data {
                            parsed_data.push(s_i.to_sample::<TargetFormat>());
                        }

                        if let Err(e) = sync_tx.send(parsed_data) {
                            tracing::error!("Failed to send data: {}", e);
                        }
                    },
                    Recorder::err_fn,
                    None,
                ) {
                    Ok(stream) => stream,
                    Err(error) => {
                        tracing::error!("Failed to build input stream: {}", error);
                        return Err("Failed to build input stream".to_string());
                    }
                },
                cpal::SampleFormat::U8 => match device.build_input_stream(
                    &config.into(),
                    move |data: &[u8], _: &_| {
                        let mut parsed_data: Vec<TargetFormat> = Vec::new();

                        for s_i in data {
                            parsed_data.push(s_i.to_sample::<TargetFormat>());
                        }

                        if let Err(e) = sync_tx.send(parsed_data) {
                            tracing::error!("Failed to send data: {}", e);
                        }
                    },
                    Recorder::err_fn,
                    None,
                ) {
                    Ok(stream) => stream,
                    Err(error) => {
                        tracing::error!("Failed to build input stream: {}", error);
                        return Err("Failed to build input stream".to_string());
                    }
                },
                cpal::SampleFormat::U16 => match device.build_input_stream(
                    &config.into(),
                    move |data: &[u16], _: &_| {
                        let mut parsed_data: Vec<TargetFormat> = Vec::new();

                        for s_i in data {
                            parsed_data.push(s_i.to_sample::<TargetFormat>());
                        }

                        if let Err(e) = sync_tx.send(parsed_data) {
                            tracing::error!("Failed to send data: {}", e);
                        }
                    },
                    Recorder::err_fn,
                    None,
                ) {
                    Ok(stream) => stream,
                    Err(error) => {
                        tracing::error!("Failed to build input stream: {}", error);
                        return Err("Failed to build input stream".to_string());
                    }
                },
                cpal::SampleFormat::U32 => match device.build_input_stream(
                    &config.into(),
                    move |data: &[u32], _: &_| {
                        let mut parsed_data: Vec<TargetFormat> = Vec::new();

                        for s_i in data {
                            parsed_data.push(s_i.to_sample::<TargetFormat>());
                        }

                        if let Err(e) = sync_tx.send(parsed_data) {
                            tracing::error!("Failed to send data: {}", e);
                        }
                    },
                    Recorder::err_fn,
                    None,
                ) {
                    Ok(stream) => stream,
                    Err(error) => {
                        tracing::error!("Failed to build input stream: {}", error);
                        return Err("Failed to build input stream".to_string());
                    }
                },
                cpal::SampleFormat::U64 => match device.build_input_stream(
                    &config.into(),
                    move |data: &[u64], _: &_| {
                        let mut parsed_data: Vec<TargetFormat> = Vec::new();

                        for s_i in data {
                            parsed_data.push(s_i.to_sample::<TargetFormat>());
                        }

                        if let Err(e) = sync_tx.send(parsed_data) {
                            tracing::error!("Failed to send data: {}", e);
                        }
                    },
                    Recorder::err_fn,
                    None,
                ) {
                    Ok(stream) => stream,
                    Err(error) => {
                        tracing::error!("Failed to build input stream: {}", error);
                        return Err("Failed to build input stream".to_string());
                    }
                },
                cpal::SampleFormat::F32 => match device.build_input_stream(
                    &config.into(),
                    move |data: &[f32], _: &_| {
                        let mut parsed_data: Vec<TargetFormat> = Vec::new();

                        for s_i in data {
                            parsed_data.push(s_i.to_sample::<TargetFormat>());
                        }

                        if let Err(e) = sync_tx.send(parsed_data) {
                            tracing::error!("Failed to send data: {}", e);
                        }
                    },
                    Recorder::err_fn,
                    None,
                ) {
                    Ok(stream) => stream,
                    Err(error) => {
                        tracing::error!("Failed to build input stream: {}", error);
                        return Err("Failed to build input stream".to_string());
                    }
                },
                cpal::SampleFormat::F64 => match device.build_input_stream(
                    &config.into(),
                    move |data: &[f64], _: &_| {
                        let mut parsed_data: Vec<TargetFormat> = Vec::new();

                        for s_i in data {
                            parsed_data.push(s_i.to_sample::<TargetFormat>());
                        }

                        if let Err(e) = sync_tx.send(parsed_data) {
                            tracing::error!("Failed to send data: {}", e);
                        }
                    },
                    Recorder::err_fn,
                    None,
                ) {
                    Ok(stream) => stream,
                    Err(error) => {
                        tracing::error!("Failed to build input stream: {}", error);
                        return Err("Failed to build input stream".to_string());
                    }
                },
                sample_format => {
                    tracing::error!("Unsupported sample format: {:?}", sample_format);
                    panic!("Unsupported sample format");
                }
            };

            tracing::info!("Stream started");
            if let Err(e) = stream.play() {
                tracing::error!("Failed to play stream: {}", e);
                return Err("Failed to play stream".to_string());
            };

            while *recording_signal.lock().unwrap() {
                thread::sleep(Duration::from_millis(CLOCK_DELAY as _));
            }

            tracing::debug!("Droping stream");
            drop(stream);

            tracing::info!("Recording stopped");
            Ok(())
        });

        Ok(sync_rx)
    }
}
