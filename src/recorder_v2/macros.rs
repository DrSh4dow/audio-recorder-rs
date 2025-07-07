/// Expands to the correct `self.record_multiple::<In, Out>(…)` call
/// for every (input, output) sample-format pair.
///
/// Usage:
/// `record_multiple_expansion!(self, input_config, output_config, input_device, output_device)`
#[macro_export]
macro_rules! record_multiple_expansion {
    // ── entry point ───────────────────────────────────────────────────────────
    ($self_:expr, $in_cfg:expr, $out_cfg:expr, $in_dev:expr, $out_dev:expr) => {{
        // helper: second-level match (output side) for a **fixed** input type
        macro_rules! match_output {
            ($in_ty:ty) => {{
                match $out_cfg.sample_format() {
                    cpal::SampleFormat::I8 => {
                        $self_.record_multiple::<$in_ty, i8>($in_dev, $out_dev)
                    }
                    cpal::SampleFormat::I16 => {
                        $self_.record_multiple::<$in_ty, i16>($in_dev, $out_dev)
                    }
                    cpal::SampleFormat::I32 => {
                        $self_.record_multiple::<$in_ty, i32>($in_dev, $out_dev)
                    }
                    cpal::SampleFormat::I64 => {
                        $self_.record_multiple::<$in_ty, i64>($in_dev, $out_dev)
                    }
                    cpal::SampleFormat::U8 => {
                        $self_.record_multiple::<$in_ty, u8>($in_dev, $out_dev)
                    }
                    cpal::SampleFormat::U16 => {
                        $self_.record_multiple::<$in_ty, u16>($in_dev, $out_dev)
                    }
                    cpal::SampleFormat::U32 => {
                        $self_.record_multiple::<$in_ty, u32>($in_dev, $out_dev)
                    }
                    cpal::SampleFormat::U64 => {
                        $self_.record_multiple::<$in_ty, u64>($in_dev, $out_dev)
                    }
                    cpal::SampleFormat::F32 => {
                        $self_.record_multiple::<$in_ty, f32>($in_dev, $out_dev)
                    }
                    cpal::SampleFormat::F64 => {
                        $self_.record_multiple::<$in_ty, f64>($in_dev, $out_dev)
                    }
                    sf => Err(format!("Unsupported sample format '{sf:?}'")),
                }
            }};
        }

        // first-level match (input side)
        match $in_cfg.sample_format() {
            cpal::SampleFormat::I8 => match_output!(i8),
            cpal::SampleFormat::I16 => match_output!(i16),
            cpal::SampleFormat::I32 => match_output!(i32),
            cpal::SampleFormat::I64 => match_output!(i64),
            cpal::SampleFormat::U8 => match_output!(u8),
            cpal::SampleFormat::U16 => match_output!(u16),
            cpal::SampleFormat::U32 => match_output!(u32),
            cpal::SampleFormat::U64 => match_output!(u64),
            cpal::SampleFormat::F32 => match_output!(f32),
            cpal::SampleFormat::F64 => match_output!(f64),
            sf => Err(format!("Unsupported sample format '{sf:?}'")),
        }
    }};
}
