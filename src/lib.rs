#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

#[cfg(feature = "standalone")]
use std::sync::{Arc, Mutex};

pub mod sequencer;
pub mod harmony;
pub mod log;
pub mod engine;
pub mod fractal;
pub mod events;
pub mod backend;
pub mod voice_manager;
pub mod voicing;
pub mod params;
pub mod mapper;

#[cfg(feature = "ai")]
pub mod ai;

// Audio module (only for standalone/WASM builds with cpal)
#[cfg(feature = "standalone")]
pub mod audio;

// VST Plugin module (only for VST builds)
#[cfg(feature = "vst")]
pub mod vst_plugin;

// Re-exports pour compatibilité avec l'ancien code
pub use harmony::basic as progression;
pub use harmony::melody as harmony_melody;

pub use sequencer::RhythmMode;
pub use harmony::HarmonyMode;

// Re-exports pour la nouvelle architecture découplée
pub use params::{MusicalParams, HarmonyStrategy, ControlMode};
pub use mapper::{EmotionMapper, MapperConfig};

// Re-export VST plugin when building with vst feature
#[cfg(feature = "vst")]
pub use vst_plugin::HarmoniumPlugin;

#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub struct RecordedData {
    format_str: String,
    data: Vec<u8>,
}

#[cfg(feature = "wasm")]
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

#[cfg(not(feature = "wasm"))]
impl RecordedData {
    pub fn format(&self) -> String {
        self.format_str.clone()
    }

    pub fn data(&self) -> Vec<u8> {
        self.data.clone()
    }
}

