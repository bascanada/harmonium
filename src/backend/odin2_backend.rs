//! Odin2 Audio Backend
//!
//! Uses odin2-core as the synthesis engine instead of fundsp/oxisynth.
//! Implements 4 dedicated instrument channels like SynthBackend:
//! - Channel 0: Bass (sine + saw through lowpass)
//! - Channel 1: Lead (supersaw with filter envelope)
//! - Channel 2: Snare (noise bandpass + tone)
//! - Channel 3: Hat (noise highpass)

use crate::backend::AudioRenderer;
use crate::events::AudioEvent;
use crate::synthesis::{EmotionalMorpher, EmotionalPresetBank, SynthPreset, apply_tension_density_modulation};

use odin2_core::dsp::envelopes::{Adsr, Envelope, EnvelopeState};
use odin2_core::dsp::filters::{Filter, LadderFilter, LadderFilterType};
use odin2_core::dsp::oscillators::{AnalogOscillator, MultiOscillator, NoiseOscillator, Oscillator, Waveform};
use odin2_core::dsp::effects::{Chorus, Delay, ZitaReverb};
use odin2_core::dsp::midi_to_freq;

#[allow(dead_code)]
const NUM_CHANNELS: usize = 16;

/// Instrument type for channel routing
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InstrumentType {
    Bass,
    Lead,
    Snare,
    Hat,
    /// Generic polyphonic synth for channels > 3
    Poly,
}

impl InstrumentType {
    fn from_channel(channel: u8) -> Self {
        match channel {
            0 => InstrumentType::Bass,
            1 => InstrumentType::Lead,
            2 => InstrumentType::Snare,
            3 => InstrumentType::Hat,
            _ => InstrumentType::Poly,
        }
    }
}

// ============================================================================
// BASS VOICE (Channel 0)
// ============================================================================

struct BassVoice {
    osc_sine: AnalogOscillator,
    osc_saw: AnalogOscillator,
    filter: LadderFilter,
    env: Adsr,
    velocity: f32,
    active: bool,
}

impl BassVoice {
    fn new(sample_rate: f32) -> Self {
        let mut osc_sine = AnalogOscillator::new(sample_rate);
        osc_sine.set_waveform(Waveform::Sine);

        let mut osc_saw = AnalogOscillator::new(sample_rate);
        osc_saw.set_waveform(Waveform::Saw);

        let mut filter = LadderFilter::new(sample_rate);
        filter.set_filter_type(LadderFilterType::LP4);
        filter.set_cutoff(800.0);
        filter.set_resonance(0.3);

        let mut env = Adsr::new(sample_rate);
        env.set_attack(0.005);
        env.set_decay(0.1);
        env.set_sustain(0.6);
        env.set_release(0.1);

        Self {
            osc_sine,
            osc_saw,
            filter,
            env,
            velocity: 0.0,
            active: false,
        }
    }

    fn note_on(&mut self, note: u8, velocity: u8) {
        let freq = midi_to_freq(note);
        self.osc_sine.set_frequency(freq);
        self.osc_saw.set_frequency(freq);
        self.velocity = velocity as f32 / 127.0;
        self.active = true;
        self.env.trigger();
    }

    fn note_off(&mut self) {
        self.env.release();
    }

    fn process(&mut self) -> (f32, f32) {
        if !self.active {
            return (0.0, 0.0);
        }

        let sine = self.osc_sine.process();
        let saw = self.osc_saw.process();
        let mix = sine * 0.7 + saw * 0.3;

        let filtered = self.filter.process(mix);
        let amp = self.env.process();

        if self.env.state() == EnvelopeState::Idle {
            self.active = false;
        }

        let out = filtered * amp * self.velocity * 0.6;
        (out, out) // Mono centered
    }
}

// ============================================================================
// LEAD VOICE (Channel 1)
// ============================================================================

struct LeadVoice {
    osc: MultiOscillator,
    filter: LadderFilter,
    amp_env: Adsr,
    filter_env: Adsr,
    base_cutoff: f32,
    velocity: f32,
    active: bool,
}

