# Harmonium Web2 — Design Document

## 1. Goals

Rebuild the harmonium/web UI from scratch with:
- Clean architecture designed before any code
- Responsive, smooth visualizations (no jank)
- Clear separation of concerns (small, focused components)
- Maintainable state management (single source of truth)
- Library export for harmonium_website consumption

## 2. Architecture Overview

```
                  WASM Handle
                      |
              EngineAdapter (1 class)
              /               \
     PlaybackStore         ParamStore
     (polled per frame)    (UI-owned, push-only)
              \               /
          EngineContext (Svelte 5 context)
                      |
          +-----------+-----------+
          |           |           |
     Visualizations  Controls   Shell
     (read-only)     (write)    (lifecycle)
```

## 3. Key Design Decisions

### 3.1 State Ownership: UI is the source of truth for params

The engine executes what we tell it. We do NOT poll params back.

| Data | Owner | Direction | Frequency |
|------|-------|-----------|-----------|
| Playback position (step, measure, chord) | Engine | Engine -> UI | Every frame (~60fps) |
| Rhythm patterns (primary, secondary) | Engine | Engine -> UI | On demand (after param change) |
| New measures (score snapshots) | Engine | Engine -> UI | When generated (~every bar) |
| Note events (NoteOn/Off) | Engine | Engine -> UI | When generated |
| All parameters (BPM, A/V/D/T, rhythm, harmony...) | UI | UI -> Engine | On user interaction |

### 3.2 Two-tier polling (EngineAdapter)

Instead of 25+ getter calls per frame:

```typescript
class EngineAdapter {
  // Called every rAF frame — 4 getters only
  pollPlayback(): PlaybackState {
    return {
      currentStep: handle.get_current_step(),
      currentMeasure: handle.get_current_measure(),
      currentChord: handle.get_current_chord_name(),
      isMinorChord: handle.is_current_chord_minor(),
    };
  }

  // Called only after rhythm param changes — 8 getters
  refreshPatterns(): PatternState {
    return {
      primaryPattern: handle.get_primary_pattern(),
      primarySteps: handle.get_primary_steps(),
      primaryPulses: handle.get_primary_pulses(),
      primaryRotation: handle.get_primary_rotation(),
      secondaryPattern: handle.get_secondary_pattern(),
      secondarySteps: handle.get_secondary_steps(),
      secondaryPulses: handle.get_secondary_pulses(),
      secondaryRotation: handle.get_secondary_rotation(),
    };
  }

  // Called every frame — drain new score data
  drainMeasures(): MeasureSnapshot[] {
    const json = handle.get_new_measures_json();
    return json ? JSON.parse(json) : [];
  }

  // Called every frame — drain note events
  drainEvents(): NoteEvent[] {
    const raw = handle.get_events();
    // Parse flat [note, channel, 0, velocity, ...] array
    ...
  }
}
```

### 3.3 State management: Svelte 5 Context with fine-grained fields

One context, set at the app root, consumed by all children via `getContext()`.

```typescript
// engine-context.ts
interface EngineContext {
  // --- Playback (read-only, updated by adapter) ---
  playback: {
    currentStep: number;     // 0-based step in pattern
    currentMeasure: number;  // 1-based bar number
    currentChord: string;
    isMinorChord: boolean;
  };

  // --- Patterns (read-only, refreshed on param change) ---
  patterns: {
    primaryPattern: boolean[];
    primarySteps: number;
    primaryPulses: number;
    primaryRotation: number;
    secondaryPattern: boolean[];
    secondarySteps: number;
    secondaryPulses: number;
    secondaryRotation: number;
  };

  // --- Params (UI-owned, read/write) ---
  params: {
    // Mode
    isEmotionMode: boolean;

    // Emotion
    arousal: number;
    valence: number;
    density: number;
    tension: number;

    // Rhythm
    rhythmMode: number;  // 0=Euclidean, 1=PerfectBalance, 2=ClassicGroove
    bpm: number;
    enableRhythm: boolean;
    enableHarmony: boolean;
    enableMelody: boolean;
    enableVoicing: boolean;
    fixedKick: boolean;
    rhythmDensity: number;
    rhythmTension: number;

    // Harmony
    harmonyMode: number;  // 0=Basic, 1=Driver
    harmonyValence: number;
    harmonyTension: number;

    // Melody/Voicing
    melodySmoothness: number;
    voicingDensity: number;
    voicingTension: number;

    // Mixer
    channelGains: number[];    // [bass, lead, snare, hat]
    channelMuted: boolean[];   // [bass, lead, snare, hat]
  };

  // --- Score (append-only, for sheet music) ---
  measures: MeasureSnapshot[];

  // --- Session (static after connect) ---
  session: {
    key: string;
    scale: string;
    audioBackend: string;
  };

  // --- Connection state ---
  isPlaying: boolean;

  // --- Actions (write to engine) ---
  actions: EngineActions;
}
```

**EngineActions** wraps all bridge calls and handles the param -> engine sync:

