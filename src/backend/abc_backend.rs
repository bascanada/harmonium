use crate::events::AudioEvent;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

pub struct AbcBackend {
    events: Arc<Mutex<Vec<(u64, AudioEvent)>>>,
    samples_elapsed: u64,
    samples_per_step: usize,
}

impl AbcBackend {
    pub fn new(_sample_rate: u32) -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
            samples_elapsed: 0,
            samples_per_step: 11025, // Default
        }
    }

    pub fn finalize(&self) -> Vec<u8> {
        let events = self.events.lock().unwrap();
        let mut abc = String::new();
        
        abc.push_str("X:1\n");
        abc.push_str("T:Harmonium Generated\n");
        abc.push_str("M:4/4\n");
        abc.push_str("L:1/16\n"); // 1 step = 1/16th
        abc.push_str("Q:1/4=120\n"); // Default BPM, could be dynamic
        abc.push_str("K:C\n");
        
        // Group by channel
        let mut channels: HashMap<u8, Vec<(u64, AudioEvent)>> = HashMap::new();
        for (time, event) in events.iter() {
            if let AudioEvent::NoteOn { channel, .. } | AudioEvent::NoteOff { channel, .. } = event {
                channels.entry(*channel).or_default().push((*time, event.clone()));
            }
        }
        
        // Process each channel as a Voice
        let mut voice_id = 1;
        let mut sorted_channels: Vec<_> = channels.keys().cloned().collect();
        
        // Custom sort: Lead (1) first, then Bass (0), then others
        sorted_channels.sort_by(|a, b| {
            let rank = |c| match c {
                1 => 0, // Lead
                0 => 1, // Bass
                _ => c + 2,
            };
            rank(*a).cmp(&rank(*b))
        });

        for channel in sorted_channels {
            let (channel_name, clef) = match channel {
                0 => ("Bass", "clef=bass"),
                1 => ("Lead", "clef=treble"),
                2 => ("Snare", "clef=perc"),
                3 => ("Hat", "clef=perc"),
                _ => ("Unknown", "clef=treble"),
            };

            abc.push_str(&format!("V:{} {} name=\"{}\"\n", voice_id, clef, channel_name));

            let chan_events = channels.get(&channel).unwrap();

            // Group events by timestamp to detect simultaneous notes (chords)
            let mut events_by_time: HashMap<u64, Vec<AudioEvent>> = HashMap::new();
            for (time, event) in chan_events {
                events_by_time.entry(*time).or_default().push(event.clone());
            }

            // Sort timestamps
            let mut timestamps: Vec<u64> = events_by_time.keys().cloned().collect();
            timestamps.sort();

            // Process events in order, handling chords (multiple notes at same time)
            let mut current_notes: Vec<u8> = Vec::new();
            let mut last_event_time = 0u64;

            for time in timestamps {
                let duration_samples = time - last_event_time;
                let duration_steps = (duration_samples as f64 / self.samples_per_step as f64).round() as usize;

                // Emit previous state (notes or rest)
                if duration_steps > 0 {
                    if current_notes.is_empty() {
                        // Emit rest
                        abc.push_str(&format!("z{}", duration_steps));
                    } else if current_notes.len() == 1 {
                        // Single note
                        let note_str = midi_to_abc(current_notes[0]);
                        abc.push_str(&format!("{}{}", note_str, duration_steps));
                    } else {
                        // Chord: [CEG]4
                        let mut chord_notes: Vec<String> = current_notes.iter()
                            .map(|&n| midi_to_abc(n))
                            .collect();
                        chord_notes.sort(); // Sort for consistent output
                        abc.push_str(&format!("[{}]{}", chord_notes.join(""), duration_steps));
                    }
                    abc.push(' '); // Spacer
                }

                // Process events at this timestamp
                if let Some(events_at_time) = events_by_time.get(&time) {
                    for event in events_at_time {
                        match event {
                            AudioEvent::NoteOn { note, velocity, .. } => {
                                if *velocity > 0 {
                                    // Add note to current chord
                                    if !current_notes.contains(note) {
                                        current_notes.push(*note);
                                    }
                                } else {
                                    // Velocity 0 = NoteOff
                                    current_notes.retain(|&n| n != *note);
                                }
                            },
                            AudioEvent::NoteOff { note, .. } => {
                                current_notes.retain(|&n| n != *note);
                            },
                            _ => {}
                        }
                    }
                }

                last_event_time = time;
            }

            abc.push_str("\n");
            voice_id += 1;
        }
        
        abc.into_bytes()
    }
    
    pub fn handle_event(&mut self, event: AudioEvent) {
        match event {
            AudioEvent::TimingUpdate { samples_per_step } => {
                self.samples_per_step = samples_per_step;
            },
            AudioEvent::NoteOn { .. } | AudioEvent::NoteOff { .. } => {
                let mut events = self.events.lock().unwrap();
                events.push((self.samples_elapsed, event));
            },
            _ => {}
        }
    }

    pub fn process_buffer(&mut self, output: &[f32], channels: usize) {
        self.samples_elapsed += (output.len() / channels) as u64;
    }
}

fn midi_to_abc(midi: u8) -> String {
    let notes = ["C", "^C", "D", "^D", "E", "F", "^F", "G", "^G", "A", "^A", "B"];
    let octave = (midi / 12) as i32 - 1;
    let note_idx = (midi % 12) as usize;
    let note_name = notes[note_idx];
    
    // ABC Octave notation:
    // C, = C2
    // C  = C3 (Middle C is C4 in MIDI? No, C4 is 60)
    // Wait, ABC standard:
    // C, = C3 (MIDI 48)
    // C  = C4 (MIDI 60)
    // c  = C5 (MIDI 72)
    // c' = C6 (MIDI 84)
    
    // Let's adjust.
    // MIDI 60 (C4) -> "C"
    // MIDI 48 (C3) -> "C,"
    // MIDI 72 (C5) -> "c"
    
    let base_note = note_name.to_string();
    
    if octave < 4 {
        // Lower octaves: C, C,,
        let commas = 4 - octave;
        format!("{}{}", base_note, ",".repeat(commas as usize))
    } else if octave == 4 {
        base_note
    } else {
        // Higher octaves: c, c', c''
        // Note: in ABC, 'c' is C5. 'C' is C4.
        // So if octave >= 5, we use lowercase.
        let lower_base = base_note.to_lowercase();
        let apostrophes = octave - 5;
        if apostrophes < 0 {
            lower_base // Just c (C5)
        } else {
            format!("{}{}", lower_base, "'".repeat(apostrophes as usize))
        }
    }
}
