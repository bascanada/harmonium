use wasm_bindgen::prelude::*;
use std::sync::{Arc, Mutex};

pub mod sequencer;
pub mod harmony;
pub mod log;
pub mod engine;
pub mod audio;
pub mod progression;
pub mod fractal;
pub mod ai;

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

    pub fn resume(&self) -> Result<(), JsValue> {
        use cpal::traits::StreamTrait;
        self.stream.play().map_err(|e| JsValue::from_str(&e.to_string()))
    }

    pub fn pause(&self) -> Result<(), JsValue> {
        use cpal::traits::StreamTrait;
        self.stream.pause().map_err(|e| JsValue::from_str(&e.to_string()))
    }
}

#[wasm_bindgen]
pub fn start() -> Result<Handle, JsValue> {
    console_error_panic_hook::set_once();

    // Créer l'état partagé pour WASM (contrôlé par l'UI, pas d'IA)
    let target_state = Arc::new(Mutex::new(engine::EngineParams::default()));
    let target_state_clone = target_state.clone();
    
    let (stream, config, harmony_state, event_queue) = audio::create_stream(target_state).map_err(|e| JsValue::from_str(&e))?;

    Ok(Handle { 
        stream,
        target_state: target_state_clone,
        harmony_state,
        event_queue,
        bpm: config.bpm,
        key: config.key,
        scale: config.scale,
        pulses: config.pulses,
        steps: config.steps,
    })
}