```typescript
interface EngineActions {
  // Lifecycle
  connect(backend: string): Promise<void>;
  disconnect(): void;

  // Mode
  setEmotionMode(): void;
  setDirectMode(): void;

  // Emotion (batch)
  setEmotion(arousal: number, valence: number, density: number, tension: number): void;

  // Individual setters (update local state + send to engine)
  setArousal(v: number): void;
  setBpm(v: number): void;
  setRhythmParams(mode, steps, pulses, rotation, density, tension, ...): void;
  // ... etc

  // Recording
  startRecording(format: 'wav' | 'midi' | 'musicxml'): void;
  stopRecording(format: 'wav' | 'midi' | 'musicxml'): void;
}
```

**Key principle:** When a user drags a slider, we:
1. Update `params.arousal` immediately (UI reflects instantly)
2. Call `handle.set_arousal(value)` (engine receives command)
3. Never poll it back

### 3.4 Visualization rendering: SVG with granular reactivity

**Principle:** Separate static geometry from animated cursor.

```
EuclideanCircle component:
  ├── PatternLayer (re-renders only when pattern/steps/pulses change)
  │   ├── Background circle (static)
  │   ├── Polygon connecting active steps (derived from pattern)
  │   └── Dots at step positions (derived from pattern)
  │
  └── CursorLayer (re-renders every frame, cheap)
      └── Line from center, rotated by CSS transform
          (transition: transform 0.08s linear)
```

In Svelte 5 terms:
- `PatternLayer` depends on `patterns.primaryPattern` (changes rarely)
- `CursorLayer` depends on `playback.currentStep` (changes every beat)
- These are separate `$derived` chains that don't cross-contaminate

### 3.5 Component boundaries

```
web2/src/lib/
├── adapter/
│   ├── engine-adapter.ts      # WASM Handle wrapper (poll + command)
│   ├── engine-context.ts      # Svelte 5 context definition
│   └── types.ts               # PlaybackState, PatternState, ParamState
│
├── components/
│   ├── shell/
│   │   ├── App.svelte         # Root: context provider, lifecycle
│   │   ├── StartScreen.svelte # Backend/algorithm/harmony selection
│   │   ├── StatusBar.svelte   # Key, BPM, A/V/D/T, bar/step
│   │   └── RecordingBar.svelte # WAV/MIDI/MusicXML buttons
│   │
│   ├── viz/
│   │   ├── EuclideanCircle.svelte   # SVG rhythm circle (pattern + cursor)
│   │   ├── RhythmPanel.svelte       # 1-2 circles + mode badge
│   │   ├── ChordDisplay.svelte      # Current chord + progression
│   │   ├── MorphPlane.svelte        # 2D draggable point plane (reusable)
│   │   ├── EmotionPlane.svelte      # Valence x Arousal morph plane
│   │   ├── MusicPlane.svelte        # Density x Tension morph plane
│   │   └── ScoreView.svelte         # VexFlow sheet music (future)
│   │
│   ├── controls/
│   │   ├── ControlPanel.svelte      # Mode toggle + conditional children
│   │   ├── EmotionSliders.svelte    # A/V/D/T sliders
│   │   ├── RhythmControls.svelte    # Mode + params per algorithm
│   │   ├── HarmonyControls.svelte   # Basic/Driver + valence/tension
│   │   ├── MelodyControls.svelte    # Smoothness + density
│   │   └── ChannelMixer.svelte      # 4-ch mute + gain
│   │
│   └── ui/
│       ├── Slider.svelte            # Styled range input
│       ├── ToggleGroup.svelte       # Radio-like buttons
│       └── Card.svelte              # Container card
│
├── stores/                          # (empty — all state in context)
├── index.ts                         # Public library exports
└── utils.ts                         # cn() helper
```

**Key differences from web1:**
- `App.svelte` (~50 lines) only creates context + renders layout
- `StartScreen.svelte` extracts the pre-play configuration UI
- `StatusBar.svelte` extracts the live info display
- `RecordingBar.svelte` extracts recording controls
- `EmotionPlane` and `MusicPlane` are standalone (not wrapped in MorphVisualization)
- No `HarmoniumDemo.svelte` monolith

## 4. Data Available from Engine

### 4.1 Playback state (polled per frame — 4 getters)

| Field | Getter | Type |
|-------|--------|------|
| currentStep | `get_current_step()` | usize (0-based in pattern) |
| currentMeasure | `get_current_measure()` | usize (1-based bar) |
| currentChord | `get_current_chord_name()` | String ("Imaj7", "iv") |
| isMinorChord | `is_current_chord_minor()` | bool |

### 4.2 Progression info (refreshed when measure changes — 2 getters)

| Field | Getter | Type |
|-------|--------|------|
| progressionName | `get_progression_name()` | String ("Hopeful", "Dark") |
| progressionLength | `get_progression_length()` | usize (bars per cycle) |

Checked when `currentMeasure` changes (bar-level frequency).

### 4.3 Patterns (refreshed on demand — 8 getters)

