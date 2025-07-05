use std::iter::Sum;

use dasp_sample::ToSample;

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

pub trait CustomSample:
    cpal::Sample
    + num_traits::Num
    + num_traits::FromPrimitive
    + Sum
    + cpal::SizedSample
    + ToSample<TargetFormat>
{
}

impl CustomSample for i8 {}
impl CustomSample for i16 {}
impl CustomSample for i32 {}
impl CustomSample for i64 {}
impl CustomSample for u8 {}
impl CustomSample for u16 {}
impl CustomSample for u32 {}
impl CustomSample for u64 {}
impl CustomSample for f32 {}
impl CustomSample for f64 {}
