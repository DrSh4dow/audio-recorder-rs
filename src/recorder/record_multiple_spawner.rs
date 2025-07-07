use cpal::traits::DeviceTrait;
use crossbeam_channel::Receiver;

use super::{
    Recorder,
    constants::{CustomSample, ResampleTargetStream, TargetFormat},
    errors::AudioRecorderError,
};

impl Recorder {
    pub fn record_multiple<T, U>(
        &mut self,
        input_device: cpal::Device,
        output_device: cpal::Device,
    ) -> Result<Receiver<Vec<TargetFormat>>, AudioRecorderError>
    where
        T: CustomSample + 'static,
        U: CustomSample + 'static,
    {
        tracing::debug!("Record multiple started");

        tracing::debug!("Collecting input and output configs");
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

        tracing::debug!("Calculating resampling target");
        // calculate the resampling target
        let input_sample_rate = input_config.sample_rate().0;
        let output_sample_rate = output_config.sample_rate().0;

        let (resampler_target, target_rate, origin_rate) =
            match input_sample_rate.cmp(&output_sample_rate) {
                std::cmp::Ordering::Less => (
                    ResampleTargetStream::Output,
                    input_sample_rate as usize,
                    output_sample_rate as usize,
                ), // resample output to achieve input rate
                std::cmp::Ordering::Equal => (
                    ResampleTargetStream::None,
                    input_sample_rate as usize,
                    output_sample_rate as usize,
                ), // no resampling
                std::cmp::Ordering::Greater => (
                    ResampleTargetStream::Input,
                    output_sample_rate as usize,
                    input_sample_rate as usize,
                ), // resample input to achieve output rate
            };

        tracing::debug!("Setting up the recorder");
        self.target_sample_rate = Some(target_rate as u32);
        self.channels = Some(2);
        self.sample_size = Some(input_config.sample_format().sample_size() as u32);

        tracing::debug!("Config: {:?}", self);

        // start recording
        match resampler_target {
            ResampleTargetStream::None => {
                self.without_resampler::<T, U>(input_device, output_device)
            }
            ResampleTargetStream::Output => self.with_output_resampler::<T, U>(
                input_device,
                output_device,
                target_rate,
                origin_rate,
            ),
            ResampleTargetStream::Input => self.with_input_resampler::<T, U>(
                input_device,
                output_device,
                target_rate,
                origin_rate,
            ),
        }
    }
}