// Handle and WASM bindings only available with standalone feature (cpal)
#[cfg(feature = "standalone")]
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub struct Handle {
    #[allow(dead_code)]
    stream: cpal::Stream,
    /// État partagé pour contrôler le moteur en temps réel (mode émotion)
    target_state: Arc<Mutex<engine::EngineParams>>,
    /// État partagé pour le mode de contrôle (émotion vs direct)
    control_mode: Arc<Mutex<params::ControlMode>>,
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

#[cfg(feature = "standalone")]
#[cfg_attr(feature = "wasm", wasm_bindgen)]
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

    /// Définir le mute d'un canal (true = Muted)
    pub fn set_channel_muted(&mut self, channel: usize, is_muted: bool) {
        if let Ok(mut state) = self.target_state.lock() {
            if channel < 16 {
                if state.muted_channels.len() <= channel {
                    state.muted_channels.resize(16, false);
                }
                state.muted_channels[channel] = is_muted;
            }
        }
    }

    // === Mixer Controls ===

    /// Set gain for lead instrument (0.0-1.0, default 1.0)
    pub fn set_gain_lead(&mut self, gain: f32) {
        if let Ok(mut state) = self.target_state.lock() {
            state.gain_lead = gain.clamp(0.0, 1.0);
        }
    }

    /// Set gain for bass instrument (0.0-1.0, default 0.6)
    pub fn set_gain_bass(&mut self, gain: f32) {
        if let Ok(mut state) = self.target_state.lock() {
            state.gain_bass = gain.clamp(0.0, 1.0);
        }
    }

    /// Set gain for snare (0.0-1.0, default 0.5)
    pub fn set_gain_snare(&mut self, gain: f32) {
        if let Ok(mut state) = self.target_state.lock() {
            state.gain_snare = gain.clamp(0.0, 1.0);
        }
    }

    /// Set gain for hi-hat (0.0-1.0, default 0.4)
    pub fn set_gain_hat(&mut self, gain: f32) {
        if let Ok(mut state) = self.target_state.lock() {
            state.gain_hat = gain.clamp(0.0, 1.0);
        }
    }

    /// Set base velocity for bass (0-127, default 85)
    pub fn set_vel_base_bass(&mut self, vel: u8) {
        if let Ok(mut state) = self.target_state.lock() {
            state.vel_base_bass = vel.min(127);
        }
    }

    /// Set base velocity for snare (0-127, default 70)
    pub fn set_vel_base_snare(&mut self, vel: u8) {
        if let Ok(mut state) = self.target_state.lock() {
            state.vel_base_snare = vel.min(127);
        }
    }

    /// Get current gain for lead
    pub fn get_gain_lead(&self) -> f32 {
        self.target_state.lock().map(|s| s.gain_lead).unwrap_or(1.0)
    }

    /// Get current gain for bass
    pub fn get_gain_bass(&self) -> f32 {
        self.target_state.lock().map(|s| s.gain_bass).unwrap_or(0.6)
    }

    /// Get current gain for snare
    pub fn get_gain_snare(&self) -> f32 {
        self.target_state.lock().map(|s| s.gain_snare).unwrap_or(0.5)
    }

    /// Get current gain for hi-hat
    pub fn get_gain_hat(&self) -> f32 {
        self.target_state.lock().map(|s| s.gain_hat).unwrap_or(0.4)
    }

    /// Get base velocity for bass
    pub fn get_vel_base_bass(&self) -> u8 {
        self.target_state.lock().map(|s| s.vel_base_bass).unwrap_or(85)
    }

    /// Get base velocity for snare
    pub fn get_vel_base_snare(&self) -> u8 {
        self.target_state.lock().map(|s| s.vel_base_snare).unwrap_or(70)
    }

    /// Set polyrythm steps (48, 96, 192...) - must be multiple of 4
    pub fn set_poly_steps(&mut self, steps: usize) {
        if let Ok(mut state) = self.target_state.lock() {
            // Ensure it's a multiple of 4 and reasonable range
            let valid_steps = (steps / 4) * 4;
            state.poly_steps = valid_steps.clamp(16, 384);
        }
    }

    /// Get current polyrythm steps
    pub fn get_poly_steps(&self) -> usize {
        self.target_state.lock().map(|s| s.poly_steps).unwrap_or(48)
    }

    /// Ajouter une SoundFont à un bank spécifique
    pub fn add_soundfont(&self, bank_id: u32, sf2_bytes: Box<[u8]>) {
        if let Ok(mut queue) = self.font_queue.lock() {
            queue.push((bank_id, sf2_bytes.into_vec()));
        }
    }

    #[cfg(feature = "wasm")]
    pub fn resume(&self) -> Result<(), JsValue> {
        use cpal::traits::StreamTrait;
        self.stream.play().map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[cfg(not(feature = "wasm"))]
    pub fn resume(&self) -> Result<(), String> {
        use cpal::traits::StreamTrait;
        self.stream.play().map_err(|e| e.to_string())
    }

    #[cfg(feature = "wasm")]
    pub fn pause(&self) -> Result<(), JsValue> {
        use cpal::traits::StreamTrait;
        self.stream.pause().map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[cfg(not(feature = "wasm"))]
    pub fn pause(&self) -> Result<(), String> {
        use cpal::traits::StreamTrait;
        self.stream.pause().map_err(|e| e.to_string())
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

    // ═══════════════════════════════════════════════════════════════════
    // MODE DE CONTRÔLE: Émotion vs Direct
    // ═══════════════════════════════════════════════════════════════════

    /// Active le mode émotionnel (sliders arousal/valence/density/tension)
    /// C'est le mode par défaut
    pub fn use_emotion_mode(&self) {
        if let Ok(mut mode) = self.control_mode.lock() {
            mode.use_emotion_mode = true;
        }
    }

    /// Active le mode technique direct (contrôle précis des paramètres musicaux)
    pub fn use_direct_mode(&self) {
        if let Ok(mut mode) = self.control_mode.lock() {
            mode.use_emotion_mode = false;
        }
    }

    /// Retourne true si le moteur est en mode émotionnel
    pub fn is_emotion_mode(&self) -> bool {
        self.control_mode.lock().map(|m| m.use_emotion_mode).unwrap_or(true)
    }

    // ═══════════════════════════════════════════════════════════════════
    // PARAMÈTRES DIRECTS (Mode Technique)
    // ═══════════════════════════════════════════════════════════════════

    /// Définit le BPM directement (mode direct uniquement)
    pub fn set_direct_bpm(&self, bpm: f32) {
        if let Ok(mut mode) = self.control_mode.lock() {
            mode.direct_params.bpm = bpm.clamp(30.0, 300.0);
        }
    }

    /// Obtient le BPM actuel en mode direct
    pub fn get_direct_bpm(&self) -> f32 {
        self.control_mode.lock().map(|m| m.direct_params.bpm).unwrap_or(120.0)
    }

    /// Active/désactive le module rythmique (global - works in both modes)
    pub fn set_direct_enable_rhythm(&self, enabled: bool) {
        if let Ok(mut mode) = self.control_mode.lock() {
            mode.enable_rhythm = enabled;
        }
    }

    /// Active/désactive le module harmonique (global - works in both modes)
    pub fn set_direct_enable_harmony(&self, enabled: bool) {
        if let Ok(mut mode) = self.control_mode.lock() {
            mode.enable_harmony = enabled;
        }
    }

    /// Active/désactive le module mélodique (global - works in both modes)
    pub fn set_direct_enable_melody(&self, enabled: bool) {
        if let Ok(mut mode) = self.control_mode.lock() {
            mode.enable_melody = enabled;
        }
    }

    /// Active/désactive le voicing (global - works in both modes)
    pub fn set_direct_enable_voicing(&self, enabled: bool) {
        if let Ok(mut mode) = self.control_mode.lock() {
            mode.enable_voicing = enabled;
        }
    }

    /// Définit le mode rythmique (0 = Euclidean, 1 = PerfectBalance)
    pub fn set_direct_rhythm_mode(&self, mode: u8) {
        if let Ok(mut m) = self.control_mode.lock() {
            m.direct_params.rhythm_mode = match mode {
                0 => RhythmMode::Euclidean,
                1 => RhythmMode::PerfectBalance,
                _ => RhythmMode::Euclidean,
            };
        }
    }

    /// Définit le nombre de steps (16, 48, 96, 192)
    pub fn set_direct_rhythm_steps(&self, steps: usize) {
        if let Ok(mut mode) = self.control_mode.lock() {
            let valid_steps = (steps / 4) * 4;
            mode.direct_params.rhythm_steps = valid_steps.clamp(16, 384);
        }
    }

    /// Définit le nombre de pulses
    pub fn set_direct_rhythm_pulses(&self, pulses: usize) {
        if let Ok(mut mode) = self.control_mode.lock() {
            mode.direct_params.rhythm_pulses = pulses.clamp(1, 32);
        }
    }

    /// Définit la rotation du pattern
    pub fn set_direct_rhythm_rotation(&self, rotation: usize) {
        if let Ok(mut mode) = self.control_mode.lock() {
            mode.direct_params.rhythm_rotation = rotation;
        }
    }

    /// Définit la densité rythmique (0.0-1.0)
    pub fn set_direct_rhythm_density(&self, density: f32) {
        if let Ok(mut mode) = self.control_mode.lock() {
            mode.direct_params.rhythm_density = density.clamp(0.0, 1.0);
        }
    }

    /// Définit la tension rythmique (0.0-1.0) - ghost notes, syncopation
    pub fn set_direct_rhythm_tension(&self, tension: f32) {
        if let Ok(mut mode) = self.control_mode.lock() {
            mode.direct_params.rhythm_tension = tension.clamp(0.0, 1.0);
        }
    }

    /// Définit les steps du séquenceur secondaire (Euclidean mode)
    pub fn set_direct_secondary_steps(&self, steps: usize) {
        if let Ok(mut mode) = self.control_mode.lock() {
            mode.direct_params.rhythm_secondary_steps = steps.clamp(4, 32);
        }
    }

    /// Définit les pulses du séquenceur secondaire (Euclidean mode)
    pub fn set_direct_secondary_pulses(&self, pulses: usize) {
        if let Ok(mut mode) = self.control_mode.lock() {
            mode.direct_params.rhythm_secondary_pulses = pulses.clamp(1, 32);
        }
    }

    /// Définit la rotation du séquenceur secondaire (Euclidean mode)
    pub fn set_direct_secondary_rotation(&self, rotation: usize) {
        if let Ok(mut mode) = self.control_mode.lock() {
            mode.direct_params.rhythm_secondary_rotation = rotation;
        }
    }

    /// Définit le mode harmonique (0 = Basic, 1 = Driver)
    pub fn set_direct_harmony_mode(&self, mode: u8) {
        if let Ok(mut m) = self.control_mode.lock() {
            m.direct_params.harmony_mode = match mode {
                0 => HarmonyMode::Basic,
                1 => HarmonyMode::Driver,
                _ => HarmonyMode::Driver,
            };
        }
    }

    /// Définit la tension harmonique (0.0-1.0)
    pub fn set_direct_harmony_tension(&self, tension: f32) {
        if let Ok(mut mode) = self.control_mode.lock() {
            mode.direct_params.harmony_tension = tension.clamp(0.0, 1.0);
        }
    }

    /// Définit la valence harmonique (-1.0 à 1.0)
    pub fn set_direct_harmony_valence(&self, valence: f32) {
        if let Ok(mut mode) = self.control_mode.lock() {
            mode.direct_params.harmony_valence = valence.clamp(-1.0, 1.0);
        }
    }

    /// Définit le lissage mélodique (0.0-1.0)
    pub fn set_direct_melody_smoothness(&self, smoothness: f32) {
        if let Ok(mut mode) = self.control_mode.lock() {
            mode.direct_params.melody_smoothness = smoothness.clamp(0.0, 1.0);
        }
    }

    /// Définit la densité de voicing (0.0-1.0)
    pub fn set_direct_voicing_density(&self, density: f32) {
        if let Ok(mut mode) = self.control_mode.lock() {
            mode.direct_params.voicing_density = density.clamp(0.0, 1.0);
        }
    }

    /// Définit la tension de voicing (0.0-1.0) - contrôle le filtre/timbre
    pub fn set_direct_voicing_tension(&self, tension: f32) {
        if let Ok(mut mode) = self.control_mode.lock() {
            mode.direct_params.voicing_tension = tension.clamp(0.0, 1.0);
        }
    }

    /// Obtient l'état actuel du mode et des paramètres directs (JSON)
    pub fn get_direct_params_json(&self) -> String {
        if let Ok(mode) = self.control_mode.lock() {
            serde_json::to_string(&mode.direct_params).unwrap_or_else(|_| "{}".to_string())
        } else {
            "{}".to_string()
        }
    }

    /// Définit tous les paramètres directs depuis un JSON
    pub fn set_direct_params_json(&self, json: &str) {
        if let Ok(params) = serde_json::from_str::<MusicalParams>(json) {
            if let Ok(mut mode) = self.control_mode.lock() {
                mode.direct_params = params;
            }
        }
    }

    // === Getters pour l'UI en mode direct ===

    pub fn get_direct_enable_rhythm(&self) -> bool {
        self.control_mode.lock().map(|m| m.enable_rhythm).unwrap_or(true)
    }

    pub fn get_direct_enable_harmony(&self) -> bool {
        self.control_mode.lock().map(|m| m.enable_harmony).unwrap_or(true)
    }

    pub fn get_direct_enable_melody(&self) -> bool {
        self.control_mode.lock().map(|m| m.enable_melody).unwrap_or(true)
    }

    pub fn get_direct_enable_voicing(&self) -> bool {
        self.control_mode.lock().map(|m| m.enable_voicing).unwrap_or(true)
    }

    /// Retourne le mode rythmique (0 = Euclidean, 1 = PerfectBalance)
    pub fn get_direct_rhythm_mode(&self) -> u8 {
        self.control_mode.lock().map(|m| {
            match m.direct_params.rhythm_mode {
                RhythmMode::Euclidean => 0,
                RhythmMode::PerfectBalance => 1,
            }
        }).unwrap_or(0)
    }

    pub fn get_direct_rhythm_steps(&self) -> usize {
        self.control_mode.lock().map(|m| m.direct_params.rhythm_steps).unwrap_or(16)
    }

    pub fn get_direct_rhythm_pulses(&self) -> usize {
        self.control_mode.lock().map(|m| m.direct_params.rhythm_pulses).unwrap_or(4)
    }

    pub fn get_direct_rhythm_rotation(&self) -> usize {
        self.control_mode.lock().map(|m| m.direct_params.rhythm_rotation).unwrap_or(0)
    }

    pub fn get_direct_rhythm_density(&self) -> f32 {
        self.control_mode.lock().map(|m| m.direct_params.rhythm_density).unwrap_or(0.5)
    }

    pub fn get_direct_rhythm_tension(&self) -> f32 {
        self.control_mode.lock().map(|m| m.direct_params.rhythm_tension).unwrap_or(0.3)
    }

    pub fn get_direct_secondary_steps(&self) -> usize {
        self.control_mode.lock().map(|m| m.direct_params.rhythm_secondary_steps).unwrap_or(12)
    }

    pub fn get_direct_secondary_pulses(&self) -> usize {
        self.control_mode.lock().map(|m| m.direct_params.rhythm_secondary_pulses).unwrap_or(3)
    }

    pub fn get_direct_secondary_rotation(&self) -> usize {
        self.control_mode.lock().map(|m| m.direct_params.rhythm_secondary_rotation).unwrap_or(0)
    }

    pub fn get_direct_harmony_tension(&self) -> f32 {
        self.control_mode.lock().map(|m| m.direct_params.harmony_tension).unwrap_or(0.3)
    }

    pub fn get_direct_harmony_valence(&self) -> f32 {
        self.control_mode.lock().map(|m| m.direct_params.harmony_valence).unwrap_or(0.3)
    }

    pub fn get_direct_melody_smoothness(&self) -> f32 {
        self.control_mode.lock().map(|m| m.direct_params.melody_smoothness).unwrap_or(0.7)
    }

    pub fn get_direct_voicing_density(&self) -> f32 {
        self.control_mode.lock().map(|m| m.direct_params.voicing_density).unwrap_or(0.5)
    }

    pub fn get_direct_voicing_tension(&self) -> f32 {
        self.control_mode.lock().map(|m| m.direct_params.voicing_tension).unwrap_or(0.3)
    }
}

#[cfg(all(feature = "standalone", feature = "wasm"))]
#[wasm_bindgen]
pub fn start(sf2_bytes: Option<Box<[u8]>>) -> Result<Handle, JsValue> {
    console_error_panic_hook::set_once();

    // Créer les états partagés pour WASM
    let target_state = Arc::new(Mutex::new(engine::EngineParams::default()));
    let control_mode = Arc::new(Mutex::new(params::ControlMode::default()));

    let target_state_clone = target_state.clone();
    let control_mode_clone = control_mode.clone();

    let (stream, config, harmony_state, event_queue, font_queue, finished_recordings) =
        audio::create_stream(target_state, control_mode, sf2_bytes.as_deref())
            .map_err(|e| JsValue::from_str(&e))?;

    Ok(Handle {
        stream,
        target_state: target_state_clone,
        control_mode: control_mode_clone,
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