impl LeadVoice {
    fn new(sample_rate: f32) -> Self {
        let mut osc = MultiOscillator::new(sample_rate);
        osc.set_detune(0.2);
        osc.set_stereo_width(0.8);

        let mut filter = LadderFilter::new(sample_rate);
        filter.set_filter_type(LadderFilterType::LP4);
        filter.set_cutoff(2000.0);
        filter.set_resonance(0.4);

        let mut amp_env = Adsr::new(sample_rate);
        amp_env.set_attack(0.005);
        amp_env.set_decay(0.2);
        amp_env.set_sustain(0.5);
        amp_env.set_release(0.15);

        let mut filter_env = Adsr::new(sample_rate);
        filter_env.set_attack(0.01);
        filter_env.set_decay(0.3);
        filter_env.set_sustain(0.2);
        filter_env.set_release(0.2);

        Self {
            osc,
            filter,
            amp_env,
            filter_env,
            base_cutoff: 1000.0,
            velocity: 0.0,
            active: false,
        }
    }

    fn note_on(&mut self, note: u8, velocity: u8) {
        let freq = midi_to_freq(note);
        self.osc.set_frequency(freq);
        self.velocity = velocity as f32 / 127.0;
        self.active = true;
        self.amp_env.trigger();
        self.filter_env.trigger();
    }

    fn note_off(&mut self) {
        self.amp_env.release();
        self.filter_env.release();
    }

    fn process(&mut self) -> (f32, f32) {
        if !self.active {
            return (0.0, 0.0);
        }

        let (osc_l, osc_r) = self.osc.process_stereo();

        // Filter envelope modulation
        let filter_mod = self.filter_env.process();
        let cutoff = self.base_cutoff + filter_mod * 4000.0;
        self.filter.set_cutoff(cutoff.min(16000.0));

        let filtered_l = self.filter.process(osc_l);
        let filtered_r = self.filter.process(osc_r);

        let amp = self.amp_env.process();

        if self.amp_env.state() == EnvelopeState::Idle {
            self.active = false;
        }

        let gain = amp * self.velocity;
        // Pan slightly right
        (filtered_l * gain * 0.7, filtered_r * gain)
    }
}

// ============================================================================
// SNARE VOICE (Channel 2)
// ============================================================================

struct SnareVoice {
    noise: NoiseOscillator,
    tone: AnalogOscillator,
    env: Adsr,
    velocity: f32,
    active: bool,
}

impl SnareVoice {
    fn new(sample_rate: f32) -> Self {
        let mut noise = NoiseOscillator::new(sample_rate);
        // Bandpass-ish: LP at 4000, HP at 500
        noise.set_lp_freq(4000.0);
        noise.set_hp_freq(500.0);

        let mut tone = AnalogOscillator::new(sample_rate);
        tone.set_waveform(Waveform::Sine);
        tone.set_frequency(180.0);

        let mut env = Adsr::new(sample_rate);
        env.set_attack(0.001);
        env.set_decay(0.1);
        env.set_sustain(0.0);
        env.set_release(0.1);

        Self {
            noise,
            tone,
            env,
            velocity: 0.0,
            active: false,
        }
    }

    fn trigger(&mut self, velocity: u8) {
        self.velocity = velocity as f32 / 127.0;
        self.active = true;
        self.env.trigger();
    }

    fn process(&mut self) -> (f32, f32) {
        if !self.active {
            return (0.0, 0.0);
        }

        let noise_out = self.noise.process();
        let tone_out = self.tone.process();
        let mix = noise_out * 0.8 + tone_out * 0.2;

        let amp = self.env.process();

        if self.env.state() == EnvelopeState::Idle {
            self.active = false;
        }

        let out = mix * amp * self.velocity * 0.5;
        // Pan slightly left
        (out * 1.2, out * 0.8)
    }
}

// ============================================================================
// HAT VOICE (Channel 3)
// ============================================================================

struct HatVoice {
    noise: NoiseOscillator,
    env: Adsr,
    velocity: f32,
    active: bool,