| Field | Getter | Type |
|-------|--------|------|
| primaryPattern | `get_primary_pattern()` | Vec<u8> (0/1) |
| primarySteps | `get_primary_steps()` | usize |
| primaryPulses | `get_primary_pulses()` | usize |
| primaryRotation | `get_primary_rotation()` | usize |
| secondaryPattern | `get_secondary_pattern()` | Vec<u8> |
| secondarySteps | `get_secondary_steps()` | usize |
| secondaryPulses | `get_secondary_pulses()` | usize |
| secondaryRotation | `get_secondary_rotation()` | usize |

### 4.4 Score data (drained per frame)

`get_new_measures_json()` returns `MeasureSnapshot[]`:

```typescript
interface MeasureSnapshot {
  index: number;              // 1-based bar number
  tempo: number;              // BPM
  time_sig_numerator: number;
  time_sig_denominator: number;
  steps: number;              // Steps in this bar
  chord_name: string;
  chord_root_offset: number;  // Semitones from key
  chord_is_minor: boolean;
  composition_bpm: number;
  notes: NoteSnapshot[];
}

interface NoteSnapshot {
  track: number;          // 0=Bass, 1=Lead, 2=Snare, 3=Hat
  pitch: number;          // MIDI 0-127
  start_step: number;     // 0-based within bar
  duration_steps: number; // How long (0 for percussion)
  velocity: number;       // 0-127
}
```

### 4.5 Note events (drained per frame)

`get_events()` returns flat `Uint32Array`: `[note, channel, 0, velocity, ...]`

### 4.6 What's missing for perfect visualization

**Currently available but not exposed via WASM Handle:**
- `current_beat` (beat within bar) — available in EngineReport but not as a getter
- `time_signature` — available in EngineReport but not as a getter
- `progression_length` — exposed as getter, good
- `rhythm_mode` — exposed as getter via `get_direct_rhythm_mode()`

**Might want to add:**
- `get_current_beat()` — for beat-level highlighting in score view
- `get_time_signature_json()` — for score rendering
- `get_playback_state_json()` — single call returning {step, measure, beat, chord, isMinor} as JSON (reduces 4 getter calls to 1 JSON parse)

## 5. Library Contract (harmonium_website compatibility)

The website currently imports these from harmonium-web:

| Component | Used In | Must Keep |
|-----------|---------|-----------|
| `WasmBridge` | DemoWidget.svelte | Yes (or compatible replacement) |
| `HarmoniumDemo` | Demo page | Yes (self-contained composite) |
| `EuclideanCircle` | Explain page | Yes (standalone, static props) |
| `ChordProgression` | Explain page | Yes (standalone, static props) |
| `MorphPlane` | Explain page | Yes (standalone, interactive) |
| `RhythmVisualizer` | DemoWidget.svelte | Yes (standalone, static props) |

**Strategy:** web2 exports the same component names with compatible props. The website doesn't need to change its imports — just point the package.json dep to `../web2`.

## 6. VST Build

Same dual-build approach:
- **Web mode:** SvelteKit static + library export (svelte-package)
- **VST mode:** Standalone Vite build → single inlined HTML (vite.vst.config.ts)

VST uses `VstBridge` instead of `WasmBridge` but same context/components.

## 7. Implementation Phases

### Phase 1: Foundation
- [ ] SvelteKit project scaffold (package.json, svelte.config, vite.config, tailwind)
- [ ] `adapter/types.ts` — all TypeScript interfaces
- [ ] `adapter/engine-adapter.ts` — WASM Handle wrapper
- [ ] `adapter/engine-context.ts` — Svelte 5 context
- [ ] `shell/App.svelte` — context provider + rAF loop
- [ ] `shell/StartScreen.svelte` — backend/algorithm selection
- [ ] Verify: engine starts, context populates, playback state updates

### Phase 2: Visualizations
- [ ] `viz/EuclideanCircle.svelte` — SVG with separated pattern/cursor layers
- [ ] `viz/RhythmPanel.svelte` — wraps 1-2 circles
- [ ] `viz/ChordDisplay.svelte` — chord progression
- [ ] `viz/MorphPlane.svelte` — reusable 2D plane
- [ ] `viz/EmotionPlane.svelte` — V x A plane
- [ ] `viz/MusicPlane.svelte` — D x T plane
- [ ] Verify: all visualizations animate smoothly

### Phase 3: Controls
- [ ] `ui/Slider.svelte`, `ui/ToggleGroup.svelte`, `ui/Card.svelte`
- [ ] `controls/EmotionSliders.svelte`
- [ ] `controls/RhythmControls.svelte`
- [ ] `controls/HarmonyControls.svelte`
- [ ] `controls/MelodyControls.svelte`
- [ ] `controls/ChannelMixer.svelte`
- [ ] `controls/ControlPanel.svelte` — mode toggle + layout
- [ ] Verify: all controls affect engine correctly

### Phase 4: Polish
- [ ] `shell/StatusBar.svelte` — live info
- [ ] `shell/RecordingBar.svelte` — WAV/MIDI/MusicXML
- [ ] Library export (`index.ts`, svelte-package)
- [ ] Website compatibility test
- [ ] VST build configuration

### Phase 5: Score View (future)
- [ ] `viz/ScoreView.svelte` — VexFlow integration using MeasureSnapshot data
