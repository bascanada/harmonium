use wasm_bindgen::prelude::*;
use std::sync::{Arc, Mutex};

pub mod sequencer;
pub mod harmony;
pub mod log;
pub mod engine;
pub mod audio;
pub mod fractal;
pub mod ai;
pub mod events;
pub mod backend;
pub mod voice_manager;
pub mod voicing;

// Re-exports pour compatibilité avec l'ancien code
pub use harmony::basic as progression;
pub use harmony::melody as harmony_melody;

pub use sequencer::RhythmMode;
pub use harmony::HarmonyMode;

#[wasm_bindgen]
pub struct RecordedData {
    format_str: String,
    data: Vec<u8>,
}

#[wasm_bindgen]
impl RecordedData {
    #[wasm_bindgen(getter)]
    pub fn format(&self) -> String {
        self.format_str.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn data(&self) -> Vec<u8> {
        self.data.clone()
    }
}

#[wasm_bindgen]
pub struct Handle {
    #[allow(dead_code)]
    stream: cpal::Stream,
    /// État partagé pour contrôler le moteur en temps réel
    target_state: Arc<Mutex<engine::EngineParams>>,
    /// État harmonique en lecture seule pour l'UI
    harmony_state: Arc<Mutex<engine::HarmonyState>>,
    /// Queue d'événements pour l'UI
    event_queue: Arc<Mutex<Vec<engine::VisualizationEvent>>>,
    /// Queue de chargement de SoundFonts
    font_queue: Arc<Mutex<Vec<(u32, Vec<u8>)>>>,
    /// Enregistrements terminés
    finished_recordings: Arc<Mutex<Vec<(events::RecordFormat, Vec<u8>)>>>,
    bpm: f32,
    key: String,
    scale: String,
    pulses: usize,
    steps: usize,
}

#[wasm_bindgen]
impl Handle {
    pub fn get_bpm(&self) -> f32 {
        self.bpm
    }

    pub fn get_key(&self) -> String {
        self.key.clone()
    }

    pub fn get_scale(&self) -> String {
        self.scale.clone()
    }

    pub fn get_pulses(&self) -> usize {
        self.pulses
    }

    pub fn get_steps(&self) -> usize {
        self.steps
    }

    // === Contrôles en Temps Réel pour l'UI (Modèle Émotionnel) ===

    /// Définir l'arousal (activation/énergie) qui contrôle le BPM (0.0 à 1.0)
    /// Low (0.0) = 70 BPM, High (1.0) = 180 BPM
    pub fn set_arousal(&mut self, arousal: f32) {
        if let Ok(mut state) = self.target_state.lock() {
            state.arousal = arousal.clamp(0.0, 1.0);
        }
    }

    /// Définir la valence (positif/négatif) pour l'harmonie (-1.0 à 1.0)
    /// Negative = Minor/Sad, Positive = Major/Happy
    pub fn set_valence(&mut self, valence: f32) {
        if let Ok(mut state) = self.target_state.lock() {
            state.valence = valence.clamp(-1.0, 1.0);
        }
    }

    /// Définir la densité rythmique (0.0 = calme, 1.0 = dense)
    pub fn set_density(&mut self, density: f32) {
        if let Ok(mut state) = self.target_state.lock() {
            state.density = density.clamp(0.0, 1.0);
        }
    }

    /// Définir la tension harmonique (0.0 = consonant, 1.0 = dissonant)
    pub fn set_tension(&mut self, tension: f32) {
        if let Ok(mut state) = self.target_state.lock() {
            state.tension = tension.clamp(0.0, 1.0);
        }
    }

    /// Définir l'algorithme rythmique (0 = Euclidean, 1 = PerfectBalance)
    /// PerfectBalance active le mode 48 steps pour les polyrythmes parfaits 4:3
    pub fn set_algorithm(&mut self, algorithm: u8) {
        if let Ok(mut state) = self.target_state.lock() {
            state.algorithm = match algorithm {
                0 => RhythmMode::Euclidean,
                1 => RhythmMode::PerfectBalance,
                _ => RhythmMode::Euclidean, // Fallback
            };
        }
    }

    /// Obtenir l'algorithme rythmique actuel (0 = Euclidean, 1 = PerfectBalance)
    pub fn get_algorithm(&self) -> u8 {
        self.target_state.lock().map(|s| match s.algorithm {
            RhythmMode::Euclidean => 0,
            RhythmMode::PerfectBalance => 1,
        }).unwrap_or(0)
    }

    /// Définir le mode d'harmonie (0 = Basic, 1 = Driver)
    /// Basic: Russell Circumplex quadrants (I-IV-vi-V progressions)
    /// Driver: Steedman Grammar + Neo-Riemannian PLR + LCC
    pub fn set_harmony_mode(&mut self, mode: u8) {
        if let Ok(mut state) = self.target_state.lock() {
            state.harmony_mode = match mode {
                0 => HarmonyMode::Basic,
                1 => HarmonyMode::Driver,
                _ => HarmonyMode::Driver, // Fallback
            };
        }
    }