    /// Contrôle dynamique du filtre (brillance)
    current_cutoff: f32,
    /// Modulation du decay (Closed vs Open)
    decay_mod: f32,
}

impl HatVoice {
    fn new(sample_rate: f32) -> Self {
        let mut noise = NoiseOscillator::new(sample_rate);
        // Élargir la plage pour laisser le contrôle au code
        // Était 4000.0 (trop restrictif), maintenant 1000.0 pour plus de flexibilité
        noise.set_hp_freq(1000.0);
        noise.set_lp_freq(16000.0);

        let mut env = Adsr::new(sample_rate);
        env.set_attack(0.001);
        env.set_decay(0.08);
        env.set_sustain(0.0);
        env.set_release(0.08);

        Self {
            noise,
            env,
            velocity: 0.0,
            active: false,
            // Valeurs par défaut pour le contrôle dynamique
            current_cutoff: 8000.0,
            decay_mod: 0.05,
        }
    }

    fn trigger(&mut self, velocity: u8) {
        self.velocity = velocity as f32 / 127.0;
        self.active = true;
        self.env.trigger();
    }

    fn process(&mut self) -> (f32, f32) {
        if !self.active {
            return (0.0, 0.0);
        }

        // 1. FILTRE DYNAMIQUE
        // On utilise le cutoff pour le filtre PASSE-HAUT (HP)
        // C'est ce qui définit le caractère "métallique"
        // Anger = 9000Hz (Très fin et aigu)
        // Calm = 2000Hz (Plus de corps, moins agressif)
        self.noise.set_hp_freq(self.current_cutoff * 0.5); // On divise pour garder du corps
        self.noise.set_lp_freq(self.current_cutoff * 2.0); // Le LP suit le HP pour créer une bande

        let noise_out = self.noise.process();
        let amp = self.env.process();

        if self.env.state() == EnvelopeState::Idle {
            self.active = false;
        }

        // 2. GAIN RÉDUIT
        // Le Hat perce le mix très facilement. On baisse de 0.25 à 0.15
        let out = noise_out * amp * self.velocity * 0.15;

        // Panoramique léger
        (out * 0.9, out * 1.1)
    }
}

// ============================================================================
// POLY VOICE (Channels 4+)
// ============================================================================

struct PolyVoice {
    osc: MultiOscillator,
    filter: LadderFilter,
    amp_env: Adsr,
    note: u8,
    channel: u8,
    velocity: f32,
    active: bool,
}

impl PolyVoice {
    fn new(sample_rate: f32) -> Self {
        let mut osc = MultiOscillator::new(sample_rate);
        osc.set_detune(0.15);
        osc.set_stereo_width(0.7);

        let mut filter = LadderFilter::new(sample_rate);
        filter.set_filter_type(LadderFilterType::LP4);
        filter.set_cutoff(3000.0);
        filter.set_resonance(0.2);

        let mut amp_env = Adsr::new(sample_rate);
        amp_env.set_attack(0.01);
        amp_env.set_decay(0.2);
        amp_env.set_sustain(0.6);
        amp_env.set_release(0.3);

        Self {
            osc,
            filter,
            amp_env,
            note: 0,
            channel: 0,
            velocity: 0.0,
            active: false,
        }
    }

    fn note_on(&mut self, note: u8, velocity: u8, channel: u8) {
        let freq = midi_to_freq(note);
        self.osc.set_frequency(freq);
        self.note = note;
        self.channel = channel;
        self.velocity = velocity as f32 / 127.0;
        self.active = true;
        self.amp_env.trigger();
    }

    fn note_off(&mut self) {
        self.amp_env.release();
    }

    fn process(&mut self) -> (f32, f32) {
        if !self.active {
            return (0.0, 0.0);
        }

        let (osc_l, osc_r) = self.osc.process_stereo();
        let filtered_l = self.filter.process(osc_l);
        let filtered_r = self.filter.process(osc_r);

        let amp = self.amp_env.process();

        if self.amp_env.state() == EnvelopeState::Idle {
            self.active = false;
        }

        let gain = amp * self.velocity;
        (filtered_l * gain, filtered_r * gain)
    }
}

