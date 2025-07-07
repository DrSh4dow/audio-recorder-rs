use std::fmt::Display;

#[derive(Debug)]
pub enum AudioRecorderError {
    DeviceError(&'static str),
    RecordingInProgress,
}

impl Display for AudioRecorderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AudioRecorderError::DeviceError(msg) => {
                write!(f, "Device Error: {}", msg)
            }
            AudioRecorderError::RecordingInProgress => {
                write!(f, "Recording is already in progress")
            }
        }
    }
}
