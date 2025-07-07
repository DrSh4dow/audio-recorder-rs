use std::{
    sync::atomic::Ordering,
    thread::{self, sleep},
    time::Duration,
};

use cpal::{
    Sample,
    traits::{DeviceTrait, StreamTrait},
};
use crossbeam_channel::Receiver;
use ringbuf::{
    HeapRb,
    traits::{Consumer, Observer, Producer, Split},
};
use rubato::{FftFixedIn, Resampler};

use super::{
    constants::{CustomSample, RESAMPLER_CHUNK_SIZE, RESAMPLER_SLEEP_DELAY, TargetFormat},
    errors::AudioRecorderError,
};

use super::Recorder;

impl Recorder {
    pub fn with_input_resampler<T, U>(
        &self,
        input_device: cpal::Device,
        output_device: cpal::Device,
        target_rate: usize,
        origin_rate: usize,
    ) -> Result<Receiver<Vec<TargetFormat>>, AudioRecorderError>
    where
        T: CustomSample + 'static,
        U: CustomSample + 'static,
    {
        tracing::info!("Starting the recorder with input resampler");
        // using the same config for input and output
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

        let buffer_size = RESAMPLER_CHUNK_SIZE * 2;

        tracing::debug!("Creating ring buffers...");
        let ring_output = HeapRb::<TargetFormat>::new(buffer_size);
        let ring_input = HeapRb::<TargetFormat>::new(buffer_size);
        let ring_resampler = HeapRb::<TargetFormat>::new(buffer_size);

        tracing::debug!("Splitting ring buffers...");
        let (mut producer_input, mut consumer_input) = ring_input.split();
        let (mut producer_output, mut consumer_output) = ring_output.split();
        let (mut producer_resampler, mut consumer_resampler) = ring_resampler.split();

        // A signal to pass on the stream
        tracing::debug!("Creating sync channel...");
        let (sync_tx, sync_rx) = crossbeam_channel::unbounded();

        // A flag to indicate that recording is in progress.
        tracing::debug!("Begin recording...");

        // Run the input stream on a separate thread.
        let recording_signal = self.recording_signal.clone();

        let output_channels = output_config.channels();
        let input_channels = input_config.channels();

        // ring buffer writers for input and output
        let write_output_data = move |data: &[U], _: &_| {
            let data = Recorder::channels_to_mono(data.to_vec(), output_channels);

            for sample in data {
                if producer_output
                    .try_push(sample.to_sample::<TargetFormat>())
                    .is_err()
                {
                    tracing::error!("output stream 1 fell behind: try increasing CHUNK_SIZE");
                }
            }
        };

        let write_input_data = move |data: &[T], _: &_| {
            let data = Recorder::channels_to_mono(data.to_vec(), input_channels);

            for sample in data {
                if producer_resampler
                    .try_push(sample.to_sample::<TargetFormat>())
                    .is_err()
                {
                    tracing::error!("output stream 2 fell behind: try increasing CHUNK_SIZE");
                }
            }
        };

        tracing::debug!("Spawning input stream thread...");
        thread::spawn(move || {
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

            let recording_signal_2 = recording_signal.clone();

            // resampler thread
            thread::spawn(move || {
                let mut resampler =
                    match FftFixedIn::<TargetFormat>::new(origin_rate, target_rate, 1024, 2, 1) {
                        Ok(r) => r,
                        Err(e) => {
                            return Err(format!("Failed to create resampler: {e}"));
                        }
                    };

                let mut resampler_output_buffer = resampler.output_buffer_allocate(true);
                let mut next_input_frames = resampler.input_frames_next();

                loop {
                    if !recording_signal_2.load(Ordering::SeqCst) {
                        break;
                    }

                    while consumer_resampler.occupied_len() >= next_input_frames {
                        let mut data_buffer = vec![0.0; next_input_frames];

                        consumer_resampler.pop_slice(&mut data_buffer);

                        match resampler.process_into_buffer(
                            &[&data_buffer],
                            &mut resampler_output_buffer,
                            None,
                        ) {
                            Ok(_) => {
                                next_input_frames = resampler.input_frames_next();
                                let output_data = &resampler_output_buffer[0];
                                // drain the output buffer
                                producer_input.push_slice(output_data);
                            }
                            Err(e) => {
                                tracing::error!("Failed to resample: {}", e);
                            }
                        };
                    }

                    sleep(Duration::from_millis(RESAMPLER_SLEEP_DELAY as _));
                }

                Ok(())
            });

            if let Err(e) = input_stream.play() {
                tracing::error!("Failed to play input stream: {}", e);
                return;
            };
            if let Err(e) = output_stream.play() {
                tracing::error!("Failed to play output stream: {}", e);
                return;
            };

            while recording_signal.load(Ordering::SeqCst) {
                if consumer_output.occupied_len() >= target_rate
                    || consumer_input.occupied_len() >= target_rate
                {
                    let mut input_buffer = vec![TargetFormat::EQUILIBRIUM; target_rate];
                    let mut output_buffer = vec![TargetFormat::EQUILIBRIUM; target_rate];

                    consumer_input.pop_slice(&mut input_buffer);
                    consumer_output.pop_slice(&mut output_buffer);

                    let mut data: Vec<TargetFormat> = Vec::with_capacity(target_rate * 2);

                    for (i, o) in input_buffer.iter().zip(output_buffer.iter()) {
                        data.push(*i);
                        data.push(*o);
                    }

                    if let Err(e) = sync_tx.send(data) {
                        tracing::error!("Failed to send data: {}", e);
                    }
                }

                sleep(Duration::from_millis(RESAMPLER_SLEEP_DELAY as _));
            }

            tracing::debug!("Pausing streams");
            if let Err(e) = input_stream.pause() {
                tracing::error!("Failed to pause input stream: {}", e);
            };

            if let Err(e) = output_stream.pause() {
                tracing::error!("Failed to pause output stream: {}", e);
            };
            tracing::debug!("Droping stream");
            drop(input_stream);
            drop(output_stream);
            tracing::info!("Recording stopped");
        });

        Ok(sync_rx)
    }

