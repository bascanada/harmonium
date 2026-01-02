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
            let mut last_time = 0;
            
            // Simple monophonic conversion for now
            // We assume NoteOn starts a note, and the next NoteOn or NoteOff ends it.
            // We need to fill gaps with rests.
            
            for (time, _event) in chan_events {
                let delta_samples = time - last_time;
                let delta_steps = (delta_samples as f64 / self.samples_per_step as f64).round() as u64;
                
                if delta_steps > 0 {
                    // If we were "playing" a note, this delta is the note duration.
                    // If we were "silent", this delta is a rest.
                    // But we need to know the STATE.
                    // This simple loop is insufficient without state tracking.
                }
                last_time = *time;
            }
            
            // Better approach: Reconstruct the timeline
            // 1. Create a timeline of "events" (NoteOn/NoteOff)
            // 2. Iterate through time, emitting notes/rests
            
            let mut current_note: Option<u8> = None;
            let mut last_event_time = 0;
            
            for (time, event) in chan_events {
                let duration_samples = time - last_event_time;
                let duration_steps = (duration_samples as f64 / self.samples_per_step as f64).round() as usize;
                
                if duration_steps > 0 {
                    if let Some(note) = current_note {
                        // Emit note
                        let note_str = midi_to_abc(note);
                        abc.push_str(&format!("{}{}", note_str, duration_steps));
                    } else {
                        // Emit rest
                        abc.push_str(&format!("z{}", duration_steps));
                    }
                    abc.push(' '); // Spacer
                }
                
                match event {
                    AudioEvent::NoteOn { note, velocity, .. } => {
                        if *velocity > 0 {
                            current_note = Some(*note);
                        } else {
                            current_note = None;
                        }
                    },
                    AudioEvent::NoteOff { .. } => {
                        current_note = None;
                    },
                    _ => {}
                }
                
                last_event_time = *time;
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
