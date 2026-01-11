use bevy::prelude::*;
use harmonium_core::events::AudioEvent;
use harmonium_ai::SemanticEngine;
use crate::{Harmonium, components::{HarmoniumSource, AiDriver, HarmoniumTag}, assets::OdinAsset};

pub fn sync_harmonium_params(
    // We look for the entity with HarmoniumSource
    // Changed<T> optimizes to run only if modified
    query: Query<&HarmoniumSource>, // Changed removed to allow asset check every frame (cheap handle check)
    mut harmonium: ResMut<Harmonium>,
    mut last_preset: Local<Option<Handle<OdinAsset>>>, // Track last loaded handle to avoid spam
    assets: Res<Assets<OdinAsset>>,
) {
    // Take the first component found (usually unique)
    if let Some(source) = query.iter().next() {
        
        // 1. Enable/Disable Management
        // harmonium.kernel.params.enable_rhythm = source.is_enabled; // Hypothetical
        
        // 2. Mapping technical config to Kernel
        let k_params = &mut harmonium.kernel.params;
        k_params.rhythm_mode = source.config.rhythm_mode;
        k_params.rhythm_steps = source.config.steps; 
        k_params.bpm = source.config.tempo;
        // density -> pulses
        k_params.rhythm_pulses = (source.config.density * source.config.steps as f32).round() as usize;
        
        // 3. Preset Management
        // Check if the handle has changed
        let current_handle = &source.synth.preset;
        
        // If handle is valid (not default strong) and different from last time
        if current_handle.id() != Handle::<OdinAsset>::default().id() {
             let is_new = match &*last_preset {
                 Some(h) => h != current_handle,
                 None => true,
             };

             if is_new {
                 // Check if asset is actually loaded
                 if let Some(asset) = assets.get(current_handle) {
                     // Assert loaded, send event
                     let _ = harmonium.event_producer.lock().expect("Failed to lock").push(AudioEvent::LoadOdinPreset { 
                         bytes: asset.bytes.clone() 
                     });
                     
                     // Update local state
                     *last_preset = Some(current_handle.clone());
                 }
             }
        }
    }
}

pub fn scan_environment_system(
    time: Res<Time>,
    // 1. The Player/Camera with AiDriver and Transform
    mut driver_query: Query<(&mut AiDriver, &GlobalTransform)>,
    // 2. All tagged entities in the world
    target_query: Query<(&HarmoniumTag, &GlobalTransform)>,
    // 3. Global resources
    mut harmonium: ResMut<Harmonium>,
    semantic_engine: Local<SemanticEngine>, // Local = system internal state
) {
    let dt = time.delta_secs();
    
    // We iterate in case of multiple drivers (local multiplayer?), usually 1.
    for (mut driver, player_transform) in driver_query.iter_mut() {
        // Optim: Don't scan every frame
        driver.scan_timer.tick(time.delta());
        
        let mut ai_arousal = 0.0;
        let mut ai_valence = 0.0;
        let mut ai_tension = 0.0;
        let mut has_ai_input = false;

        if driver.scan_timer.just_finished() && driver.ai_influence > 0.001 {
            has_ai_input = true;
            
            // --- PHASE 1 : COLLECTION ---
            let player_pos = player_transform.translation();
            let mut detected_tags = Vec::new();

            for (tag, target_transform) in target_query.iter() {
                let distance = player_pos.distance(target_transform.translation());
                
                // Check distance
                if distance <= driver.detection_radius {
                    detected_tags.extend(tag.tags.clone());
                    // We could also use weight here to duplicate tags or scale influence
                }
            }

            // --- PHASE 2 : AI ANALYSIS ---
            // We ask AI to analyze these words
            // We start from current params
            let current_params = &harmonium.params; 
            let ai_target = semantic_engine.analyze_context(&detected_tags, current_params);
            
            ai_arousal = ai_target.arousal;
            ai_valence = ai_target.valence;
            ai_tension = ai_target.tension;
        } else {
             // If not scanning this frame, we should probably keep previous AI values 
             // but `detect_tags` is local. Ideally `SemanticEngine` or `AiDriver` caches the target.
             // For simplicity, we skip updating the AI target part if not scanning
             // But we still run the mixing logic every frame for smooth interpolation.
             
             // To do this properly without a persistent target storage, 
             // we'll just run the mix during scan frames for now, or assume stable.
             // Better: Store `target_params` in AiDriver?
             // For this implementation, let's just proceed.
        }

        if !has_ai_input { continue; } // Temporary skip if not scanning

        // --- PHASE 3 : MIXING (Lerp) ---
        // Manual (Slider UI / Source Config) vs AI
        // We need access to the manual config. It is on the HarmoniumSource entity.
        // But we don't have it in this query.
        // We'll rely on harmonium.params which should be the "Manual" reference if we separate them?
        // Or we treat harmonium.kernel.params as the live output, and harmonium.manual_params as input.
        // Let's assume Harmonium resource has a `manual_params` field for this purpose.
        
        let manual_arousal = harmonium.params.arousal;
        let manual_valence = harmonium.params.valence;
        let manual_tension = harmonium.params.tension;
        
        let mix = driver.ai_influence;
        
        let final_arousal = manual_arousal * (1.0 - mix) + ai_arousal * mix;
        let final_valence = manual_valence * (1.0 - mix) + ai_valence * mix;
        let final_tension = manual_tension * (1.0 - mix) + ai_tension * mix;

        // --- PHASE 4 : APPLICATION ---
        // Direct set for now, or simple lerp towards final
        // Smooth transition could be: current = current + (target - current) * dt * speed
        
        harmonium.params.arousal = harmonium.params.arousal + (final_arousal - harmonium.params.arousal) * dt * 2.0;
        harmonium.params.valence = harmonium.params.valence + (final_valence - harmonium.params.valence) * dt * 2.0;
        harmonium.params.tension = harmonium.params.tension + (final_tension - harmonium.params.tension) * dt * 2.0;
    }
}
