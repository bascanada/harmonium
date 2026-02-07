# Harmonium Core

`harmonium_core` is the brain of the Harmonium project. It contains the platform-agnostic logic for music generation, harmonic analysis, rhythmic sequencing, and event scheduling. It is designed to be compiled to both native targets (for the VST/Desktop app) and WASM (for the Web interface).

## Architecture

The core revolves around the **`MusicKernel`**, which acts as the central conductor. It orchestrates the interaction between the **Sequencer** (Rhythm) and the **Harmonic Driver** (Pitch/Harmony) to generate abstract `AudioEvent`s.

```mermaid
graph TD
    Params[MusicalParams] --> Kernel[MusicKernel]
    Kernel --> Seq[Sequencer]
    Kernel --> Har[Harmonic Driver]
    Seq --> Events[AudioEvents]
    Har --> Events
    Events --> AudioEngine[Audio Engine (Odin2/FluidSynth)]
```

## Key Modules

### 1. Harmony (`src/harmony`)
This is the most advanced module, implementing multiple sophisticated music theory concepts to generate meaningful chord progressions and melodies.

*   **Strategies**:
    *   **Basic**: Uses the Russell Circumplex model to map emotions (Valence/Arousal) to harmonic quadrants.
    *   **Neo-Riemannian**: Implements geometric chord transformations (Parallel, Leading-tone, Relative) for smooth, cinematic transitions.
    *   **Steedman Grammar**: A generative grammar approach to functional harmony (jazz/pop progressions).
    *   **Lydian Chromatic Concept**: Based on George Russell's theory, organizing tonal gravity levels.
    *   **Parsimonious Voice Leading**: Ensures smooth movement between chords by minimizing semitone distances.

*   **Context**: The `HarmonyContext` struct tracks the state:
    *   `current_chord`: The active chord.
    *   `tension` (0.0 - 1.0): Controls dissonance and complexity.
    *   `valence` (-1.0 - 1.0): Controls mood (Major/Minor bias).

### 2. Sequencer (`src/sequencer.rs`)
Handles the generation of rhythmic patterns using various algorithms.

*   **Rhythm Modes**:
    *   **Euclidean**: Uses Bjorklund's algorithm to distribute pulses evenly (classic "world music" rhythms).
    *   **Perfect Balance**: Inspired by XronoMorph, it superimposes regular polygons (Triangle, Square, etc.) to create "well-formed" rhythms.
    *   **Classic Groove**: Generates realistic drum patterns (Four-on-the-floor, Breakbeat) with ghost notes and syncopation based on tension.

*   **StepTrigger**: Each step generates a trigger containing boolean flags for instruments (`kick`, `snare`, `hat`, `bass`, `lead`) and velocity.

### 3. Fractal (`src/fractal.rs`)
Implements **Pink Noise (1/f noise)** using the Voss-McCartney algorithm.
*   Used to add "humanization" and natural variation to velocity, timing, and melodic contours, preventing the output from sounding too robotic.

### 4. Events (`src/events.rs`)
Defines the protocol for musical actions. The core outputs these events, which are then consumed by the audio backend (`harmonium_audio`).

*   `NoteOn` / `NoteOff`: Standard MIDI-like note events.
*   `ControlChange`: Parameter automation.
*   `LoadOdinPreset`: Loads patches for the Odin2 synthesizer.
*   `StartRecording` / `StopRecording`: Controls audio/MIDI export.

## Usage

The `MusicKernel` is initialized with `Sequencer` and `MusicalParams`.

```rust
use harmonium_core::{MusicKernel, Sequencer, MusicalParams};
use harmonium_core::sequencer::RhythmMode;

// 1. Setup Parameters
let params = MusicalParams::default();

// 2. Initialize Sequencer
let sequencer = Sequencer::new_with_mode(
    16, // Steps
    4,  // Pulses
    120.0, // BPM
    RhythmMode::Euclidean
);

// 3. Create Kernel
let mut kernel = MusicKernel::new(sequencer, params);

// 4. In your audio loop (e.g., every buffer processing)
let dt = 0.005; // Time delta in seconds
let events = kernel.update(dt);

for event in events {
    // Dispatch event to audio engine
    println!("Event: {:?}", event);
}
```

## Dependencies

*   **`midly`**: For MIDI message handling.
*   **`rust-music-theory`**: Primitives for intervals, scales, and chords.
*   **`rand`**: For stochastic generation processes.
*   **`serde`**: For state serialization.
