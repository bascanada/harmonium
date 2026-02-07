# Harmonium Project Overview

Harmonium is a procedural music generation system that bridges the gap between emotional intent and technical musical execution. It uses a multi-layered approach to generate rhythm, harmony, and melody in real-time.

## Architecture

The project is structured as a Rust workspace with several specialized crates:

### 1. `harmonium_core`
The brain of the system.
- **Harmony**: Implements Steedman Grammars, Neo-Riemannian PLR transformations, and the Lydian Chromatic Concept (LCC).
- **Sequencer**: Handles rhythm generation using Euclidean algorithms, "Perfect Balance" (XronoMorph style), and realistic grooves.
- **Params**: Defines the data contracts between the UI and the engine.

### 2. `harmonium_audio`
The rendering engine.
- **Backend**: Abstractions for different synthesis engines (FundSP + Oxisynth, Odin2).
- **Voice Manager**: Manages MIDI routing and instrument voices.
- **Real-time Safety**: Includes a custom allocator to detect illegal allocations in the audio thread.

### 3. `harmonium_host`
The bridge between the library and the outside world.
- **Engine**: Orchestrates the core logic and audio rendering.
- **Standalone**: Provides a CLI host with OSC support and recording capabilities.
- **VST/CLAP**: (In progress) Plugin interfaces for DAWs.

### 4. `harmonium_ai`
The emotional intelligence layer.
- **Emotion Mapper**: Maps high-level emotional coordinates (Arousal, Valence) to technical musical parameters (BPM, Scale, Density).
- **Semantic Engine**: (If enabled) Uses ML models to predict musical parameters from text labels.

## Current State
- **Validation**: The codebase has been cleaned of dead code and common Rust anti-patterns.
- **Stability**: Fixed synchronization issues between UI and Audio threads using lock-free triple buffering and SPSC queues.
- **Export**: Supports recording to WAV, MIDI, and MusicXML.
