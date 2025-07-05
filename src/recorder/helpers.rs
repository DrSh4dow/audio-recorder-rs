use std::iter::Sum;

use cpal::StreamError;
use num_traits::{FromPrimitive, Num};

#[cfg(target_os = "macos")]
use crate::recorder::macos_utils;

use super::Recorder;

pub struct Config {
    pub sample_rate: u32,
    pub channels: u16,
    pub sample_size: u32,
}

impl Recorder {
    /// Checks if the recorder is currently recording.
    ///
    /// This function attempts to acquire a lock on the `recording_signal_mutex`.
    /// If the lock is successfully acquired, it returns the value of the recording signal.
    /// If the lock cannot be acquired, it logs the error and returns `true`, assuming
    /// that the recording signal is being used.
    ///
    /// # Returns
    ///
    /// * `true` - if the recording signal mutex is locked by another process or if the
    ///   recording signal indicates that recording is in progress.
    /// * `false` - if the recording signal indicates that recording is not in progress.
    ///
    /// # Logging
    ///
    /// This function logs the following events:
    /// * "Failed to lock the recording signal mutex: {}" - if the mutex lock fails,
    ///   with the error message.
    /// * "Returning true, as the recording signal is being used" - if the mutex lock fails.
    #[tracing::instrument]
    pub fn get_is_recording(&self) -> bool {
        tracing::debug!("Checking if the recorder is alive");

        let recording_signal = match self.recording_signal_mutex.try_lock() {
            Ok(recording_signal) => recording_signal,
            Err(error) => {
                tracing::info!("Failed to lock the recording signal mutex: {}", error);
                tracing::info!("Returning true, as the recording signal is being used");
                return true;
            }
        };

        *recording_signal
    }

    #[tracing::instrument]
    pub fn get_config(&self) -> Result<Config, String> {
        let sample_rate = match self.target_rate {
            Some(rate) => rate,
            None => {
                return Err("Sample rate not set".to_string());
            }
        };

        let channels = match self.channels {
            Some(channels) => channels,
            None => {
                return Err("Channels not set".to_string());
            }
        };

        let sample_size = match self.sample_size {
            Some(size) => size,
            None => {
                return Err("Sample size not set".to_string());
            }
        };

        Ok(Config {
            sample_rate,
            channels,
            sample_size,
        })
    }

    /// Prepare the recorder devices
    /// and create the virtual input devices for recording
    #[tracing::instrument]
    pub fn prepare(&mut self) -> Result<(), String> {
        tracing::debug!("Preparing recorder");

        let mut recording_signal = match self.recording_signal_mutex.lock() {
            Ok(recording_signal) => recording_signal,
            Err(error) => {
                tracing::error!("Failed to lock the recording signal mutex: {}", error);
                return Err("Failed to lock the recording signal mutex".to_string());
            }
        };

        if *recording_signal {
            return Err("Recording already in progress".to_string());
        }

        *recording_signal = true;
        self.sample_size = None;
        self.channels = None;
        self.target_rate = None;

        #[cfg(target_os = "macos")]
        {
            tracing::info!("Saving the original default device");
            let default_device = match macos_utils::get_current_default_device() {
                Ok(device) => device,
                Err(e) => {
                    tracing::error!("Error getting the default device: {}", e);
                    return Ok(());
                }
            };
            self.original_output_device_name = Some(default_device);
            tracing::info!("Setting the default device to the multi-output device");
            if let Err(e) = macos_utils::switch_device("mappa-recorder-device") {
                tracing::error!("Error switching the default device: {}", e);
            };
        }

        tracing::debug!("Recorder prepared");

        Ok(())
    }

    /// Converts multi-channel audio data to mono by averaging the channels.
    ///
    /// # Parameters
    ///
    /// - `stereo_data`: A vector containing the audio data. Each element represents a sample.
    /// - `channels`: The number of channels in the audio data (e.g., 2 for stereo).
    ///
    /// # Returns
    ///
    /// A vector containing the mono audio data, where each sample is the average of the corresponding
    /// samples from the input channels.
    ///
    /// # Panics
    ///
    /// This function will panic if the length of `stereo_data` is not a multiple of `channels`.
    ///
    /// # Type Parameters
    ///
    /// - `T`: The type of the audio samples. It must implement the `Num`, `Copy`, `Sum`, and `FromPrimitive` traits.
    ///
    /// # Examples
    ///
    /// ```
    /// use num_traits::{Num, FromPrimitive};
    /// use std::iter::Sum;
    ///
    /// fn channels_to_mono<T>(stereo_data: Vec<T>, channels: u16) -> Vec<T>
    /// where
    ///     T: Num + Copy + Sum + FromPrimitive,
    /// {
    ///     let channels = channels as usize; // Convert u16 to usize for safe indexing and operations
    ///
    ///     // Ensure that the length of stereo_data is a multiple of the number of channels
    ///     assert!(
    ///         stereo_data.len() % channels == 0,
    ///         "Data length must be a multiple of the number of channels"
    ///     );
    ///
    ///     // Create a new vector to hold the mono data
    ///     let mut mono_data = Vec::with_capacity(stereo_data.len() / channels);
    ///
    ///     // Iterate over the stereo data in chunks of 'channels'
    ///     for chunk in stereo_data.chunks(channels) {
    ///         // Compute the average of the chunk
    ///         let mono = chunk.iter().copied().sum::<T>() / T::from_usize(channels).unwrap();
    ///         mono_data.push(mono);
    ///     }
    ///
    ///     mono_data
    /// }
    /// ```
    pub fn channels_to_mono<T>(stereo_data: Vec<T>, channels: u16) -> Vec<T>
    where
        T: Num + Copy + Sum + FromPrimitive,
    {
        let channels = channels as usize; // Convert u16 to usize for safe indexing and operations

        // Ensure that the length of stereo_data is a multiple of the number of channels
        assert!(
            stereo_data.len() % channels == 0,
            "Data length must be a multiple of the number of channels"
        );

        // Create a new vector to hold the mono data
        let mut mono_data = Vec::with_capacity(stereo_data.len() / channels);

        // Iterate over the stereo data in chunks of 'channels'
        for chunk in stereo_data.chunks(channels) {
            // Compute the average of the chunk
            let mono = chunk.iter().copied().sum::<T>() / T::from_usize(channels).unwrap();
            mono_data.push(mono);
        }

        mono_data
    }

    pub fn err_fn(err: StreamError) {
        tracing::error!("an error occurred on stream: {}", err);
    }
}
