use std::{
    thread::{self, sleep},
    time::Duration,
};

use cpal::traits::{DeviceTrait, StreamTrait};
use crossbeam_channel::Receiver;
use dasp_sample::Sample;
use ringbuf::{
    traits::{Consumer, Producer, Split},
    HeapRb,
};

use crate::recorder::common::CLOCK_DELAY;

use super::{
    common::{CustomSample, TargetFormat},
    Recorder,
};

impl Recorder {
    pub fn without_resampler<T, U>(
        &self,
        input_device: cpal::Device,
        output_device: cpal::Device,
    ) -> Result<Receiver<Vec<TargetFormat>>, String>
    where
        T: CustomSample + 'static,
        U: CustomSample + 'static,
    {
        tracing::info!("Starting the recorder without resampler");
        tracing::debug!("Collecting input and output configs");
        // using the same config for input and output
        let input_config = match input_device.default_input_config() {
            Ok(c) => c,
            Err(e) => {
                return Err(format!("Failed to get input config: {}", e));
            }
        };
        let output_config = match output_device.default_output_config() {
            Ok(c) => c,
            Err(e) => {
                return Err(format!("Failed to get output config: {}", e));
            }
        };

        // We'll try and use the same configuration between streams to keep it simple.
        let config: cpal::StreamConfig = input_config.clone().into();

        // Create a delay in case the input and output devices aren't synced.
        let latency_frames = (150.0 / 1_000.0) * config.sample_rate.0 as f32;
        let latency_samples = latency_frames as usize * config.channels as usize;

        tracing::debug!("Latency samples: {}", latency_samples);
        tracing::debug!("Latency frames: {}", latency_frames);

        // The buffer to share samples
        tracing::debug!("Creating ring buffer...");
        let ring = HeapRb::<TargetFormat>::new(latency_samples * 2);

        tracing::debug!("Splitting ring buffers...");
        let (mut producer, mut consumer) = ring.split();

        // A signal to pass on the stream
        tracing::debug!("Creating sync channel...");
        let (sync_tx, sync_rx) = crossbeam_channel::unbounded();

        // Fill the samples with 0.0 equal to the length of the delay.
        tracing::debug!("Filling ring buffer with EQUILIBRIUM samples");
        for _ in 0..latency_samples {
            // The ring buffer has twice as much space as necessary to add latency here,
            // so this should never fail
            if let Err(e) = producer.try_push(TargetFormat::EQUILIBRIUM) {
                tracing::error!("Failed to push equilibrium sample: {}", e);
            }
        }

        // A flag to indicate that recording is in progress.
        tracing::debug!("Clone recording signal mutex...");
        let recording_signal = self.recording_signal_mutex.clone();

        let output_channels = output_config.channels();
        let input_channels = input_config.channels();

        // ring buffer writers for input and output
        let write_output_data = move |data: &[U], _: &_| {
            let data = Recorder::channels_to_mono(data.to_vec(), output_channels);

            for sample in data {
                if producer
                    .try_push(sample.to_sample::<TargetFormat>())
                    .is_err()
                {
                    tracing::warn!("output stream fell behind: increase buffer size");
                }
            }
        };

        let write_input_data = move |data: &[T], _: &_| {
            let data = Recorder::channels_to_mono(data.to_vec(), input_channels);
            let mut parsed_data: Vec<TargetFormat> = Vec::new();

            for s_i in data {
                parsed_data.push(s_i.to_sample::<TargetFormat>());
                if let Some(s_o) = consumer.try_pop() {
                    parsed_data.push(s_o.to_sample::<TargetFormat>());
                } else {
                    parsed_data.push(TargetFormat::EQUILIBRIUM.to_sample());
                }
            }

            if let Err(e) = sync_tx.send(parsed_data) {
                tracing::error!("Failed to send data: {}", e);
            }
        };

        let record_signal_clone_1 = recording_signal.clone();
        tracing::debug!("Spawning stream thread...");
        thread::spawn(move || {
            // Build the input stream
            let input_stream = match input_device.build_input_stream(
                &input_config.into(),
                write_input_data,
                Recorder::err_fn,
                None,
            ) {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!("Failed to build input stream: {}", e);
                    return;
                }
            };

            // Build the output stream
            let output_stream = match output_device.build_input_stream(
                &output_config.into(),
                write_output_data,
                Recorder::err_fn,
                None,
            ) {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!("Failed to build output stream: {}", e);
                    return;
                }
            };

            tracing::debug!("Starting streams...");

            // Start the streams
            if let Err(e) = input_stream.play() {
                tracing::error!("Failed to play input stream: {}", e);
                return;
            };
            if let Err(e) = output_stream.play() {
                tracing::error!("Failed to play output stream: {}", e);
                return;
            };

            while *record_signal_clone_1.lock().unwrap() {
                sleep(Duration::from_millis(CLOCK_DELAY as _));
            }

            tracing::debug!("Droping streams");

            if let Err(e) = input_stream.pause() {
                tracing::error!("Failed to pause input stream: {}", e);
            };

            if let Err(e) = output_stream.pause() {
                tracing::error!("Failed to pause output stream: {}", e);
            };

            drop(input_stream);
            tracing::debug!("input stream dropped");

            // drop audio streams
            drop(output_stream);
            tracing::debug!("output stream dropped");
        });

        Ok(sync_rx)
    }
}
