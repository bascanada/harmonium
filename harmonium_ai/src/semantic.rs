use std::collections::HashMap;
use harmonium_core::params::EngineParams;

/// Represents the emotional impact of a word on the engine parameters.
#[derive(Debug, Clone)]
pub struct WordWeight {
    pub arousal_delta: f32, // +Energy
    pub valence_delta: f32, // +Happiness / -Sadness
    pub tension_delta: f32, // +Dissonance
}

/// The semantic engine that translates environmental tags into emotional parameters.
pub struct SemanticEngine {
    // In the future: this could be replaced by a BERT model or embeddings
    dictionary: HashMap<String, WordWeight>,
}

impl Default for SemanticEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl SemanticEngine {
    pub fn new() -> Self {
        let mut dict = HashMap::new();
        
        // --- DANGER / COMBAT ---
        dict.insert("monster".to_string(), WordWeight { arousal_delta: 0.3, valence_delta: -0.4, tension_delta: 0.5 });
        dict.insert("danger".to_string(), WordWeight { arousal_delta: 0.5, valence_delta: -0.5, tension_delta: 0.6 });
        dict.insert("boss".to_string(), WordWeight { arousal_delta: 0.8, valence_delta: -0.2, tension_delta: 0.8 });
        dict.insert("combat".to_string(), WordWeight { arousal_delta: 0.6, valence_delta: -0.1, tension_delta: 0.4 });
        
        // --- ATMOSPHERE ---
        dict.insert("dark".to_string(), WordWeight { arousal_delta: -0.1, valence_delta: -0.3, tension_delta: 0.2 });
        dict.insert("scary".to_string(), WordWeight { arousal_delta: 0.2, valence_delta: -0.6, tension_delta: 0.4 });
        dict.insert("mechanical".to_string(), WordWeight { arousal_delta: 0.0, valence_delta: -0.1, tension_delta: 0.3 });
        dict.insert("nature".to_string(), WordWeight { arousal_delta: -0.2, valence_delta: 0.4, tension_delta: -0.2 });
        
        // --- SAFE ---
        dict.insert("safe".to_string(), WordWeight { arousal_delta: -0.4, valence_delta: 0.5, tension_delta: -0.5 });
        dict.insert("holy".to_string(), WordWeight { arousal_delta: -0.1, valence_delta: 0.6, tension_delta: -0.3 });
        dict.insert("light".to_string(), WordWeight { arousal_delta: 0.1, valence_delta: 0.4, tension_delta: -0.2 });

        Self { dictionary: dict }
    }

    /// Analyzes a list of semantic tags present in the environment and modifies the target parameters.
    /// 
    /// # Arguments
    /// * `tags` - A list of strings representing the current context (e.g. "monster", "dark").
    /// * `base_params` - The base parameters to start from (usually neutral or the current manual settings).
    /// 
    /// # Returns
    /// The mapped `EngineParams` adjusted by the semantic context.
    pub fn analyze_context(&self, tags: &[String], base_params: &EngineParams) -> EngineParams {
        let mut target = base_params.clone();
        
        let mut total_arousal = 0.0;
        let mut total_valence = 0.0;
        let mut total_tension = 0.0;
        
        // Using f32 for count to allow potential weighted averaging later
        let count = tags.len();

        if count == 0 { return target; }

        for tag in tags {
            // Find the word or a default minimal impact if not found?
            // For now, only known words have impact.
            // In a ML version, we would embed the word and check distance.
            if let Some(weight) = self.dictionary.get(tag) {
                total_arousal += weight.arousal_delta;
                total_valence += weight.valence_delta;
                total_tension += weight.tension_delta;
            }
        }

        // Apply deltas. We perform clamping to ensure values stay in valid ranges.
        // Assuming params are generally 0.0-1.0 or -1.0-1.0
        
        target.arousal = (target.arousal + total_arousal).clamp(0.0, 1.0);
        
        // Valence is usually -1.0 to 1.0
        target.valence = (target.valence + total_valence).clamp(-1.0, 1.0);
        
        // Tension is usually 0.0 to 1.0
        target.tension = (target.tension + total_tension).clamp(0.0, 1.0);

        target
    }
}
