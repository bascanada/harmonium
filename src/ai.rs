use crate::engine::EngineParams;
use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config};
use tokenizers::Tokenizer;
use wasm_bindgen::prelude::*;

struct Anchor {
    #[allow(dead_code)]
    text: String,
    embedding: Vec<f32>,
    params: EngineParams,
}

#[wasm_bindgen]
pub struct EmotionEngine {
    model: BertModel,
    tokenizer: Tokenizer,
    anchors: Vec<Anchor>,
}

#[wasm_bindgen]
impl EmotionEngine {
    /// Initialiser le moteur avec les fichiers binaires chargés depuis le JS
    pub fn new(
        config_data: &[u8],
        weights_data: &[u8],
        tokenizer_data: &[u8],
    ) -> Result<EmotionEngine, JsError> {
        // 1. Configuration
        let config: Config =
            serde_json::from_slice(config_data).map_err(|e| JsError::new(&e.to_string()))?;

        // 2. Chargement du modèle depuis la mémoire (CPU pour commencer)
        let device = Device::Cpu;
        let vb = VarBuilder::from_slice_safetensors(weights_data, DType::F32, &device)
            .map_err(|e| JsError::new(&e.to_string()))?;

        let model = BertModel::load(vb, &config).map_err(|e| JsError::new(&e.to_string()))?;

        // 3. Tokenizer
        let tokenizer =
            Tokenizer::from_bytes(tokenizer_data).map_err(|e| JsError::new(&e.to_string()))?;

        let mut engine = EmotionEngine {
            model,
            tokenizer,
            anchors: Vec::new(),
        };

        engine.init_anchors()?;

        Ok(engine)
    }

    fn init_anchors(&mut self) -> Result<(), JsError> {
        // Use prototypical sentences for better semantic matching with Sentence-BERT
        // Format: (Text, Arousal, Valence, Tension, Density)
        // Arousal: 0.0 (Calme) -> 1.0 (Excité/Rapide)
        // Valence: -1.0 (Triste/Négatif) -> 1.0 (Heureux/Positif)
        // Tension: 0.0 (Consonant/Pur) -> 1.0 (Dissonant/Bruyant)
        // Density: 0.0 (Minimaliste) -> 1.0 (Chargé/Rapide)

        let definitions = vec![
            // === QUADRANT 1: JOIE / VICTOIRE (High Valence, High Arousal) ===
            // Son: Majeur, Rapide, Brillant, Consonant
            (
                "victory win success celebration triumph reward happy joy excited energetic dance glory light sun",
                0.85, // Arousal: Élevé (Rapide)
                0.9,  // Valence: Très positif (Majeur)
                0.1,  // Tension: Faible (Son pur/harmonique)
                0.7,  // Density: Rythmé
            ),
            // === QUADRANT 2: STRESS / COMBAT (Low Valence, High Arousal) ===
            // Son: Dissonant, Rapide, Métallique (FM élevé), Distorsion
            (
                "battle fight boss danger run escape panic fire scream blood kill weapon enemy aggression war intensity action",
                0.95, // Arousal: Max (Très rapide)
                -0.6, // Valence: Négatif (Menaçant)
                0.9,  // Tension: Max (FM Ratio élevé, inharmonique)
                0.9,  // Density: Très dense (Beaucoup de notes)
            ),
            // === QUADRANT 3: TRISTESSE / DONJON (Low Valence, Low Arousal) ===
            // Son: Mineur, Lent, Sombre, Reverb "froide", Nappes
            (
                "darkness cave dungeon crypt ghost horror fear mystery death sorrow grief lonely cold rain shadow abyss creep",
                0.2,  // Arousal: Faible (Lent)
                -0.9, // Valence: Très négatif (Mineur sombre)
                0.7,  // Tension: Élevée (Dissonances lentes, inquiétant)
                0.2,  // Density: Peu de notes (Atmosphérique)
            ),
            // === QUADRANT 4: CALME / EXPLORATION (High Valence, Low Arousal) ===
            // Son: Majeur/Neutre, Lent, Spacieux (Reverb), Doux
            (
                "nature forest river peace calm safe village home rest sleep meditation heal gentle clouds breeze float",
                0.1, // Arousal: Très faible (Très lent)
                0.7, // Valence: Positif (Apaisant)
                0.0, // Tension: Nulle (Son très pur/sine)
                0.3, // Density: Calme
            ),
            // === SPÉCIAL: MÉCANIQUE / PUZZLE (Neutre) ===
            // Son: Séquencé, Précis, "Bleep-bloop", FM modérée
            (
                "technology robot machine logic math puzzle structure clock time industry metal future science neutral",
                0.5, // Arousal: Moyen
                0.0, // Valence: Neutre
                0.4, // Tension: Moyenne (Son un peu synthétique/froid)
                0.6, // Density: Régulier
            ),
            // === SPÉCIAL: MAGIE / ÉTHÉRÉ ===
            // Son: Beaucoup de reverb, Aigu, Scintillant
            (
                "magic spell crystal star space cosmos dream spirit mystery wonder divine ethereal glow",
                0.4, // Arousal: Moyen-bas
                0.5, // Valence: Positif mais mystérieux
                0.2, // Tension: Basse
                0.8, // Density: Dense mais léger (arpèges rapides)
            ),
        ];

        let mut anchors = Vec::new();
        for (text, arousal, valence, tension, density) in definitions {
            let embedding = self.get_embedding(text)?;
            anchors.push(Anchor {
                text: text.to_string(),
                embedding,
                params: EngineParams {
                    arousal,
                    valence,
                    tension,
                    density,
                    smoothness: 0.5,
                    algorithm: crate::sequencer::RhythmMode::default(),
                    channel_routing: vec![0; 16],
                },
            });
        }
        self.anchors = anchors;
        Ok(())
    }

