use fundsp::hacker32::*;
use crate::voice_manager::{VoiceManager, ChannelType};
use crate::events::AudioEvent;
use crate::backend::AudioRenderer;
use crate::backend::adapter::BlockRateAdapter;

pub struct SynthBackend {
    voice_manager: VoiceManager,
    node: BlockRateAdapter,
    samples_per_step: usize, // Needed for VoiceManager::process_event
}

impl SynthBackend {
    pub fn new(sample_rate: f64, sf2_bytes: Option<&[u8]>, initial_routing: &[i32]) -> Self {
        // === 1. DSP GRAPH CONSTRUCTION ===
        
        // Paramètres partagés
        let frequency_lead = shared(440.0);
        let gate_lead = shared(0.0);
        let frequency_bass = shared(110.0);
        let gate_bass = shared(0.0);
        let gate_snare = shared(0.0);
        let gate_hat = shared(0.0);
        
        let cutoff = shared(1000.0);
        let resonance = shared(1.0);
        let distortion = shared(0.0);
        let fm_ratio = shared(2.0);
        let fm_amount = shared(0.3);
        let timbre_mix = shared(0.0);
        let reverb_mix = shared(0.25);

        // Mixer Gains (controllable from UI)
        let gain_lead = shared(1.0);
        let gain_bass = shared(0.6);
        let gain_snare = shared(0.5);
        let gain_hat = shared(0.4);

        // --- INSTRUMENT 1: LEAD (FM/Organic Hybrid) ---
        let drift_lfo = lfo(|t| (t * 0.3).sin() * 2.0); 
        let freq_lead_mod = var(&frequency_lead) + drift_lfo;

        // FM Path
        let mod_freq = freq_lead_mod.clone() * var(&fm_ratio);
        let modulator = mod_freq >> sine();
        let car_freq = freq_lead_mod.clone() + (modulator * var(&fm_amount) * freq_lead_mod.clone());
        let fm_voice = car_freq >> saw();

        // Organic Path
        let osc_organic = (freq_lead_mod.clone() >> triangle()) * 0.8 
                        + (freq_lead_mod.clone() >> square()) * 0.2;
        let breath = (noise() >> lowpass_hz(2000.0, 0.5)) * 0.15;
        let organic_voice = (osc_organic + breath) >> lowpass_hz(1200.0, 1.0);

        // Mix & Envelope
        let env_lead = var(&gate_lead) >> adsr_live(0.005, 0.2, 0.5, 0.15);
        let lead_mix = (organic_voice * (1.0 - var(&timbre_mix))) + (fm_voice * var(&timbre_mix));
        let lead_out = ((lead_mix * env_lead | var(&cutoff) | var(&resonance)) >> lowpass()) * var(&gain_lead) >> pan(0.3);

        // --- INSTRUMENT 2: BASS ---
        let bass_osc = (var(&frequency_bass) >> sine()) * 0.7 + (var(&frequency_bass) >> saw()) * 0.3;
        let env_bass = var(&gate_bass) >> adsr_live(0.005, 0.1, 0.6, 0.1);
        let bass_out = ((bass_osc * env_bass) >> lowpass_hz(800.0, 0.5)) * var(&gain_bass) >> pan(0.0);

        // --- INSTRUMENT 3: SNARE (Noise Burst + Tone) ---
        // Bruit blanc filtré passe-bande pour le "claquement"
        let snare_noise = noise() >> bandpass_hz(1500.0, 0.8);
        // Onde triangle rapide pour le corps (pitch drop rapide)
        // Note: fundsp statique limite les env de pitch complexes, on fait simple
        let snare_tone = sine_hz(180.0) >> saw(); 
        let snare_src = (snare_noise * 0.8) + (snare_tone * 0.2);
        let env_snare = var(&gate_snare) >> adsr_live(0.001, 0.1, 0.0, 0.1);
        let snare_out = (snare_src * env_snare) * var(&gain_snare) >> pan(-0.2);

        // --- INSTRUMENT 4: HAT (High Frequency Noise) ---
        // Bruit rose filtré passe-haut
        let hat_src = noise() >> highpass_hz(6000.0, 0.8);
        // Enveloppe très courte
        let env_hat = var(&gate_hat) >> adsr_live(0.001, 0.05, 0.0, 0.05);
        let hat_out = (hat_src * env_hat) * var(&gain_hat) >> pan(0.2);

        // --- MIXAGE FINAL ---
        let mix = lead_out + bass_out + snare_out + hat_out;
        
        let node = BlockRateAdapter::new(Box::new(mix), sample_rate);

        let mut voice_manager = VoiceManager::new(
            sf2_bytes, sample_rate as f32,
            frequency_lead, gate_lead,
            frequency_bass, gate_bass,
            gate_snare, gate_hat,
            cutoff, resonance, distortion,
            fm_ratio, fm_amount, timbre_mix, reverb_mix,
            gain_lead, gain_bass, gain_snare, gain_hat,
        );

        // Apply initial routing
        for (i, &mode) in initial_routing.iter().enumerate() {
             if i < 16 {
                 let mode_enum = if mode >= 0 { ChannelType::Oxisynth { bank: mode as u32 } } else { ChannelType::FundSP };
                 voice_manager.set_channel_route(i, mode_enum);
            }
        }

        Self {
            voice_manager,
            node,
            samples_per_step: 0, // Will be updated
        }
    }

    pub fn set_samples_per_step(&mut self, samples: usize) {
        self.samples_per_step = samples;
    }
    
    pub fn add_font(&mut self, id: u32, bytes: &[u8]) {
        self.voice_manager.add_font(id, bytes);
    }
    
    pub fn set_channel_route(&mut self, channel: usize, mode: ChannelType) {
        self.voice_manager.set_channel_route(channel, mode);
    }
    
    pub fn update_timers(&mut self) {
        self.voice_manager.update_timers();
    }

    pub fn set_gains(&mut self, lead: f32, bass: f32, snare: f32, hat: f32) {
        self.voice_manager.set_gains(lead, bass, snare, hat);
    }
}

impl AudioRenderer for SynthBackend {
    fn handle_event(&mut self, event: AudioEvent) {
        match event {
            AudioEvent::LoadFont { id, bytes } => {
                self.voice_manager.add_font(id, &bytes);
            },
            AudioEvent::SetChannelRoute { channel, bank } => {
                let mode = if bank >= 0 { ChannelType::Oxisynth { bank: bank as u32 } } else { ChannelType::FundSP };
                self.voice_manager.set_channel_route(channel as usize, mode);
            },
            AudioEvent::TimingUpdate { samples_per_step } => {
                self.samples_per_step = samples_per_step;
            },
            AudioEvent::SetMixerGains { lead, bass, snare, hat } => {
                self.voice_manager.set_gains(lead, bass, snare, hat);
            },
            _ => {
                self.voice_manager.process_event(event, self.samples_per_step);
            }
        }
    }

    fn process_buffer(&mut self, output: &mut [f32], channels: usize) {
        // 1. Oxisynth (Stereo)
        if channels == 2 {
            self.voice_manager.synth.write(&mut *output);
        } else {
            output.fill(0.0);
        }

        // 2. FundSP + Mix
        for frame in output.chunks_mut(channels) {
            self.voice_manager.update_timers();
            let (l, r) = self.node.get_stereo();
            
            frame[0] += l;
            if channels >= 2 {
                frame[1] += r;
            }
        }
    }
}
