# AI & Emotion Mapping

Harmonium is designed to be controlled using high-level emotional concepts rather than technical musical parameters. This is achieved through the `EmotionMapper` and the optional `SemanticEngine`.

## The Russell Circumplex Model

Harmonium uses a 2D emotional space (Arousal and Valence):
- **Arousal (0.0 to 1.0)**: Represents physical energy.
    - Maps to **BPM**, **Rhythmic Density**, and **Distortion**.
- **Valence (-1.0 to 1.0)**: Represents positivity/negativity.
    - Maps to **Scale (Major/Minor)**, **Chord Selection**, and **Reverb Amount**.

## EmotionMapper

The `EmotionMapper` is a deterministic engine that translates `EngineParams` (Emotion) into `MusicalParams` (Technical).

| Input Parameter | Musical Impact |
| :--- | :--- |
| **Arousal** | BPM (70 - 180), Snare/Kick Velocity, FM Ratio |
| **Valence** | Harmony Strategy selection, Major/Minor bias, Pivot probability |
| **Density** | Number of sequencer pulses, Voicing complexity |
| **Tension** | Geometric PLR transformations, Syncopation, Filter Cutoff |

## Semantic Engine (AI Feature)

The `SemanticEngine` allows users to control the system using natural language labels (e.g., "Sad", "Intense", "Ethereal").

- **Model**: A small neural network (typically a Transformer or ML MLP) trained on emotional-musical mappings.
- **Workflow**:
    1. Label (e.g., "Cinematic") is sent via OSC.
    2. The model predicts the `(A, V, D, T)` coordinates.
    3. The engine morphs smoothly to these new parameters.

## Morphing Engine

To avoid jarring jumps in sound, Harmonium implements a **Morphing Engine**:
- Parameters like BPM and Filter Cutoff are interpolated over several audio blocks using a configurable `morph_factor`.
- This ensures that a transition from "Angry" to "Calm" feels like a natural musical evolution rather than a sudden state change.
