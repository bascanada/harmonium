# 🎛️ Harmonium - Dual Control Implementation

## Architecture Overview

Harmonium now supports **two distinct control paradigms** depending on the context:

### 🖥️ Native Application (main.rs)
**AI-Driven Morphing Simulation**
- Automatic parameter changes every 5 seconds
- Simulates future AI/text analysis control
- Demonstrates autonomous morphing capabilities
- Perfect for testing and development

### 🌐 Web Application (lib.rs + Svelte UI)
**Manual Slider Control**
- Real-time parameter tweaking via UI sliders
- Direct user interaction
- Instant feedback on parameter changes
- Smooth morphing as sliders are moved

---

## Native Implementation (main.rs)

### AI Simulator Thread
```rust
// Spawns a background thread that periodically changes parameters
thread::spawn(move || {
    loop {
        thread::sleep(Duration::from_secs(5));
        let mut params = controller_state.lock().unwrap();
        
        params.bpm = rng.gen_range(70.0..160.0);
        params.density = rng.gen_range(0.15..0.95);
        params.tension = rng.gen_range(0.0..1.0);
        params.arousal = rng.gen_range(0.2..0.9);
    }
});
```

### Console Output
```
🎵 Harmonium - Procedural Music Generator
🧠 State Management + Morphing Engine activé
🤖 Simulateur d'IA démarré (changements toutes les 5s)

🎭 ACTION CHANGE: BPM 140.5 | Density 0.71 | Tension 0.35 | Arousal 0.69
🔄 Morphing Rhythm -> Pulses: 5 | BPM: 115.0
🔄 Morphing Rhythm -> Pulses: 6 | BPM: 128.7
```

---

## Web Implementation (lib.rs + WASM)

### Exposed API Methods

#### Individual Parameter Setters
```typescript
handle.set_bpm(120.0);        // Set tempo (70.0 - 180.0)
handle.set_density(0.7);      // Set rhythm density (0.0 - 1.0)
handle.set_tension(0.5);      // Set harmonic tension (0.0 - 1.0)
handle.set_arousal(0.8);      // Set intensity (0.0 - 1.0)
```

#### Batch Update
```typescript
handle.set_params(120.0, 0.7, 0.5, 0.8); // Update all at once
```

#### Getters
```typescript
const bpm = handle.get_target_bpm();
const density = handle.get_target_density();
const tension = handle.get_target_tension();
const arousal = handle.get_target_arousal();
```

### Svelte UI Implementation

#### Reactive Sliders
```svelte
<script lang="ts">
    let bpm = 100;
    let density = 0.5;
    let tension = 0.3;
    let arousal = 0.5;

    function updateParams() {
        if (handle && isPlaying) {
            handle.set_params(bpm, density, tension, arousal);
        }
    }
</script>

<input 
    type="range" 
    min="70" 
    max="180" 
    bind:value={bpm}
    oninput={updateParams}
/>
```

### Live Controls UI

The web interface features **4 real-time sliders**:

1. **🎯 BPM (Tempo)** - 70 to 180
   - Slow (70) → Fast (180)
   - Controls playback speed
   
2. **🥁 Density (Rhythm)** - 0.0 to 1.0
   - Sparse (0.0) → Dense (1.0)
   - Affects Euclidean rhythm pulses (1-12 out of 16 steps)
   
3. **⚡ Tension (Harmony)** - 0.0 to 1.0
   - Consonant (0.0) → Dissonant (1.0)
   - [Future: Will control filter cutoff/resonance]
   
4. **🔥 Arousal (Intensity)** - 0.0 to 1.0
   - Calm (0.0) → Intense (1.0)
   - [Future: Will control distortion/saturation]

---

## Technical Implementation Details

### Thread-Safe State Management

Both implementations share the same core architecture:

