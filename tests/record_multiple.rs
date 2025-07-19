use audio_recorder_rs::Recorder;
use tracing_test::traced_test;

#[test]
#[traced_test]
fn record_multiple_devices() {
    tracing::info!("Spawning record multiple test");

    tracing::info!("Creating recorder");
    let mut recorder = Recorder::new();

    tracing::info!("Starting recorder");
    let receiver = match recorder.start(false) {
        Ok(receiver) => receiver,
        Err(e) => {
            panic!("Failed to start recorder: {e}");
        }
    };
    tracing::info!("Recorder started");

    tracing::info!("Asserting if is recording");
    assert!(recorder.get_is_recording(), "Recorder is not recording");

    let config = match recorder.get_config() {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Failed to get config: {}", e);
            panic!("Failed to get config");
        }
    };

    let mut writer = hound::WavWriter::create(
        "output.wav",
        hound::WavSpec {
            sample_rate: config.sample_rate,
            channels: config.channels,
            bits_per_sample: (config.sample_size * 8) as _,
            sample_format: hound::SampleFormat::Float,
        },
    )
    .expect("Could not generate writer");

    tracing::info!("Awaiting data");
    let instant = std::time::Instant::now();

    while let Ok(d) = receiver.recv() {
        if instant.elapsed().as_secs() > 8 {
            break;
        }

        for sample in d {
            writer.write_sample(sample).ok();
        }
    }
    tracing::info!("Finished recording");
    recorder.stop();

    // assert output.wav exists
    assert!(
        std::path::Path::new("output.wav").exists(),
        "output.wav does not exist"
    );

    // delete output.wav
    // if let Err(e) = std::fs::remove_file("output.wav") {
    //     tracing::error!("Failed to delete output.wav: {}", e);
    // }
}
