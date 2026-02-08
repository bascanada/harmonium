# Harmonium AI

`harmonium_ai` is the semantic intelligence layer of the Harmonium project. It translates natural language descriptions and emotional contexts into precise musical parameters that drive the generation engine.

## Architecture

The crate is composed of three main layers that work together to bridge the gap between human concepts and machine execution:

```mermaid
graph TD
    Input[Text/Tags] --> AI[EmotionEngine (BERT)]
    Input --> Semantic[SemanticEngine (Keywords)]
    AI --> EmotionalParams[Valence/Arousal/Tension]
    Semantic --> EmotionalParams
    EmotionalParams --> Mapper[EmotionMapper]
    Mapper --> MusicalParams[BPM/Scale/Chords]
```

### 1. Emotion Engine (`src/ai.rs`)
The heavy-lifting AI component powered by **Candle** (Hugging Face's minimalist ML framework for Rust).

*   **Model**: Runs a quantized **BERT** model (or similar Transformer) to generate semantic embeddings for input text.
*   **Anchor System**: Instead of training a regression model from scratch, it uses a "Few-Shot" approach with **Semantic Anchors**.
    *   Pre-defined anchors exist for specific moods (e.g., "Victory", "Dungeon", "Battle").
    *   The user's input is compared to these anchors using **Cosine Similarity**.
    *   The final emotional parameters are a weighted average of the closest anchors.
*   **WASM Support**: Compiles to WebAssembly, allowing the BERT model to run entirely in the browser (client-side AI).

### 2. Emotion Mapper (`src/mapper.rs`)
A deterministic logic layer that implements **Russell's Circumplex Model** of affect. It translates high-level emotional dimensions into concrete musical parameters.

| Emotional Dimension | Musical Parameter | Logic |
|---------------------|-------------------|-------|
| **Arousal** (Energy) | BPM, Velocity | Higher arousal = Faster tempo, louder dynamics. |
| **Valence** (Mood) | Scale, Harmony | Positive = Major/Lydian. Negative = Minor/Phrygian. |
| **Tension** (Stress) | Dissonance, Strategy | Low = Functional Harmony. High = Neo-Riemannian/Chromatic. |
| **Density** (Complexity) | Note Density, Polyrhythm | Higher density = More active rhythms, complex signatures. |

### 3. Semantic Engine (`src/semantic.rs`)
A lightweight, dictionary-based fallback for environments where running a full BERT model is too costly (e.g., constrained embedded systems or rapid tag processing).
*   Maps specific keywords (e.g., "scary", "safe", "mechanical") to parameter deltas.

## Usage

### Using the Emotion Engine (WASM/Native)

```rust
use harmonium_ai::ai::EmotionEngine;

// Initialize with model weights (loaded from files/network)
let engine = EmotionEngine::new(config_data, weights_data, tokenizer_data)?;

// Analyze text
let params = engine.predict_native("A dark and mysterious cave with glowing crystals")?;

println!("Predicted Arousal: {}", params.arousal);
println!("Predicted Valence: {}", params.valence);
```

### Using the Mapper

```rust
use harmonium_ai::mapper::EmotionMapper;
use harmonium_core::params::EngineParams;

let mapper = EmotionMapper::new();
let emotions = EngineParams {
    arousal: 0.8, // High energy
    valence: -0.5, // Negative mood (Stress/Battle)
    ..Default::default()
};

let musical_params = mapper.map(&emotions);

// The mapper decides the implementation details:
assert!(musical_params.bpm > 140.0); // Fast
assert!(musical_params.harmony_mode == HarmonyMode::Driver); // Complex harmony
```

## Dependencies

*   **`candle-core`**: Tensor operations.
*   **`candle-transformers`**: Pre-trained model architectures.
*   **`tokenizers`**: Text tokenization.
*   **`wasm-bindgen`**: WASM glue code.