```rust
// Shared state between audio thread and control thread
pub struct EngineParams {
    pub bpm: f32,
    pub density: f32,
    pub tension: f32,
    pub arousal: f32,
}

// Wrapped in Arc<Mutex<>> for thread safety
let target_state = Arc::new(Mutex::new(EngineParams::default()));
```

### Morphing Engine

The audio engine **smoothly interpolates** towards target values:

```rust
// Exponential convergence (no jumps!)
self.current_state.bpm += (target.bpm - self.current_state.bpm) * 0.05;
self.current_state.density += (target.density - self.current_state.density) * 0.02;
```

**Result**: Organic transitions regardless of control source (AI or UI)

### Input Validation

All web setters include **clamping**:

```rust
pub fn set_bpm(&mut self, bpm: f32) {
    if let Ok(mut state) = self.target_state.lock() {
        state.bpm = bpm.clamp(70.0, 180.0); // Prevent invalid values
    }
}
```

---

## Usage Examples

### Native (AI Control)
```bash
cargo run
# Music auto-morphs every 5 seconds
# Press Ctrl+C to stop
```

### Web (Manual Control)
```bash
# Build WASM
cargo build --lib --target wasm32-unknown-unknown
wasm-bindgen --out-dir pkg --target web target/wasm32-unknown-unknown/debug/harmonium.wasm

# Start dev server
cd web
npm run dev
```

Then in browser:
1. Click "▶ Start Music"
2. Adjust sliders in real-time
3. Observe smooth morphing as you move controls

---

## Future Enhancements

### Native Side
- Replace simulator with **real AI** (ONNX Runtime)
- Text → Emotion analysis → Parameter mapping
- Multiple AI models (GPT, BERT, custom)

### Web Side
- **Preset buttons** ("Calm", "Intense", "Chaotic")
- **Waveform visualization** reacting to parameters
- **Recording/Export** of morphing sequences
- **MIDI input** for parameter control
- **WebSocket** for remote control (mobile app?)

### Shared
- **DSP expansion**: Dynamic filters, distortion, reverb
- **Harmonic morphing**: Scale transitions (Major ↔ Minor)
- **Multi-voice** polyphony with per-voice parameters

---

## API Comparison

| Feature | Native (AI) | Web (Manual) |
|---------|-------------|--------------|
| Control Source | Automated thread | User sliders |
| Update Frequency | Every 5s | Real-time |
| Randomness | Yes | User choice |
| Morphing | ✅ | ✅ |
| Audio Thread | ✅ | ✅ |
| State Management | Arc<Mutex> | Arc<Mutex> |
| Parameter Range | Random | Validated |

Both use **identical morphing logic** - only the control source differs!

---

## Development Notes

### Adding New Parameters

1. **Add to `EngineParams`** (engine.rs):
   ```rust
   pub struct EngineParams {
       // ... existing ...
       pub reverb: f32,  // New parameter
   }
   ```

2. **Add to `CurrentState`** (engine.rs)

3. **Implement morphing** in `process()`:
   ```rust
   self.current_state.reverb += (target.reverb - self.current_state.reverb) * 0.05;
   ```

4. **Expose to web** (lib.rs):
   ```rust
   pub fn set_reverb(&mut self, reverb: f32) {
       if let Ok(mut state) = self.target_state.lock() {
           state.reverb = reverb.clamp(0.0, 1.0);
       }
   }
   ```

5. **Add UI slider** (+page.svelte)

### Testing Strategy

**Native**:
```bash
cargo run
# Observe console logs
# Listen to morphing
```

**Web**:
```bash
# Terminal 1
cargo build --lib --target wasm32-unknown-unknown --release
wasm-bindgen --out-dir pkg --target web target/wasm32-unknown-unknown/release/harmonium.wasm

# Terminal 2
cd web && npm run dev
# Open browser, test sliders
```

---

**Status**: ✅ Dual Control Implementation Complete  
**Date**: 30 décembre 2025  
**Ready for**: AI Integration / DSP Expansion
