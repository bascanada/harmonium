# Harmonium Web UI

This directory contains the SvelteKit application that serves as the user interface for Harmonium. It is designed to run in two distinct environments:

1.  **Web Playground** (WASM): Runs entirely in the browser using WebAssembly. Audio is generated via `cpal` and the Web Audio API.
2.  **VST GUI** (WebView): Runs inside a DAW plugin window (using `nih-plug-webview`). It communicates with the Rust backend via IPC, acting purely as a controller and visualizer.

## Architecture

The core design principle is the **Bridge Pattern**, which abstracts the underlying communication mechanism from the UI components.

```mermaid
graph TD
    UI[Svelte Components] --> Store[Engine State Store]
    Store --> Bridge{HarmoniumBridge}
    
    subgraph "Web Mode (WASM)"
        Bridge --> WasmBridge
        WasmBridge --> WASM[harmonium.wasm]
        WASM --> WebAudio[Web Audio API]
    end
    
    subgraph "VST Mode (WebView)"
        Bridge --> VstBridge
        VstBridge --> IPC[nih-plug-webview IPC]
        IPC --> RustHost[Rust Backend (DAW)]
    end
```

### Key Modules

*   **`src/lib/bridge`**: The communication layer.
    *   `HarmoniumBridge` (Interface): Defines common methods (`setArousal`, `connect`, `subscribe`).
    *   `WasmBridge`: Implements the bridge for the browser. Imports `harmonium.js` (wasm-bindgen), initializes audio, and uses polling (`requestAnimationFrame`) to sync state.
    *   `VstBridge`: Implements the bridge for the VST. Uses `window.sendToPlugin` and `window.onPluginMessage` to sync state with the C++ host.
*   **`src/lib/stores`**: Svelte stores for reactive state management.
    *   `engineState`: Holds the full state of the audio engine (BPM, current chord, visualization data).
    *   `bridge`: Holds the active bridge instance.
*   **`src/components`**:
    *   **Visualizations**: `RhythmVisualizer` (Polygons/Euclidean), `MorphVisualization` (Emotion Plane), `ChordProgression`.
    *   **Controls**: `EmotionalControls` (Valence/Arousal sliders), `TechnicalControls` (Direct parameter tweaking), `ControlPanel` (Mode switcher).
*   **`src/routes`**:
    *   `/`: Renders the project's root `README.md` as a documentation page.
    *   `/test`: The main application playground ("Try Harmonium").

## Features

*   **Dual Control Modes**:
    *   **Emotional**: High-level control using Valence (Mood), Arousal (Energy), Tension, and Density. The AI/Mapper translates these into musical parameters.
    *   **Technical**: Direct control over BPM, Rhythm Algorithms, Poly-rhythm steps, and Harmony rules.
*   **Real-time Visualization**:
    *   See the active chord progression and measure position.
    *   Visualize the rhythmic geometry (Euclidean circles or Perfect Balance polygons).
    *   Track the emotional morphing trajectory.
*   **Audio Backend Selection** (Web Mode only):
    *   **Odin2**: High-quality analog modeling synthesis.
    *   **FundSP**: Lightweight functional DSP synthesis.
*   **Recording** (Web Mode only):
    *   Export sessions to WAV, MIDI, or MusicXML directly from the browser.

## Development

### Prerequisites
*   Node.js & npm
*   Rust toolchain (for building the WASM package)

### Setup
1.  Build the WASM package first:
    ```bash
    # From project root
    cargo xtask build-wasm
    ```
    This generates the `pkg` directory which is a dependency of the web app.

2.  Install web dependencies:
    ```bash
    cd web
    npm install
    ```

### Running

**Web Mode (Dev Server):**
```bash
npm run dev
```

**VST Mode (Build):**
This builds the frontend into `dist` for embedding into the VST.
```bash
npm run build:vst
```
Then rebuild the Rust VST using `cargo xtask bundle harmonium`.