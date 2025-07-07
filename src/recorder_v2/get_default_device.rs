use cpal::traits::{DeviceTrait, HostTrait};

use super::errors::AudioRecorderError;

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub enum DeviceType {
    Input,
    Output,
}

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct AudioDevice {
    pub name: String,
    pub device_type: DeviceType,
}

impl AudioDevice {
    pub fn new(name: String, device_type: DeviceType) -> Self {
        AudioDevice { name, device_type }
    }

    pub fn from_name(name: &str) -> Result<Self, AudioRecorderError> {
        if name.trim().is_empty() {
            return Err(AudioRecorderError::FailedToGetDevice(
                "Device name cannot be empty",
            ));
        }

        let (name, device_type) = if name.to_lowercase().ends_with("(input)") {
            (
                name.trim_end_matches("(input)").trim().to_string(),
                DeviceType::Input,
            )
        } else if name.to_lowercase().ends_with("(output)") {
            (
                name.trim_end_matches("(output)").trim().to_string(),
                DeviceType::Output,
            )
        } else {
            return Err(AudioRecorderError::FailedToGetDevice(
                "Device type (input/output) not specified in the name",
            ));
        };

        Ok(AudioDevice::new(name, device_type))
    }

    pub fn get_default_input_device() -> Result<Self, AudioRecorderError> {
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

        let device_name = device.name().map_err(|e| {
            tracing::error!("Failed to get device name: {}", e);
            AudioRecorderError::FailedToGetDevice("Failed to get device name")
        })?;

        // TODO: implement linux device selection (this changes due to pipewire/alsa/pulseaudio)
        Ok(AudioDevice::new(device_name, DeviceType::Input))
    }

    pub fn get_default_output_device() -> Result<Self, AudioRecorderError> {
        #[cfg(target_os = "macos")]
        {
            tracing::debug!("Using macOS specific host for audio device selection");
            tracing::debug!("Trying to use ScreenCaptureKit to capture system audio");
            // ! see https://github.com/RustAudio/cpal/pull/894
            if let Ok(host) = cpal::host_from_id(cpal::HostId::ScreenCaptureKit) {
                if let Some(device) = host.default_input_device() {
                    if let Ok(name) = device.name() {
                        return Ok(AudioDevice::new(name, DeviceType::Output));
                    }
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

            let device_name = device.name().map_err(|e| {
                tracing::error!("Failed to get device name: {}", e);
                AudioRecorderError::FailedToGetDevice("Failed to get device name")
            })?;
            return Ok(AudioDevice::new(device_name, DeviceType::Output));
        }

        #[cfg(target_os = "windows")]
        {
            tracing::debug!("Using Windows specific host for audio device selection");
            // Try WASAPI host first for Windows
            if let Ok(wasapi_host) = cpal::host_from_id(cpal::HostId::Wasapi) {
                if let Some(device) = wasapi_host.default_output_device() {
                    if let Ok(name) = device.name() {
                        return Ok(AudioDevice::new(name, DeviceType::Output));
                    }
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

            let device_name = device.name().map_err(|e| {
                tracing::error!("Failed to get device name: {}", e);
                AudioRecorderError::FailedToGetDevice("Failed to get device name")
            })?;

            Ok(AudioDevice::new(device_name, DeviceType::Output))
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

            let device_name = device.name().map_err(|e| {
                tracing::error!("Failed to get device name: {}", e);
                AudioRecorderError::FailedToGetDevice("Failed to get device name")
            })?;

            Ok(AudioDevice::new(device_name, DeviceType::Output))
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

            let device_name = device.name().map_err(|e| {
                tracing::error!("Failed to get device name: {}", e);
                AudioRecorderError::FailedToGetDevice("Failed to get device name")
            })?;

            Ok(AudioDevice::new(device_name, DeviceType::Output))
        }
    }
}
