use wasm_bindgen::prelude::*;
use std::sync::{Arc, Mutex};

pub mod sequencer;
pub mod harmony;
pub mod log;
pub mod engine;
pub mod audio;

#[wasm_bindgen]
pub struct Handle {
    #[allow(dead_code)]
    stream: cpal::Stream,
    /// État partagé pour contrôler le moteur en temps réel
    target_state: Arc<Mutex<engine::EngineParams>>,
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
}

#[wasm_bindgen]
pub fn start() -> Result<Handle, JsValue> {
    console_error_panic_hook::set_once();

    // Créer l'état partagé pour WASM (contrôlé par l'UI, pas d'IA)
    let target_state = Arc::new(Mutex::new(engine::EngineParams::default()));
    let target_state_clone = target_state.clone();
    
    let (stream, config) = audio::create_stream(target_state).map_err(|e| JsValue::from_str(&e))?;

    Ok(Handle { 
        stream,
        target_state: target_state_clone,
        bpm: config.bpm,
        key: config.key,
        scale: config.scale,
        pulses: config.pulses,
        steps: config.steps,
    })
}

