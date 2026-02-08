# Harmonium Host

`harmonium_host` is the runtime environment that bridges the pure logic of `harmonium_core` with the audio generation of `harmonium_audio`. It provides the platform-specific implementations for running Harmonium as a **Standalone Application**, **VST Plugin**, or **Web Assembly (WASM)** module.

## Architecture

The central component is the **`HarmoniumEngine`** (`src/engine.rs`). It acts as the "glue" that synchronizes the musical logic with the audio callback.

```mermaid
graph TD
    CLI/DAW/Web --> Inputs[Triple Buffer (Params)]
    Inputs --> Engine[HarmoniumEngine]
    Engine --> Core[MusicKernel (Sequencer/Harmony)]
    Engine --> Audio[AudioRenderer (Odin2/Synth)]
    Audio --> Output[Audio/MIDI Output]
    Engine --> Queue[Ring Buffer (Events)]
    Queue --> UI[Visualization/GUI]
```

### Concurrency Model
To ensure glitch-free audio performance, the host employs a strict lock-free architecture for communication between the UI/Control thread and the Audio thread:

*   **UI → Audio**: A **Triple Buffer** is used to pass `EngineParams`. This allows the UI to write updates atomically without blocking the audio thread.
*   **Audio → UI**: **Ring Buffers (SPSC)** are used to stream `VisualizationEvent`s and `HarmonyState` back to the UI for real-time feedback.

## Build Targets

### 1. Standalone CLI
*   **Entry Point**: `src/main.rs`
*   **Audio Backend**: Uses `cpal` for cross-platform audio output.
*   **Features**:
    *   Real-time audio playback.
    *   OSC (Open Sound Control) server for remote control (port 8080).
    *   File recording (WAV, MIDI, MusicXML).
    *   Command-line arguments for configuration.

```bash
# Run standalone with default settings
cargo run --release --bin harmonium

# Record to WAV for 30 seconds
cargo run --release --bin harmonium -- --record-wav output.wav --duration 30
```

### 2. VST3 / CLAP Plugin
*   **Entry Point**: `src/vst_plugin.rs`
*   **Framework**: Built using `nih_plug`.
*   **Functionality**: Acts as a MIDI Generator plugin. It syncs with the DAW's transport and outputs MIDI notes to drive other virtual instruments.
*   **GUI**: Features a webview-based editor (`src/vst_gui`) that embeds the Svelte frontend.

```bash
# Build VST3 plugin
cargo xtask bundle harmonium --release
```

### 3. Web Assembly (WASM)
*   **Entry Point**: `src/lib.rs` (wasm-bindgen exports)
*   **Usage**: Compiled to WASM to run directly in the browser, powering the web interface.
*   **Audio**: Uses `WebAudioContext` (via `cpal` wasm backend or custom implementation).

## Key Components

*   **`HarmoniumEngine`**: Orchestrates the `tick()` loop (logic update) and `process_buffer()` (audio generation). It manages the "Emotional Morphing" interpolation.
*   **`EmotionMapper`**: Translates high-level emotional parameters (Valence, Arousal) into low-level musical parameters (BPM, Scale, Density, Tension).
*   **`FontQueue`**: A thread-safe queue for loading SoundFonts dynamically.

## Features

*   **Multiple Backends**: Hot-swappable support for `Odin2` (Synthesizer), `FundSP` (DSP Graph), or MIDI output.
*   **Graceful Shutdown**: Handles Ctrl+C signals to ensure recordings are finalized and saved correctly before exiting.
*   **Simulation Mode**: Can run an internal "AI Simulator" thread that randomly mutates emotional parameters to test the morphing engine.