// ============================================================================
// ODIN2 BACKEND
// ============================================================================

const MAX_POLY_VOICES: usize = 8;

/// Odin2 Audio Backend implementing AudioRenderer
pub struct Odin2Backend {
    // Dedicated instruments (channels 0-3)
    bass: BassVoice,
    lead: LeadVoice,
    snare: SnareVoice,
    hat: HatVoice,

    // Polyphonic voices for channels 4+
    poly_voices: Vec<PolyVoice>,

    // Mixer gains
    gain_bass: f32,
    gain_lead: f32,
    gain_snare: f32,
    gain_hat: f32,

    // Global effects
    delay: Delay,
    chorus: Chorus,
    reverb: ZitaReverb,

    // Effect mix levels
    delay_mix: f32,
    chorus_mix: f32,
    reverb_mix: f32,

    // Emotional morphing system
    morpher: EmotionalMorpher,
    last_bass_preset: Option<SynthPreset>,
    last_lead_preset: Option<SynthPreset>,
    last_snare_preset: Option<SynthPreset>,
    last_hat_preset: Option<SynthPreset>,
    last_poly_preset: Option<SynthPreset>,

    #[allow(dead_code)]
    sample_rate: f32,
    samples_per_step: usize,
}

impl Odin2Backend {
    pub fn new(sample_rate: f64) -> Self {
        let sr = sample_rate as f32;

        // Initialize dedicated instruments
        let bass = BassVoice::new(sr);
        let lead = LeadVoice::new(sr);
        let snare = SnareVoice::new(sr);
        let hat = HatVoice::new(sr);

        // Initialize poly voices
        let mut poly_voices = Vec::with_capacity(MAX_POLY_VOICES);
        for _ in 0..MAX_POLY_VOICES {
            poly_voices.push(PolyVoice::new(sr));
        }

        // Initialize effects
        let mut delay = Delay::new(sr);
        delay.set_delay_time(0.3);
        delay.set_feedback(0.3);
        delay.set_wet(0.15);
        delay.set_dry(0.85);

        let mut chorus = Chorus::new(sr);
        chorus.set_lfo_freq(0.5);
        chorus.set_amount(0.3);
        chorus.set_dry_wet(0.2);

        let mut reverb = ZitaReverb::new(sr);
        reverb.set_mix(0.15);

        // Initialize emotional morphing system
        let preset_bank = EmotionalPresetBank::default_presets();
        let morpher = EmotionalMorpher::new(preset_bank);

        Self {
            bass,
            lead,
            snare,
            hat,
            poly_voices,
            gain_bass: 0.6,
            gain_lead: 1.0,
            gain_snare: 0.5,
            gain_hat: 0.3,
            delay,
            chorus,
            reverb,
            delay_mix: 0.15,
            chorus_mix: 0.2,
            reverb_mix: 0.15,
            morpher,
            last_bass_preset: None,
            last_lead_preset: None,
            last_snare_preset: None,
            last_hat_preset: None,
            last_poly_preset: None,
            sample_rate: sr,
            samples_per_step: 0,
        }
    }

    /// Set mixer gains
    pub fn set_gains(&mut self, lead: f32, bass: f32, snare: f32, hat: f32) {
        self.gain_lead = lead;
        self.gain_bass = bass;
        self.gain_snare = snare;
        self.gain_hat = hat;
    }

    fn find_free_poly_voice(&self) -> Option<usize> {
        self.poly_voices.iter().position(|v| !v.active)
    }

    fn find_poly_voice_for_note(&self, note: u8, channel: u8) -> Option<usize> {
        self.poly_voices
            .iter()
            .position(|v| v.active && v.note == note && v.channel == channel)
    }

