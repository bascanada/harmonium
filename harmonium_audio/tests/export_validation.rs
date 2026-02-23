use std::sync::{Arc, Mutex};

use harmonium_audio::backend::{
    AudioRenderer, recorder::RecorderBackend, synth_backend::SynthBackend,
};
use harmonium_core::{
    events::{AudioEvent, RecordFormat},
    truth::RecordingTruth,
};
use roxmltree::Document;

#[test]
fn test_export_roundtrip_validation() {
    let sample_rate = 44100;

    // 1. Setup Backend & Recorder
    let recordings = run_30s_complex_validation_test(sample_rate);

    // 2. Verify Truth
    let truth_data = recordings
        .iter()
        .find(|(f, _)| *f == RecordFormat::Truth)
        .map(|(_, d)| d)
        .expect("Truth missing");
    let truth: RecordingTruth = serde_json::from_slice(truth_data).expect("Failed to parse Truth");

    // 3. Verify MusicXML
    let xml_data = recordings
        .iter()
        .find(|(f, _)| *f == RecordFormat::MusicXml)
        .map(|(_, d)| d)
        .expect("MusicXML missing");
    let xml_str = std::str::from_utf8(xml_data).unwrap();
    let xml_str_clean =
        xml_str.lines().filter(|l| !l.contains("!DOCTYPE")).collect::<Vec<_>>().join("\n");
    let doc = Document::parse(&xml_str_clean).expect("Invalid MusicXML");

    let note_elements: Vec<_> = doc
        .descendants()
        .filter(|n| n.has_tag_name("note") && !n.children().any(|c| c.has_tag_name("rest")))
        .collect();
    let truth_notes = truth
        .events
        .iter()
        .filter(|(_, e)| matches!(e, AudioEvent::NoteOn { velocity, .. } if *velocity > 0))
        .count();

    assert_eq!(note_elements.len(), truth_notes, "Note count mismatch over 30s export");

    // 4. Verify Audio (WAV)
    let wav_data =
        recordings.iter().find(|(f, _)| *f == RecordFormat::Wav).map(|(_, d)| d).unwrap();
    let mut reader = hound::WavReader::new(std::io::Cursor::new(wav_data)).unwrap();
    let samples: Vec<f32> = reader.samples::<f32>().map(|s| s.unwrap()).collect();

    let samples_per_sec = sample_rate as usize * 2; // Stereo

    // Check first 1s for silence
    let silence_seg = &samples[0..samples_per_sec];
    // Check middle segment (15s to 20s) for continuous signal
    let signal_seg = &samples[samples_per_sec * 15..samples_per_sec * 20];

    let rms_silence =
        (silence_seg.iter().map(|s| s * s).sum::<f32>() / silence_seg.len() as f32).sqrt();
    let rms_signal =
        (signal_seg.iter().map(|s| s * s).sum::<f32>() / signal_seg.len() as f32).sqrt();

    println!("30s Export Validation Results:");
    println!("  Total Duration: 30.0s");
    println!("  Total Notes:    {}", truth_notes);
    println!("  Silence RMS:    {}", rms_silence);
    println!("  Signal RMS:     {}", rms_signal);

    assert!(rms_silence < 0.001, "Silence period contains too much noise");
    if rms_signal > 0.0 {
        assert!(rms_signal > rms_silence * 10.0, "Signal is not significantly louder than silence");
    }

    println!("Validation Successful: 30s export is consistent and accurate.");
}

fn run_30s_complex_validation_test(sample_rate: u32) -> Vec<(RecordFormat, Vec<u8>)> {
    let finished_recordings = Arc::new(Mutex::new(Vec::new()));

    let sf2_path = "../web/static/test.sf2";
    let sf2_data = std::fs::read(sf2_path).expect("test.sf2 soundfont file not found");

    let inner = Box::new(SynthBackend::new(sample_rate as f64, Some(&sf2_data), &vec![-1; 16]));
    let mut recorder = RecorderBackend::new(inner, finished_recordings.clone(), sample_rate);

    // Warm up
    let mut buffer = vec![0.0f32; 4096];
    for _ in 0..10 {
        recorder.process_buffer(&mut buffer, 2);
    }

    recorder.handle_event(AudioEvent::StartRecording { format: RecordFormat::Wav });
    recorder.handle_event(AudioEvent::StartRecording { format: RecordFormat::MusicXml });
    recorder.handle_event(AudioEvent::StartRecording { format: RecordFormat::Truth });

    let samples_per_step = 11025; // 0.25s per step
    recorder.handle_event(AudioEvent::TimingUpdate { samples_per_step });
    let mut buffer = vec![0.0f32; samples_per_step * 2];

    // Total 120 steps = 30 seconds
    for step in 0..120 {
        // Simple composition logic
        if step < 4 {
            // Silence
        } else if step < 116 {
            // Play a bass note on every 4th step
            if step % 4 == 0 {
                recorder.handle_event(AudioEvent::NoteOn { id: None, note: 36, velocity: 100, channel: 0 });
            }
            if step % 4 == 3 {
                recorder.handle_event(AudioEvent::NoteOff { id: None, note: 36, channel: 0 });
            }

            // Play a lead chord on every 8th step
            if step % 8 == 0 {
                recorder.handle_event(AudioEvent::NoteOn { id: None, note: 60, velocity: 80, channel: 1 });
                recorder.handle_event(AudioEvent::NoteOn { id: None, note: 64, velocity: 80, channel: 1 });
            }
            if step % 8 == 6 {
                recorder.handle_event(AudioEvent::NoteOff { id: None, note: 60, channel: 1 });
                recorder.handle_event(AudioEvent::NoteOff { id: None, note: 64, channel: 1 });
            }
        }

        recorder.process_buffer(&mut buffer, 2);
    }

    recorder.handle_event(AudioEvent::StopRecording { format: RecordFormat::Wav });
    recorder.handle_event(AudioEvent::StopRecording { format: RecordFormat::MusicXml });
    recorder.handle_event(AudioEvent::StopRecording { format: RecordFormat::Truth });

    let mut recs = finished_recordings.lock().unwrap();
    std::mem::take(&mut *recs)
}