    /// Analyser un texte et retourner les paramètres émotionnels
    pub fn predict(&self, text: &str) -> Result<JsValue, JsError> {
        // 1. Get the embedding for the WHOLE user input at once
        // This captures context (e.g. "Not happy" will be different from "Happy")
        let input_embedding = self.get_embedding(text)?;

        let mut total_score = 0.0;
        let mut accum_params = EngineParams {
            arousal: 0.0,
            valence: 0.0,
            tension: 0.0,
            density: 0.0,
            smoothness: 0.0,
            algorithm: crate::sequencer::RhythmMode::default(),
            channel_routing: vec![0; 16],
        };

        // 2. Compare the input vector to each Anchor vector
        for anchor in &self.anchors {
            let similarity = Self::cosine_similarity(&input_embedding, &anchor.embedding);

            // 3. Softmax-like logic or Thresholding
            // If similarity is negative (opposite meaning), ignore it or clamp to 0
            let score = if similarity > 0.0 {
                similarity.powi(3)
            } else {
                0.0
            }; // powi(3) accentuates strong matches

            accum_params.arousal += anchor.params.arousal * score;
            accum_params.valence += anchor.params.valence * score;
            accum_params.tension += anchor.params.tension * score;
            accum_params.density += anchor.params.density * score;

            total_score += score;
        }

        // 4. Normalize
        if total_score > 0.001 {
            accum_params.arousal /= total_score;
            accum_params.valence /= total_score;
            accum_params.tension /= total_score;
            accum_params.density /= total_score;
        } else {
            // Fallback if no similarity found (return neutral/ambient)
            accum_params = EngineParams {
                arousal: 0.3,
                valence: 0.0,
                tension: 0.1,
                density: 0.2,
                smoothness: 0.5,
                algorithm: crate::sequencer::RhythmMode::default(),
                channel_routing: vec![0; 16],
            };
        }

        accum_params.smoothness = 0.5; // Constant or derived logic

        Ok(serde_wasm_bindgen::to_value(&accum_params)?)
    }

    /// Analyser un texte et retourner son embedding (CLS token)
    /// Retourne un Vec<f32> qui représente le vecteur sémantique du texte
    pub fn get_embedding(&self, text: &str) -> Result<Vec<f32>, JsError> {
        let device = Device::Cpu;

        // Tokenization
        let tokens = self
            .tokenizer
            .encode(text, true)
            .map_err(|e| JsError::new(&e.to_string()))?;
        let token_ids = Tensor::new(tokens.get_ids(), &device)
            .map_err(|e| JsError::new(&e.to_string()))?
            .unsqueeze(0)
            .map_err(|e| JsError::new(&e.to_string()))?;

        let token_type_ids = Tensor::new(tokens.get_type_ids(), &device)
            .map_err(|e| JsError::new(&e.to_string()))?
            .unsqueeze(0)
            .map_err(|e| JsError::new(&e.to_string()))?;

        let attention_mask = Tensor::new(tokens.get_attention_mask(), &device)
            .map_err(|e| JsError::new(&e.to_string()))?
            .unsqueeze(0)
            .map_err(|e| JsError::new(&e.to_string()))?;

        // Inférence
        let embeddings = self
            .model
            .forward(&token_ids, &token_type_ids, Some(&attention_mask))
            .map_err(|e| JsError::new(&e.to_string()))?;

        // On prend le token [CLS] (premier token, index 0) pour la représentation globale
        // Shape: [1, seq_len, hidden_size] -> [1, hidden_size]
        let cls_embedding = embeddings
            .get(0)
            .map_err(|e| JsError::new(&e.to_string()))?
            .get(0)
            .map_err(|e| JsError::new(&e.to_string()))?;

        // Convertir en Vec<f32>
        let vec: Vec<f32> = cls_embedding
            .to_vec1()
            .map_err(|e| JsError::new(&e.to_string()))?;

        Ok(vec)
    }

    /// Calcule la similarité cosinus entre deux vecteurs
    pub fn cosine_similarity(v1: &[f32], v2: &[f32]) -> f32 {
        let dot_product: f32 = v1.iter().zip(v2.iter()).map(|(a, b)| a * b).sum();
        let norm_v1: f32 = v1.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_v2: f32 = v2.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_v1 == 0.0 || norm_v2 == 0.0 {
            0.0
        } else {
            dot_product / (norm_v1 * norm_v2)
        }
    }
}