    /// Apply emotional morphing to synthesis parameters
    ///
    /// This is the main entry point for the emotional morphing system.
    /// It performs bilinear interpolation across the 4 emotional quadrants,
    /// applies tension/density modulation, and updates voice parameters.
    pub fn apply_emotional_morphing(&mut self, valence: f32, arousal: f32, tension: f32, density: f32) {
        // Step 1: Get morphed presets from bilinear interpolation
        let morphed = self.morpher.morph(valence, arousal);

        // Step 2: Apply tension/density modulation ON TOP
        let bass_final = apply_tension_density_modulation(&morphed.bass, tension, density);
        let lead_final = apply_tension_density_modulation(&morphed.lead, tension, density);
        let snare_final = apply_tension_density_modulation(&morphed.snare, tension, density);
        let hat_final = apply_tension_density_modulation(&morphed.hat, tension, density);
        let poly_final = apply_tension_density_modulation(&morphed.poly, tension, density);

        // Step 3: Apply to voices (only if changed)
        if self.preset_changed(&self.last_bass_preset, &bass_final) {
            self.apply_preset_to_bass(&bass_final);
            self.last_bass_preset = Some(bass_final);
        }

        if self.preset_changed(&self.last_lead_preset, &lead_final) {
            self.apply_preset_to_lead(&lead_final);
            self.last_lead_preset = Some(lead_final);
        }

        if self.preset_changed(&self.last_snare_preset, &snare_final) {
            self.apply_preset_to_snare(&snare_final);
            self.last_snare_preset = Some(snare_final);
        }

        if self.preset_changed(&self.last_hat_preset, &hat_final) {
            self.apply_preset_to_hat(&hat_final);
            self.last_hat_preset = Some(hat_final);
        }

        if self.preset_changed(&self.last_poly_preset, &poly_final) {
            self.apply_preset_to_poly(&poly_final);
            self.last_poly_preset = Some(poly_final);
        }

        // Step 4: Apply global effects
        self.apply_effects_params(&morphed.bass.effects);
    }

    /// Check if preset changed significantly enough to warrant updating
    fn preset_changed(&self, last: &Option<SynthPreset>, new: &SynthPreset) -> bool {
        match last {
            None => true,
            Some(last) => {
                // Check if any significant parameter changed
                (last.osc.waveform_mix - new.osc.waveform_mix).abs() > 0.01
                    || (last.osc.detune - new.osc.detune).abs() > 0.01
                    || (last.filter.cutoff - new.filter.cutoff).abs() > 10.0
                    || (last.filter.resonance - new.filter.resonance).abs() > 0.01
                    || (last.filter.drive - new.filter.drive).abs() > 0.05
                    || (last.envelopes.amp.attack - new.envelopes.amp.attack).abs() > 0.005
                    || (last.envelopes.amp.release - new.envelopes.amp.release).abs() > 0.005
            }
        }
    }

    /// Apply preset to bass voice
    fn apply_preset_to_bass(&mut self, preset: &SynthPreset) {
        // Filter params
        self.bass.filter.set_cutoff(preset.filter.cutoff);
        self.bass.filter.set_resonance(preset.filter.resonance);

        // Envelope params
        self.bass.env.set_attack(preset.envelopes.amp.attack);
        self.bass.env.set_decay(preset.envelopes.amp.decay);
        self.bass.env.set_sustain(preset.envelopes.amp.sustain);
        self.bass.env.set_release(preset.envelopes.amp.release);

        // Note: Waveform mix requires refactoring BassVoice to store
        // dynamic sine/saw ratios (currently hardcoded 70/30 mix)
        // This is a future enhancement
    }

    /// Apply preset to lead voice
    fn apply_preset_to_lead(&mut self, preset: &SynthPreset) {
        // MultiOscillator supports dynamic parameters
        self.lead.osc.set_detune(preset.osc.detune);
        self.lead.osc.set_stereo_width(preset.osc.stereo_width);

        // Filter
        self.lead.filter.set_cutoff(preset.filter.cutoff);
        self.lead.filter.set_resonance(preset.filter.resonance);
        self.lead.base_cutoff = preset.filter.cutoff;

        // Amp envelope
        self.lead.amp_env.set_attack(preset.envelopes.amp.attack);
        self.lead.amp_env.set_decay(preset.envelopes.amp.decay);
        self.lead.amp_env.set_sustain(preset.envelopes.amp.sustain);
        self.lead.amp_env.set_release(preset.envelopes.amp.release);

        // Filter envelope
        self.lead.filter_env.set_attack(preset.envelopes.filter.attack);
        self.lead.filter_env.set_decay(preset.envelopes.filter.decay);
        self.lead.filter_env.set_sustain(preset.envelopes.filter.sustain);
        self.lead.filter_env.set_release(preset.envelopes.filter.release);
    }

