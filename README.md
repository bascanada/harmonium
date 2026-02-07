# Harmonium: Reactive Procedural Music Generation

Harmonium is a Rust ecosystem for creating procedural music driven by emotions. It combines music theory, geometric rhythms, and advanced synthesis into a reactive engine designed for applications, games, and interactive installations.

## Architecture

The project is organized as a Cargo workspace with specialized crates:

| Crate | Role | Description |
|-------|------|-------------|
| **`harmonium_core`** | **The Brain** | Platform-agnostic logic. Contains the `MusicKernel`, `Sequencer`, and `HarmonicDriver`. Generates abstract `AudioEvent`s. |
| **`harmonium_audio`** | **The Lung** | Sound generation engine. Supports hot-swappable backends (**Odin2**, **FundSP**, **SoundFonts**) and handles emotional timbre morphing. |
| **`harmonium_ai`** | **The Soul** | Semantic layer. Uses **Candle** (BERT) to translate natural language into emotional parameters (Valence/Arousal/Tension). |
| **`harmonium_host`** | **The Body** | Runtime environment. Bridges logic and audio, managing threads, **Triple Buffers**, and platform integration (CLI, VST3, WASM). |
| **`web`** | **The Face** | SvelteKit frontend. Provides the UI for the WASM build and the VST editor. |

## The Method: From Emotion to Sound

Harmonium translates a high-level "Emotional State" into concrete audio through five distinct layers:

### 1. The Context (AI & Input) -> `harmonium_ai`
*   **Input**: User text ("A dark, mysterious cave") or direct parameters (Valence, Arousal).
*   **Processing**: A **BERT** model (via `candle`) computes semantic embeddings and maps them to **Russell's Circumplex Model**.
*   **Output**: `EngineParams` (BPM, Scale, Density, Tension).

### 2. The Skeleton (Rhythm) -> `harmonium_core::sequencer`
*   **Euclidean Rhythms**: Uses Bjorklund’s algorithm to distribute pulses evenly (e.g., 3 hits in 8 steps = Tresillo).
*   **Perfect Balance**: Implements "Well-Formed" rhythms by superimposing regular polygons (Triangle, Square) based on XronoMorph theory.
*   **Polyrhythm**: Dual sequencers run in parallel (e.g., 16 steps vs 12 steps) to create phasing patterns (Steve Reich style).

### 3. The Body (Harmony) -> `harmonium_core::harmony`
*   **Generative Grammar**: Uses **Steedman's Grammar** to create logical, resolving jazz/pop phrases (Syntax Trees).
*   **Neo-Riemannian Theory**: Uses the **Tonnetz** topology (PLR transformations) to connect chords that are geometrically close but tonally distant, creating cinematic transitions.
*   **Lydian Chromatic Concept**: Filters all pitch choices through George Russell's "Tonal Gravity" to ensure coherence regardless of complexity.

### 4. The Voice (Melody & Voicing) -> `harmonium_audio::voicing`
*   **Fractal Melody**: Melodic contours are guided by **Pink Noise (1/f)** to balance unpredictability with structure.
*   **Smart Voicing**:
    *   **Block Chords**: George Shearing style (locked hands).
    *   **Shell Voicings**: Be-Bop style (Root, 3rd, 7th).
    *   **Drop-2**: Spreads close voicings for a more open sound.

### 5. The Lung (Audio Synthesis) -> `harmonium_audio::backend`
*   **Odin 2 Integration**: Embeds the powerful **Odin 2** synthesizer engine (ported to Rust).
*   **Emotional Morphing**: Synthesizer presets are mapped to emotional quadrants. The engine bilinearly interpolates parameters in real-time, morphing the timbre from "Sad" to "Angry" seamlessly.
*   **Hybrid Engine**: Fallback to **FundSP** (Functional DSP) and **OxiSynth** (SoundFonts) for lightweight targets.

## Usage

### Standalone (CLI)
Real-time playback, OSC control, and recording.

```bash
# Run with default settings
cargo run --release --bin harmonium

# Record to WAV/MIDI
cargo run --release --bin harmonium -- --record-wav output.wav --record-midi output.mid

# Enable OSC control (UDP 8080)
cargo run --release --bin harmonium -- --osc
```

### VST3 / CLAP Plugin
Runs as a MIDI generator in your DAW.

```bash
# Build plugin bundle
cargo xtask bundle harmonium --release
```

## Technology Stack

*   **Logic**: `rust-music-theory`, `midly`, `rand`
*   **Audio**: `cpal` (I/O), `odin2-core` (Synth), `fundsp` (DSP), `oxisynth` (SF2), `hound` (WAV)
*   **AI**: `candle-core`, `candle-transformers`, `tokenizers`
*   **Plugin**: `nih_plug`
*   **Web**: `wasm-bindgen`, SvelteKit

## Scientific Foundations

*   **Loy, Gareth**: *Musimathics* (Mathematical Foundations)
*   **Toussaint, Godfried**: *The Geometry of Musical Rhythm*
*   **Russell, George**: *The Lydian Chromatic Concept of Tonal Organization*
*   **Steedman, Mark**: *A Generative Grammar for Jazz Chord Sequences*
*   **Cohn, Richard**: *Audacious Euphony* (Neo-Riemannian Theory)
