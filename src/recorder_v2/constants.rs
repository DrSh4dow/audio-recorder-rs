pub type TargetFormat = f32;
pub const CLOCK_DELAY: u32 = 400;

pub const RESAMPLER_SLEEP_DELAY: u32 = 10;
pub const RESAMPLER_CHUNK_SIZE: usize = 44100;

pub enum ResampleTargetStream {
    /// Resample the input stream to achieve the output rate
    Input,
    /// Resample the output stream to achieve
    Output,
    /// No resampling
    None,
}