    pub fn with_output_resampler<T, U>(
        &self,
        input_device: cpal::Device,
        output_device: cpal::Device,
        target_rate: usize,
        origin_rate: usize,
    ) -> Result<Receiver<Vec<TargetFormat>>, AudioRecorderError>
    where
        T: CustomSample + 'static,
        U: CustomSample + 'static,
    {
        tracing::info!("Recording with output resampler");
        // using the same config for input and output
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

        let buffer_size = RESAMPLER_CHUNK_SIZE * 2;

        tracing::debug!("Creating ring buffers...");
        let ring_output = HeapRb::<TargetFormat>::new(buffer_size);
        let ring_input = HeapRb::<TargetFormat>::new(buffer_size);
        let ring_resampler = HeapRb::<TargetFormat>::new(buffer_size);

        tracing::debug!("Splitting ring buffers...");
        let (mut producer_input, mut consumer_input) = ring_input.split();
        let (mut producer_output, mut consumer_output) = ring_output.split();
        let (mut producer_resampler, mut consumer_resampler) = ring_resampler.split();

        // A signal to pass on the stream
        let (sync_tx, sync_rx) = crossbeam_channel::unbounded();

        // A flag to indicate that recording is in progress.
        tracing::debug!("Begin recording...");

        // Run the input stream on a separate thread.
        let recording_signal = self.recording_signal.clone();

        let output_channels = output_config.channels();
        let input_channels = input_config.channels();

        // ring buffer writers for input and output
        let write_output_data = move |data: &[TargetFormat], _: &_| {
            let data = Recorder::channels_to_mono(data.to_vec(), output_channels);

            for sample in data {
                if producer_resampler
                    .try_push(sample.to_sample::<TargetFormat>())
                    .is_err()
                {
                    tracing::error!("output stream 2 fell behind: try increasing CHUNK_SIZE");
                }
            }
        };

        let write_input_data = move |data: &[TargetFormat], _: &_| {
            let data = Recorder::channels_to_mono(data.to_vec(), input_channels);

            for sample in data {
                if producer_input
                    .try_push(sample.to_sample::<TargetFormat>())
                    .is_err()
                {
                    tracing::error!("output stream 2 fell behind: try increasing CHUNK_SIZE");
                }
            }
        };

        tracing::debug!("Spawning input stream thread...");
        thread::spawn(move || {
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

            let recording_signal_2 = recording_signal.clone();

            // resampler thread
            thread::spawn(move || {
                let mut resampler =
                    match FftFixedIn::<TargetFormat>::new(origin_rate, target_rate, 1024, 2, 1) {
                        Ok(r) => r,
                        Err(e) => {
                            tracing::error!("Failed to create resampler: {}", e);
                            return;
                        }
                    };

                let mut resampler_output_buffer = resampler.output_buffer_allocate(true);
                let mut next_input_frames = resampler.input_frames_next();

                loop {
                    if !recording_signal_2.load(Ordering::SeqCst) {
                        break;
                    }

                    while consumer_resampler.occupied_len() >= next_input_frames {
                        let mut data_buffer = vec![0.0; next_input_frames];

                        consumer_resampler.pop_slice(&mut data_buffer);

                        match resampler.process_into_buffer(
                            &[&data_buffer],
                            &mut resampler_output_buffer,
                            None,
                        ) {
                            Ok(_) => {
                                next_input_frames = resampler.input_frames_next();
                                let output_data = &resampler_output_buffer[0];
                                // drain the output buffer
                                producer_output.push_slice(output_data);
                            }
                            Err(e) => {
                                tracing::error!("Failed to resample: {}", e);
                            }
                        };
                    }

                    sleep(Duration::from_millis(RESAMPLER_SLEEP_DELAY as _));
                }
            });

            if let Err(e) = input_stream.play() {
                tracing::error!("Failed to play input stream: {}", e);
                return;
            };
            if let Err(e) = output_stream.play() {
                tracing::error!("Failed to play output stream: {}", e);
                return;
            };

            while recording_signal.load(Ordering::SeqCst) {
                if consumer_output.occupied_len() >= target_rate
                    || consumer_input.occupied_len() >= target_rate
                {
                    let mut input_buffer = vec![TargetFormat::EQUILIBRIUM; target_rate];
                    let mut output_buffer = vec![TargetFormat::EQUILIBRIUM; target_rate];

                    consumer_input.pop_slice(&mut input_buffer);
                    consumer_output.pop_slice(&mut output_buffer);

                    let mut data: Vec<TargetFormat> = Vec::with_capacity(target_rate * 2);

                    for (i, o) in input_buffer.iter().zip(output_buffer.iter()) {
                        data.push(*i);
                        data.push(*o);
                    }

                    if let Err(e) = sync_tx.send(data) {
                        tracing::error!("Failed to send data: {}", e);
                    }
                }

                sleep(Duration::from_millis(RESAMPLER_SLEEP_DELAY as _));
            }

            tracing::debug!("Pausing streams");
            if let Err(e) = input_stream.pause() {
                tracing::error!("Failed to pause input stream: {}", e);
            };

            if let Err(e) = output_stream.pause() {
                tracing::error!("Failed to pause output stream: {}", e);
            };
            tracing::debug!("Droping stream");
            drop(input_stream);
            drop(output_stream);
            tracing::info!("Recording stopped");
        });

        Ok(sync_rx)
    }
}
