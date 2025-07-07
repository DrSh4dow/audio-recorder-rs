use std::fmt::Display;

#[derive(Debug)]
pub enum AudioRecorderError {
    FailedToGetDevice(&'static str),
    RecordingInProgress,
}

impl Display for AudioRecorderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AudioRecorderError::FailedToGetDevice(msg) => {
                write!(f, "Failed to get device: {}", msg)
            }
            AudioRecorderError::RecordingInProgress => {
                write!(f, "Recording is already in progress")
            }
        }
    }
}