    /// Obtenir le mode d'harmonie actuel depuis l'état du moteur (0 = Basic, 1 = Driver)
    pub fn get_harmony_mode(&self) -> u8 {
        self.harmony_state.lock().map(|s| match s.harmony_mode {
            HarmonyMode::Basic => 0,
            HarmonyMode::Driver => 1,
        }).unwrap_or(1)
    }

    // === Getters pour l'état actuel ===

    /// Obtenir l'arousal cible actuel
    pub fn get_target_arousal(&self) -> f32 {
        self.target_state.lock().map(|s| s.arousal).unwrap_or(0.5)
    }

    /// Obtenir la valence cible actuelle
    pub fn get_target_valence(&self) -> f32 {
        self.target_state.lock().map(|s| s.valence).unwrap_or(0.0)
    }

    /// Obtenir la densité cible actuelle
    pub fn get_target_density(&self) -> f32 {
        self.target_state.lock().map(|s| s.density).unwrap_or(0.5)
    }

    /// Obtenir la tension cible actuelle
    pub fn get_target_tension(&self) -> f32 {
        self.target_state.lock().map(|s| s.tension).unwrap_or(0.5)
    }

    /// Obtenir le BPM calculé depuis l'arousal
    pub fn get_computed_bpm(&self) -> f32 {
        self.target_state.lock().map(|s| s.compute_bpm()).unwrap_or(125.0)
    }

    /// Définir tous les paramètres émotionnels en une fois
    pub fn set_params(&mut self, arousal: f32, valence: f32, density: f32, tension: f32) {
        if let Ok(mut state) = self.target_state.lock() {
            state.arousal = arousal.clamp(0.0, 1.0);
            state.valence = valence.clamp(-1.0, 1.0);
            state.density = density.clamp(0.0, 1.0);
            state.tension = tension.clamp(0.0, 1.0);
        }
    }

    // === Getters pour l'état harmonique (UI Display) ===

    /// Obtenir le nom de l'accord courant ("I", "vi", "IV", "V")
    pub fn get_current_chord_name(&self) -> String {
        self.harmony_state.lock().map(|s| s.chord_name.clone()).unwrap_or("?".to_string())
    }

    /// Obtenir l'index de l'accord courant (0-3)
    pub fn get_current_chord_index(&self) -> usize {
        self.harmony_state.lock().map(|s| s.current_chord_index).unwrap_or(0)
    }

    /// Obtenir si l'accord courant est mineur
    pub fn is_current_chord_minor(&self) -> bool {
        self.harmony_state.lock().map(|s| s.chord_is_minor).unwrap_or(false)
    }

    /// Obtenir le numéro de mesure courant
    pub fn get_current_measure(&self) -> usize {
        self.harmony_state.lock().map(|s| s.measure_number).unwrap_or(1)
    }

    /// Obtenir le numéro de cycle courant
    pub fn get_current_cycle(&self) -> usize {
        self.harmony_state.lock().map(|s| s.cycle_number).unwrap_or(1)
    }

    /// Obtenir le step courant dans la mesure (0-15)
    pub fn get_current_step(&self) -> usize {
        self.harmony_state.lock().map(|s| s.current_step).unwrap_or(0)
    }

    /// Obtenir le nom de la progression harmonique active
    pub fn get_progression_name(&self) -> String {
        self.harmony_state.lock().map(|s| s.progression_name.clone()).unwrap_or("?".to_string())
    }

    /// Obtenir la longueur de la progression active (nombre d'accords)
    pub fn get_progression_length(&self) -> usize {
        self.harmony_state.lock().map(|s| s.progression_length).unwrap_or(4)
    }

    // === Getters pour la Visualisation Rythmique ===

    pub fn get_primary_pulses(&self) -> usize {
        self.harmony_state.lock().map(|s| s.primary_pulses).unwrap_or(4)
    }

    pub fn get_secondary_pulses(&self) -> usize {
        self.harmony_state.lock().map(|s| s.secondary_pulses).unwrap_or(3)
    }

    pub fn get_primary_rotation(&self) -> usize {
        self.harmony_state.lock().map(|s| s.primary_rotation).unwrap_or(0)
    }

    pub fn get_secondary_rotation(&self) -> usize {
        self.harmony_state.lock().map(|s| s.secondary_rotation).unwrap_or(0)
    }

    pub fn get_primary_steps(&self) -> usize {
        self.harmony_state.lock().map(|s| s.primary_steps).unwrap_or(16)
    }

    pub fn get_secondary_steps(&self) -> usize {
        self.harmony_state.lock().map(|s| s.secondary_steps).unwrap_or(12)
    }