    /// Apply preset to snare voice
    fn apply_preset_to_snare(&mut self, preset: &SynthPreset) {
        // Snare uses noise, so less parameters to control
        // Envelope
        self.snare.env.set_attack(preset.envelopes.amp.attack);
        self.snare.env.set_decay(preset.envelopes.amp.decay);
        self.snare.env.set_sustain(preset.envelopes.amp.sustain);
        self.snare.env.set_release(preset.envelopes.amp.release);
    }

    /// Apply preset to hat voice
    fn apply_preset_to_hat(&mut self, preset: &SynthPreset) {
        // 1. Enveloppe de base
        self.hat.env.set_attack(preset.envelopes.amp.attack);
        // On garde le decay du preset comme "base", mais on pourra le moduler
        self.hat.decay_mod = preset.envelopes.amp.decay;
        self.hat.env.set_decay(self.hat.decay_mod);
        self.hat.env.set_sustain(0.0); // Toujours 0 pour un Hat
        self.hat.env.set_release(preset.envelopes.amp.release);

        // 2. Filtre Dynamique
        // Plus le cutoff est haut, plus le Hat est "tchik" (brillant)
        // Plus il est bas, plus il est "tsst" (douceur)
        self.hat.current_cutoff = preset.filter.cutoff;
    }

    /// Apply preset to poly voices
    fn apply_preset_to_poly(&mut self, preset: &SynthPreset) {
        for voice in &mut self.poly_voices {
            // MultiOscillator parameters
            voice.osc.set_detune(preset.osc.detune);
            voice.osc.set_stereo_width(preset.osc.stereo_width);

            // Filter
            voice.filter.set_cutoff(preset.filter.cutoff);
            voice.filter.set_resonance(preset.filter.resonance);

            // Amp envelope
            voice.amp_env.set_attack(preset.envelopes.amp.attack);
            voice.amp_env.set_decay(preset.envelopes.amp.decay);
            voice.amp_env.set_sustain(preset.envelopes.amp.sustain);
            voice.amp_env.set_release(preset.envelopes.amp.release);
        }
    }

    /// Apply global effects parameters
    fn apply_effects_params(&mut self, effects: &crate::synthesis::EffectsParams) {
        // Delay
        self.delay.set_delay_time(effects.delay.time);
        self.delay.set_feedback(effects.delay.feedback);
        self.delay.set_wet(effects.delay.mix);
        self.delay.set_dry(1.0 - effects.delay.mix);

        // Chorus
        self.chorus.set_lfo_freq(effects.chorus.lfo_freq);
        self.chorus.set_amount(effects.chorus.depth);
        self.chorus.set_dry_wet(effects.chorus.mix);

        // Reverb
        self.reverb.set_mix(effects.reverb.mix);
        // Note: ZitaReverb may not expose room_size/damping directly
        // Using default mix parameter for now
    }
}

