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
        k_params.rhythm_mode = source.config.rhythm_mode.into();
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
                     if let Ok(mut producer) = harmonium.event_producer.lock() {
                         let _ = producer.push(AudioEvent::LoadOdinPreset { 
                             channel: 0, // Default to channel 0 for now
                             bytes: asset.bytes.clone() 
                         });
                    }
                     
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
    // 3. Source of Manual configuration
    source_query: Query<&HarmoniumSource>,
    // 4. Global resources
    mut harmonium: ResMut<Harmonium>,
    semantic_engine: Local<SemanticEngine>, // Local = system internal state
) {
    let dt = time.delta_secs();
    
    // Attempt to get the manual params (source of truth for manual override)
    // If multiple sources exist, we take the first one found.
    let manual_params: harmonium_core::EngineParams = if let Some(source) = source_query.iter().next() {
        source.manual_visual_params.clone().into()
    } else {
        // Fallback if no source entity exists
        harmonium.params.clone()
    };

    // We iterate in case of multiple drivers (local multiplayer?), usually 1.
    for (mut driver, player_transform) in driver_query.iter_mut() {
        // Optim: Don't scan every frame
        driver.scan_timer.tick(time.delta());

        if driver.scan_timer.just_finished() && driver.ai_influence > 0.001 {
            
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
            // We start from current params (or better, from neutral?)
            // Usually we want the AI contribution relative to "neutral" or Relative to "current manual"?
            // analyze_context takes a base_params. If we want absolute emotional state from tags, base should be neutral.
            // If we pass manual_params, the AI modifies it.
            let ai_target = semantic_engine.analyze_context(&detected_tags, &manual_params);
            
            // Cache the target in the component
            driver.ai_target = ai_target.into();
        } 

        // --- PHASE 3 : MIXING (Lerp) ---
        // Manual (from Component) vs AI (from cached Driver)
        
        let ai_target: harmonium_core::EngineParams = driver.ai_target.clone().into();
        let mix = driver.ai_influence;
        
        let final_arousal = manual_params.arousal * (1.0 - mix) + ai_target.arousal * mix;
        let final_valence = manual_params.valence * (1.0 - mix) + ai_target.valence * mix;
        let final_tension = manual_params.tension * (1.0 - mix) + ai_target.tension * mix;

        // --- PHASE 4 : APPLICATION ---
        // Frame-rate independent smoothing
        // lerp(target, 1.0 - exp(-speed * dt))
        let speed = 2.0;
        let f = 1.0 - (-speed * dt).exp();
        
        harmonium.params.arousal = harmonium.params.arousal + (final_arousal - harmonium.params.arousal) * f;
        harmonium.params.valence = harmonium.params.valence + (final_valence - harmonium.params.valence) * f;
        harmonium.params.tension = harmonium.params.tension + (final_tension - harmonium.params.tension) * f;
    }
}