    /// Récupérer le pattern primaire (Vec<bool> converti en Vec<u8> pour WASM)
    /// 1 = pulse actif, 0 = silence
    pub fn get_primary_pattern(&self) -> Vec<u8> {
        self.harmony_state.lock()
            .map(|s| s.primary_pattern.iter().map(|&b| if b { 1 } else { 0 }).collect())
            .unwrap_or_else(|_| vec![0; 16])
    }

    /// Récupérer le pattern secondaire
    pub fn get_secondary_pattern(&self) -> Vec<u8> {
        self.harmony_state.lock()
            .map(|s| s.secondary_pattern.iter().map(|&b| if b { 1 } else { 0 }).collect())
            .unwrap_or_else(|_| vec![0; 12])
    }

    /// Récupérer les événements de visualisation (Note On)
    /// Retourne un tableau plat [note, instr, step, dur, note, instr, step, dur, ...]
    pub fn get_events(&self) -> Vec<u32> {
        let mut result = Vec::new();
        if let Ok(mut queue) = self.event_queue.lock() {
            for event in queue.drain(..) {
                result.push(event.note_midi as u32);
                result.push(event.instrument as u32);
                result.push(event.step as u32);
                result.push(event.duration_samples as u32);
            }
        }
        result
    }

    /// Définir le routage d'un canal (-1 = FundSP, >=0 = Bank ID)
    pub fn set_channel_routing(&mut self, channel: usize, mode: i32) {
        if let Ok(mut state) = self.target_state.lock() {
            if channel < 16 {
                if state.channel_routing.len() <= channel {
                    state.channel_routing.resize(16, -1);
                }
                state.channel_routing[channel] = mode;
            }
        }
    }

    /// Ajouter une SoundFont à un bank spécifique
    pub fn add_soundfont(&self, bank_id: u32, sf2_bytes: Box<[u8]>) {
        if let Ok(mut queue) = self.font_queue.lock() {
            queue.push((bank_id, sf2_bytes.into_vec()));
        }
    }

    pub fn resume(&self) -> Result<(), JsValue> {
        use cpal::traits::StreamTrait;
        self.stream.play().map_err(|e| JsValue::from_str(&e.to_string()))
    }

    pub fn pause(&self) -> Result<(), JsValue> {
        use cpal::traits::StreamTrait;
        self.stream.pause().map_err(|e| JsValue::from_str(&e.to_string()))
    }

    // === Recording ===

    pub fn start_recording_wav(&self) {
        if let Ok(mut state) = self.target_state.lock() {
            state.record_wav = true;
        }
    }

    pub fn stop_recording_wav(&self) {
        if let Ok(mut state) = self.target_state.lock() {
            state.record_wav = false;
        }
    }

    pub fn start_recording_midi(&self) {
        if let Ok(mut state) = self.target_state.lock() {
            state.record_midi = true;
        }
    }

    pub fn stop_recording_midi(&self) {
        if let Ok(mut state) = self.target_state.lock() {
            state.record_midi = false;
        }
    }
    
    pub fn start_recording_abc(&self) {
        if let Ok(mut state) = self.target_state.lock() {
            state.record_abc = true;
        }
    }

    pub fn stop_recording_abc(&self) {
        if let Ok(mut state) = self.target_state.lock() {
            state.record_abc = false;
        }
    }

    /// Récupère le dernier enregistrement terminé (WAV ou MIDI)
    pub fn pop_finished_recording(&self) -> Option<RecordedData> {
        if let Ok(mut queue) = self.finished_recordings.lock() {
            if let Some((fmt, data)) = queue.pop() {
                let format_str = match fmt {
                    events::RecordFormat::Wav => "wav".to_string(),
                    events::RecordFormat::Midi => "midi".to_string(),
                    events::RecordFormat::Abc => "abc".to_string(),
                };
                return Some(RecordedData {
                    format_str,
                    data,
                });
            }
        }
        None
    }
}

#[wasm_bindgen]
pub fn start(sf2_bytes: Option<Box<[u8]>>) -> Result<Handle, JsValue> {
    console_error_panic_hook::set_once();

    // Créer l'état partagé pour WASM (contrôlé par l'UI, pas d'IA)
    let target_state = Arc::new(Mutex::new(engine::EngineParams::default()));
    let target_state_clone = target_state.clone();
    
    let (stream, config, harmony_state, event_queue, font_queue, finished_recordings) = audio::create_stream(target_state, sf2_bytes.as_deref()).map_err(|e| JsValue::from_str(&e))?;

    Ok(Handle { 
        stream,
        target_state: target_state_clone,
        harmony_state,
        event_queue,
        font_queue,
        finished_recordings,
        bpm: config.bpm,
        key: config.key,
        scale: config.scale,
        pulses: config.pulses,
        steps: config.steps,
    })
}