impl AudioRenderer for Odin2Backend {
    fn handle_event(&mut self, event: AudioEvent) {
        match event {
            AudioEvent::NoteOn { note, velocity, channel } => {
                match InstrumentType::from_channel(channel) {
                    InstrumentType::Bass => {
                        self.bass.note_on(note, velocity);
                    }
                    InstrumentType::Lead => {
                        self.lead.note_on(note, velocity);
                    }
                    InstrumentType::Snare => {
                        self.snare.trigger(velocity);
                    }
                    InstrumentType::Hat => {
                        self.hat.trigger(velocity);
                    }
                    InstrumentType::Poly => {
                        if let Some(idx) = self.find_free_poly_voice() {
                            self.poly_voices[idx].note_on(note, velocity, channel);
                        }
                    }
                }
            }

            AudioEvent::NoteOff { note, channel } => {
                match InstrumentType::from_channel(channel) {
                    InstrumentType::Bass => {
                        self.bass.note_off();
                    }
                    InstrumentType::Lead => {
                        self.lead.note_off();
                    }
                    InstrumentType::Snare | InstrumentType::Hat => {
                        // Drums don't respond to note off (one-shot)
                    }
                    InstrumentType::Poly => {
                        if let Some(idx) = self.find_poly_voice_for_note(note, channel) {
                            self.poly_voices[idx].note_off();
                        }
                    }
                }
            }

            AudioEvent::AllNotesOff { channel } => {
                match InstrumentType::from_channel(channel) {
                    InstrumentType::Bass => self.bass.note_off(),
                    InstrumentType::Lead => self.lead.note_off(),
                    InstrumentType::Snare | InstrumentType::Hat => {}
                    InstrumentType::Poly => {
                        for voice in &mut self.poly_voices {
                            if voice.channel == channel {
                                voice.note_off();
                            }
                        }
                    }
                }
            }

            AudioEvent::ControlChange { ctrl, value, channel: _ } => {
                let val_norm = value as f32 / 127.0;
                match ctrl {
                    1 => {
                        // Modulation wheel -> filter cutoff on lead
                        self.lead.base_cutoff = 500.0 + val_norm * 3500.0;
                    }
                    91 => {
                        // Reverb
                        self.reverb_mix = val_norm * 0.4;
                        self.reverb.set_mix(self.reverb_mix);
                    }
                    93 => {
                        // Chorus
                        self.chorus_mix = val_norm * 0.4;
                        self.chorus.set_dry_wet(self.chorus_mix);
                    }
                    94 => {
                        // Delay
                        self.delay_mix = val_norm * 0.4;
                        self.delay.set_wet(self.delay_mix);
                        self.delay.set_dry(1.0 - self.delay_mix);
                    }
                    _ => {}
                }
            }

            AudioEvent::TimingUpdate { samples_per_step } => {
                self.samples_per_step = samples_per_step;
            }

            AudioEvent::SetMixerGains { lead, bass, snare, hat } => {
                self.set_gains(lead, bass, snare, hat);
            }

            // Events not applicable to Odin2 backend
            AudioEvent::LoadFont { .. } => {}
            AudioEvent::SetChannelRoute { .. } => {}
            AudioEvent::StartRecording { .. } => {}
            AudioEvent::StopRecording { .. } => {}
        }
    }

    fn process_buffer(&mut self, output: &mut [f32], channels: usize) {
        output.fill(0.0);

        for frame in output.chunks_mut(channels) {
            // Process dedicated instruments
            let (bass_l, bass_r) = self.bass.process();
            let (lead_l, lead_r) = self.lead.process();
            let (snare_l, snare_r) = self.snare.process();
            let (hat_l, hat_r) = self.hat.process();

            // Mix with gains
            let mut left = bass_l * self.gain_bass
                + lead_l * self.gain_lead
                + snare_l * self.gain_snare
                + hat_l * self.gain_hat;

            let mut right = bass_r * self.gain_bass
                + lead_r * self.gain_lead
                + snare_r * self.gain_snare
                + hat_r * self.gain_hat;

            // Add poly voices
            for voice in &mut self.poly_voices {
                if voice.active {
                    let (vl, vr) = voice.process();
                    left += vl;
                    right += vr;
                }
            }

            // Apply global effects chain
            let (dl, dr) = self.delay.process(left, right);
            // Chorus takes mono, returns stereo
            let chorus_in = (dl + dr) * 0.5;
            let (cl, cr) = self.chorus.process(chorus_in);
            let (rl, rr) = self.reverb.process(cl, cr);

            // Soft clipping
            frame[0] = (rl * 0.8_f32).tanh();
            if channels >= 2 {
                frame[1] = (rr * 0.8_f32).tanh();
            }
        }
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn odin2_backend_mut(&mut self) -> Option<&mut Odin2Backend> {
        Some(self)
    }
}
