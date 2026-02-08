# Harmony System

The Harmonium Harmony system is designed to generate musically coherent progressions that adapt to emotional input. It operates using three distinct strategies orchestrated by the `HarmonicDriver`.

## Core Components

### 1. HarmonicDriver
The main orchestrator that chooses the best strategy based on the current **Tension** and **Valence**.
- **Low Tension (< 0.5)**: Uses **Steedman Grammar** for functional, narrative progressions.
- **High Tension (> 0.7)**:
    - **Triads**: Uses **Neo-Riemannian (PLR)** transformations.
    - **Tetrads (7th chords)**: Uses **Parsimonious Voice-Leading**.
- **Transition Zone (0.5 - 0.7)**: Blends strategies using probabilistic weights and hysteresis to avoid "jittery" changes.

### 2. Steedman Grammar
Based on Mark Steedman's work on combinatorial grammars for jazz.
- Generates progressions by recursively expanding symbols (e.g., `V -> ii-V`, `I -> vi-I`).
- Supports different styles: `Jazz`, `Pop`, `Classical`, `Contemporary`.
- Maintains internal state to ensure long-term coherence.

### 3. Neo-Riemannian Engine
Focuses on geometric transformations between chords rather than functional roles.
- **P (Parallel)**: C Major <-> C Minor.
- **L (Leittonwechsel)**: C Major <-> E Minor.
- **R (Relative)**: C Major <-> A Minor.
- Ideal for cinematic or ambient textures where traditional functional harmony is too restrictive.

### 4. Lydian Chromatic Concept (LCC)
Acts as a vertical filter over all generated harmony.
- Derived from George Russell's theory.
- Maps **Tension** to specific "Lydian Scales" (from Lydian to Chromatic).
- Ensures that melodies and voicings are always "tonally gravitating" towards the current chord and global key.

## Data Structures

### `Chord`
Represents a musical chord with a `PitchClass` (0-11) and `ChordType` (Major, Minor, Dominant7, etc.).

### `HarmonyDecision`
The output of a strategy, containing:
- `next_chord`: The chord to play.
- `transition_type`: How we got there (Functional, Pivot, Chromatic).
- `suggested_scale`: The LCC scale to use for melody generation.

## Implementation Details
- **Taboo List**: The driver maintains a sliding window of the last two chords to prevent `A->B->A` loops, unless resolving to the Tonic (`I`).
- **Cadential Resolution**: If tension drops dramatically, the system forces a `V -> I` or `IV -> I` resolution to provide a sense of emotional relief.
