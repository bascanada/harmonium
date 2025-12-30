use rust_music_theory::scale::{Scale, ScaleType, Direction};
use rust_music_theory::note::{PitchSymbol, Pitch, Notes};
use rand::Rng;

pub struct HarmonyNavigator {
    pub current_scale: Scale,
    pub current_index: i32,
    pub octave: i32,
}

impl HarmonyNavigator {
    pub fn new(root_note: PitchSymbol, scale_type: ScaleType, octave: i32) -> Self {
        let pitch = Pitch::from(root_note);
        let scale = Scale::new(scale_type, pitch, octave as u8, None, Direction::Ascending).unwrap();
        HarmonyNavigator {
            current_scale: scale,
            current_index: 0,
            octave,
        }
    }

    pub fn next_note(&mut self) -> f32 {
        let mut rng = rand::thread_rng();
        let step: i32 = rng.gen_range(-1..=1);
        
        self.current_index += step;

        // Keep index within reasonable bounds to avoid drifting too far? 
        // Or just let it walk. For musicality, maybe clamp or wrap?
        // Let's just calculate frequency based on the scale notes.
        
        // Scale in rust-music-theory gives us a list of notes.
        // If index is 0, it's the root.
        // If index > notes.len(), we go up octaves.
        // If index < 0, we go down octaves.
        
        self.get_frequency()
    }

    fn get_frequency(&self) -> f32 {
        let notes = self.current_scale.notes();
        let len = notes.len() as i32;
        
        // Calculate the actual note index and octave shift
        let mut index = self.current_index;
        let mut octave_shift = 0;

        while index < 0 {
            index += len;
            octave_shift -= 1;
        }
        while index >= len {
            index -= len;
            octave_shift += 1;
        }

        let note = &notes[index as usize];
        
        // We need to reconstruct the frequency.
        // The 'note' struct from the scale has a fixed octave usually (the one the scale was created with).
        // We need to adjust it.
        
        // rust-music-theory Note has a frequency() method? Or we calculate from PitchClass and Octave.
        // Note struct usually has `pitch_class` and `octave`.
        
        // Let's create a new note with the shifted octave to get the freq.
        // Note: The scale notes already have the base octave of the scale.
        // So we just add the octave_shift to that note's octave.
        
        // Accessing private fields might be an issue if we try to construct manually.
        // Let's see if we can use a helper or if Note is easy to clone/modify.
        
        // Assuming Note has a public way to get freq or we can use the formula.
        // f = 440 * 2^((n - 69)/12)
        // Let's rely on the crate if possible, otherwise manual calc.
        
        // Warning: rust-music-theory `Note` struct fields might be private.
        // Use `pitch_class` and `octave` getters if available.
        
        // Actually, let's just use the `freq` method if it exists, or calculate.
        // A safer bet for a POC without full docs is to calculate:
        // pitch_class to semitone index (C=0, C#=1...)
        // midi_val = (octave + 1) * 12 + semitone
        // freq = 440.0 * 2.0_f32.powf((midi_val - 69.0) / 12.0)
        
        let pc_val = note.pitch.into_u8() as i32; // Assuming PitchClass can be converted to int
        let note_octave = note.octave as i32 + octave_shift;
        
        let midi_note = (note_octave + 1) * 12 + pc_val;
        let freq = 440.0 * 2.0_f32.powf((midi_note as f32 - 69.0) / 12.0);
        
        freq
    }
}
