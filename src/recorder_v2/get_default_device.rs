use cpal::{Device, traits::HostTrait};

use super::errors::AudioRecorderError;

pub fn get_default_input_device() -> Result<Device, AudioRecorderError> {
    let host = cpal::default_host();
    let device = match host.default_input_device() {
        Some(d) => d,
        None => {
            tracing::error!("Failed to get default input device");
            return Err(AudioRecorderError::FailedToGetDevice(
                "No default input device found",
            ));
        }
    };

    // TODO: implement linux device selection (this changes due to pipewire/alsa/pulseaudio)
    Ok(device)
}

pub fn get_default_output_device() -> Result<Device, AudioRecorderError> {
    #[cfg(target_os = "macos")]
    {
        tracing::debug!("Using macOS specific host for audio device selection");
        tracing::debug!("Trying to use ScreenCaptureKit to capture system audio");
        // ! see https://github.com/RustAudio/cpal/pull/894
        if let Ok(host) = cpal::host_from_id(cpal::HostId::ScreenCaptureKit) {
            if let Some(device) = host.default_input_device() {
                return Ok(device);
            }
        }

        tracing::warn!(
            "Falling back to default host for audio device selection, this usually will not work for system audio capture on macOS"
        );
        let device = match host.default_output_device() {
            Some(d) => d,
            None => {
                tracing::error!("Failed to get default output device");
                return Err(AudioRecorderError::FailedToGetDevice(
                    "No default output device found",
                ));
            }
        };

        return Ok(device);
    }

    #[cfg(target_os = "windows")]
    {
        tracing::debug!("Using Windows specific host for audio device selection");
        // Try WASAPI host first for Windows
        if let Ok(wasapi_host) = cpal::host_from_id(cpal::HostId::Wasapi) {
            if let Some(device) = wasapi_host.default_output_device() {
                return Ok(device);
            }
        }

        // Fallback to default host if WASAPI fails
        let host = cpal::default_host();
        let device = match host.default_output_device() {
            Some(d) => d,
            None => {
                tracing::error!("Failed to get default output device");
                return Err(AudioRecorderError::FailedToGetDevice(
                    "No default output device found",
                ));
            }
        };

        Ok(device)
    }

    #[cfg(target_os = "linux")]
    {
        // TODO: implement linux device selection (this changes due to pipewire/alsa/pulseaudio)
        // For Linux, we can use the default host
        let host = cpal::default_host();
        let device = match host.default_output_device() {
            Some(d) => d,
            None => {
                tracing::error!("Failed to get default output device");
                return Err(AudioRecorderError::FailedToGetDevice(
                    "No default output device found",
                ));
            }
        };

        Ok(device)
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        let host = cpal::default_host();
        let device = match host.default_output_device() {
            Some(d) => d,
            None => {
                tracing::error!("Failed to get default output device");
                return Err(AudioRecorderError::FailedToGetDevice(
                    "No default output device found",
                ));
            }
        };

        Ok(device)
    }
}
