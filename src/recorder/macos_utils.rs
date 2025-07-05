#[cfg(target_os = "macos")]
use cpal::traits::{DeviceTrait, HostTrait};

#[cfg(target_os = "macos")]
pub fn find_macos_monitor_source(host: &cpal::Host) -> Result<cpal::Device, String> {
    let devices = match host.devices() {
        Ok(devices) => devices,
        Err(e) => {
            tracing::error!("Error finding devices: {:?}", e);
            return Err(String::from("Error finding devices"));
        }
    };

    for device in devices {
        let name = device.name().unwrap_or(String::from("UNKNOWN"));
        tracing::debug!("Found device: {}", name);

        if name.trim().to_lowercase().contains("blackhole") {
            return Ok(device);
        }
    }

    Err(String::from("(Blackhole monitor not found)"))
}

/// Gets the current default audio device using SwitchAudioSource.
/// Requires `SwitchAudioSource` to be installed.
#[cfg(target_os = "macos")]
pub fn get_current_default_device() -> Result<String, String> {
    use std::process::Command;

    let output = match Command::new("SwitchAudioSource").arg("-c").output() {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Error getting current default device on MacOS: {:?}", e);
            return Err(String::from(
                "Error getting current default device on MacOS",
            ));
        }
    };

    if !output.status.success() {
        tracing::error!(
            "Error getting current default device on MacOS: {:?}",
            output.status
        );
        return Err(String::from(
            "Error getting current default device on MacOS",
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Switches the default audio device to the specified device using SwitchAudioSource.
#[cfg(target_os = "macos")]
pub fn switch_device(device: &str) -> Result<(), String> {
    use std::process::Command;

    let status = match Command::new("SwitchAudioSource")
        .arg("-s")
        .arg(device)
        .status()
    {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Error switching device on MacOS: {:?}", e);
            return Err(format!("Error switching device on MacOS: {:?}", e));
        }
    };

    if status.success() {
        tracing::info!("Switched to device: {}", device);
        Ok(())
    } else {
        tracing::error!("Failed to switch to device: {}", device);
        Err(format!("Failed to switch to device: {}", device))
    }
}
