# Harmonium Communication Layer & Frontend Rebuild Plan

## Overview

**Problem**: Communication between engine and UI is broken, with three competing synchronization mechanisms (triple buffer, SPSC rings, mutex) creating state conflicts. Technical vs Emotion mode switching destroys state and controls don't work.

**Solution**: Unified command-driven architecture with:
- Single lock-free command/report queue system
- EmotionMapper moved out of audio thread into controller layer
- CLI-first validation approach
- Rebuilt frontend preserving working visualizations
- Unified interface for Web and VST

## Architecture Design

### Core Components

**1. EngineCommand Enum** (`harmonium_core/src/command.rs` - NEW)

Complete command interface covering all 40+ controllable parameters:

```rust
pub enum EngineCommand {
    // Global: SetBpm(f32), SetMasterVolume(f32), SetTimeSignature
    // Modules: EnableRhythm(bool), EnableHarmony(bool), EnableMelody(bool), EnableVoicing(bool)
    // Rhythm: SetRhythmMode, SetRhythmSteps, SetRhythmPulses, SetRhythmRotation, SetRhythmDensity, SetRhythmTension
    // Harmony: SetHarmonyMode, SetHarmonyStrategy, SetHarmonyTension, SetHarmonyValence, SetKeyRoot
    // Melody: SetMelodySmoothness, SetMelodyOctave, SetVoicingDensity, SetVoicingTension
    // Mixer: SetChannelGain, SetChannelMute, SetChannelRoute, SetVelocityBase
    // Recording: StartRecording, StopRecording
    // Mode: UseEmotionMode, UseDirectMode, SetEmotionParams
    // Utility: GetState, Reset
}
```

**2. EngineReport Struct** (`harmonium_core/src/report.rs` - NEW)

Unified state reporting (replaces HarmonyState + VisualizationEvent):

```rust
pub struct EngineReport {
    // Timing: current_bar, current_beat, current_step, time_signature
    // Harmony: current_chord, chord_root_offset, chord_is_minor, progression_name
    // Rhythm: primary/secondary steps/pulses/rotation/pattern
    // Notes: Vec<NoteEvent> (pre-allocated)
    // Params: musical_params (echoed back)
    // Session: session_key, session_scale
}
```

Uses fixed-size arrays (`[bool; 192]`, `ArrayString<64>`) to avoid allocations.

**3. HarmoniumController API** (`harmonium_core/src/controller.rs` - NEW)

Public interface - the ONLY way to control the engine:

```rust
pub struct HarmoniumController {
    command_tx: rtrb::Producer<EngineCommand>,
    report_rx: rtrb::Consumer<EngineReport>,
    emotion_mapper: Option<EmotionMapper>,  // Runs HERE, not in audio thread
    control_mode: ControlMode,
    cached_state: Option<EngineReport>,
}

impl HarmoniumController {
    pub fn send(&mut self, cmd: EngineCommand) -> Result<(), CommandError>;
    pub fn poll_reports(&mut self) -> Vec<EngineReport>;
    pub fn get_state(&self) -> Option<&EngineReport>;

    // Emotion mode (EmotionMapper translates to commands)
    pub fn set_emotions(&mut self, arousal: f32, valence: f32, density: f32, tension: f32);

    // Direct mode (bypass EmotionMapper)
    pub fn set_bpm(&mut self, bpm: f32);
    pub fn set_rhythm_density(&mut self, density: f32);
    // ... all direct setters
}
```

**4. Transport Mechanism**

**DELETE from `harmonium_host/src/engine.rs`**:
- Line 29: `target_params: Output<EngineParams>` (triple buffer)
- Line 31: `harmony_state_tx: rtrb::Producer<HarmonyState>` (SPSC ring)
- Line 32: `event_queue_tx: rtrb::Producer<VisualizationEvent>` (SPSC ring)
- Line 76: `control_mode: Arc<Mutex<ControlMode>>` (mutex)

**ADD to `harmonium_host/src/engine.rs`**:
```rust
// Lock-free command queue (UI→Audio, 1024 slots)
command_rx: rtrb::Consumer<EngineCommand>,

// Lock-free report queue (Audio→UI, 256 slots)
report_tx: rtrb::Producer<EngineReport>,
```

**Communication Flow**:
```
UI/CLI/VST → HarmoniumController → EngineCommand (SPSC queue) → Audio Thread
                                                                       ↓
UI/CLI/VST ← HarmoniumController ← EngineReport (SPSC queue) ← Audio Thread
```

Unidirectional, lock-free, allocation-free, ordered delivery.

### EmotionMapper Integration

**Current Problem**: EmotionMapper runs in `update_controls()` (audio thread, line 311-321 of engine.rs), overwrites MusicalParams every frame.

**Solution**: Move to `HarmoniumController` (main thread):

```rust
// In HarmoniumController::set_emotions()
pub fn set_emotions(&mut self, arousal: f32, valence: f32, density: f32, tension: f32) {
    let emotions = EngineParams { arousal, valence, density, tension, /* ... */ };
    let musical_params = self.emotion_mapper.map(&emotions);

    // Send commands for changed params
    if musical_params.bpm != self.cached_state.bpm {
        self.send(EngineCommand::SetBpm(musical_params.bpm));
    }
    if musical_params.rhythm_density != self.cached_state.rhythm_density {
        self.send(EngineCommand::SetRhythmDensity(musical_params.rhythm_density));
    }
    // ... all other params
}
```

Mode switching:
- Emotion mode: UI sliders (Arousal, Valence, Density, Tension) → `set_emotions()` → EmotionMapper → Commands
- Direct mode: UI sliders (BPM, Rhythm Density, etc.) → Direct commands

## Implementation Phases

### Phase 1: Core Command Infrastructure (Week 1)

**Goal**: Replace communication layer without breaking existing audio engine.

**Tasks**:

1. **Create command/report types** (2 days)
   - `harmonium_core/src/command.rs`: Define all EngineCommand variants
   - `harmonium_core/src/report.rs`: Define EngineReport struct
   - Add Serialize/Deserialize derives
   - Write serde roundtrip tests

2. **Create HarmoniumController** (2 days)
   - `harmonium_core/src/controller.rs`: Implement controller API
   - Command queue (SPSC 1024 slots)
   - Report queue (SPSC 256 slots)
   - EmotionMapper integration
   - Convenience methods
   - Integration tests

3. **Modify HarmoniumEngine** (3 days)
   - `harmonium_host/src/engine.rs`:
     - Remove triple buffer, SPSC rings, mutex (lines 29, 31, 32, 76)
     - Add command_rx and report_tx
     - Implement `process_commands()` method (drain queue, apply commands)
     - Implement `send_report()` method (generate and push report)
     - Update `tick()` to generate reports
   - Verify no allocations in audio thread (rt_check)

**Verification**:
- [ ] Command serde roundtrip works
- [ ] Report generation passes rt_check (no allocations)
- [ ] Command queue delivers all commands in order
- [ ] Report queue delivers all reports in order
- [ ] No audio dropouts during command processing

### Phase 2: CLI Implementation (Week 2)

**Goal**: Interactive REPL proving all engine control works before UI rebuild.

**Tasks**:

1. **Create CLI crate** (1 day)
   - `harmonium_cli/Cargo.toml`: Add dependencies (rustyline, clap, colored)
   - Set up project structure

2. **Implement command parser** (2 days)
   - `harmonium_cli/src/parser.rs`: Parse all command variants
   - Error handling
   - Tab completion
   - Parser tests

3. **Implement REPL loop** (2 days)
   - `harmonium_cli/src/repl.rs`:
     - Initialize engine + controller
     - Read-eval-print loop
     - Real-time state display
     - Command history

4. **Add help system** (1 day)
   - `harmonium_cli/src/help.rs`: Generate help from command definitions

5. **Testing** (1 day)
   - Manual testing of all parameters
   - Record session logs
   - Verify state changes

**Example CLI Commands**:
```bash
$ harmonium-cli
> start
[Engine] Started | BPM: 120 | Key: C major

> set bpm 140
[OK] BPM set to 140.0

> set rhythm mode perfect_balance
[OK] Rhythm mode set to PerfectBalance

> emotion arousal 0.9 valence 0.5 density 0.7 tension 0.6
[OK] Emotion mode activated
[Mapper] Arousal 0.9 → BPM 169.0

> direct
[OK] Direct mode activated

> show state
[State] BPM: 140.0 | Chord: Imaj7 | Pattern: [x . . x . x . . x . x . . x . .]

> quit
```

**Verification**:
- [ ] All 40+ parameters can be set and verified
- [ ] EmotionMapper works correctly (emotion → musical params)
- [ ] Direct mode works (bypass EmotionMapper)
- [ ] Mode switching (emotion ↔ direct) preserves state
- [ ] Real-time state updates show parameter changes
- [ ] Recording (WAV/MIDI/MusicXML) works
- [ ] Channel muting and gain control works
- [ ] Rhythm/harmony mode switching works
- [ ] No audio dropouts during parameter changes

### Phase 3: Frontend Rebuild (Week 3-4)

**Goal**: Rebuild Svelte UI with unified command interface.

#### Phase 3.1: Bridge Layer Redesign (3 days)

**DELETE**:
- Current bridge implementation that uses `sendCommand()` abstraction with fragmented state

**PRESERVE**:
- Bridge pattern (abstraction for WASM vs VST)
- Factory detection logic in `web/src/lib/bridge/index.ts`

**CREATE**: `web/src/lib/bridge/unified-bridge.ts`

```typescript
export class UnifiedBridge implements HarmoniumBridge {
    private commandQueue: EngineCommand[] = [];
    private reportQueue: EngineReport[] = [];
    private subscribers: ((report: EngineReport) => void)[] = [];

    constructor(private backend: WasmBackend | VstBackend) {}

    sendCommand(cmd: EngineCommand): void {
        this.commandQueue.push(cmd);
        this.backend.sendCommand(cmd);
    }

    pollReports(): EngineReport[] {
        const reports = this.backend.receiveReports();
        reports.forEach(r => {
            this.reportQueue.push(r);
            this.subscribers.forEach(cb => cb(r));
        });
        return reports;
    }

    // High-level API
    setBpm(bpm: number): void {
        this.sendCommand({ type: 'SetBpm', value: bpm });
    }

    setEmotions(arousal, valence, density, tension): void {
        this.sendCommand({ type: 'SetEmotionParams', arousal, valence, density, tension });
    }

    useEmotionMode(): void {
        this.sendCommand({ type: 'UseEmotionMode' });
    }

    useDirectMode(): void {
        this.sendCommand({ type: 'UseDirectMode' });
    }
}
```

#### Phase 3.2: Component Hierarchy (4 days)

**DELETE**:
- `web/src/lib/components/controls/EmotionalControls.svelte` (broken emotion mapper in UI)
- `web/src/lib/components/controls/TechnicalControls.svelte` (duplicates direct controls)
- Old state management code

**PRESERVE** (rebind to new data):
- `web/src/lib/components/visualizations/RhythmVisualizer.svelte`
- `web/src/lib/components/visualizations/EuclideanCircle.svelte`
- `web/src/lib/components/visualizations/ChordProgression.svelte`
- `web/src/lib/components/controls/ChannelMixer.svelte`

**CREATE**:

1. **`web/src/lib/components/controls/EngineControlPanel.svelte`** (top-level)
   - Imports: RhythmSection, HarmonySection, MelodySection, MixerSection, ModeToggle
   - Binds all to `latestReport` from store

2. **`web/src/lib/components/controls/ModeToggle.svelte`**
   - Toggle button: Emotion ↔ Technical
   - Calls `bridge.useEmotionMode()` or `bridge.useDirectMode()`

3. **`web/src/lib/components/controls/RhythmSection.svelte`**
   - Emotion mode: Density/Tension sliders
   - Direct mode: Mode/Steps/Pulses/Rotation sliders
   - Includes RhythmVisualizer and EuclideanCircle

4. **`web/src/lib/components/controls/HarmonySection.svelte`**
   - Emotion mode: Valence slider
   - Direct mode: Mode/Strategy/Tension/Valence sliders
   - Includes ChordProgression

5. **`web/src/lib/components/controls/MelodySection.svelte`**
   - Direct mode only: Smoothness/Octave/Voicing sliders

6. **`web/src/lib/components/controls/MixerSection.svelte`**
   - Wraps existing ChannelMixer component
   - Binds to `bridge.setChannelGain/Mute`

#### Phase 3.3: State Management (2 days)

**REWRITE**: `web/src/lib/stores/engine.ts`

```typescript
import { writable, derived } from 'svelte/store';
import type { EngineReport, UnifiedBridge } from '$lib/bridge';

export const bridge = writable<UnifiedBridge | null>(null);
export const latestReport = writable<EngineReport | null>(null);

export const currentChord = derived(latestReport, $report => $report?.current_chord ?? 'I');
export const currentBpm = derived(latestReport, $report => $report?.musical_params.bpm ?? 120);

export function startReportPolling(bridgeInstance: UnifiedBridge) {
    bridge.set(bridgeInstance);

    const poll = () => {
        const reports = bridgeInstance.pollReports();
        if (reports.length > 0) {
            latestReport.set(reports[reports.length - 1]);
        }
        requestAnimationFrame(poll);
    };

    requestAnimationFrame(poll);
}
```

#### Phase 3.4: Integration & Testing (3 days)

1. Wire up all components
2. Test mode switching (emotion ↔ direct)
3. Verify visualizations update correctly
4. Test all parameter controls
5. Verify no UI lag or audio dropouts
6. Cross-browser testing

**Verification**:
- [ ] Web frontend connects to engine
- [ ] All controls send correct commands
- [ ] Visualizations update in real-time
- [ ] RhythmVisualizer shows correct pattern
- [ ] EuclideanCircle matches engine state
- [ ] ChordProgression displays current chord
- [ ] ChannelMixer mute/gain controls work
- [ ] Mode toggle switches between emotion/direct
- [ ] No UI lag or freezing
- [ ] Cross-browser compatibility

### Phase 4: VST Integration (Week 5 - Optional)

**Goal**: Update VST to use new command/report system.

**Tasks**:
1. Update `harmonium_host/src/vst_plugin.rs`: Use new command/report queues
2. Update VST webview: Use UnifiedBridge with IPC backend
3. Test in DAW (Ableton, FL Studio, Reaper)

**Verification**:
- [ ] VST loads in DAW
- [ ] DAW automation works
- [ ] Webview controls match DAW params
- [ ] MIDI output works
- [ ] No IPC errors

## Migration Strategy

**Incremental Approach**:

1. **Week 1**: Add new code alongside old (feature flag `unified-transport`)
2. **Week 2**: CLI as standalone test (proves new architecture)
3. **Week 3-4**: Migrate Web frontend (test thoroughly)
4. **Week 5**: Migrate VST frontend
5. **Week 6**: Cleanup (delete old code, remove feature flags)

**Rollback Plan**:
- Git tags before each phase: `v0.1.0-pre-phase1`, etc.
- Feature flags allow reverting to old architecture if needed

## Critical Files for Implementation

1. **`harmonium_core/src/command.rs`** (NEW) - Core command enum
2. **`harmonium_core/src/controller.rs`** (NEW) - Public controller API with EmotionMapper
3. **`harmonium_host/src/engine.rs`** (MODIFY ~1136 lines) - Replace communication layer
4. **`harmonium_cli/src/main.rs`** (NEW) - CLI REPL for validation
5. **`web/src/lib/bridge/unified-bridge.ts`** (NEW) - Unified bridge for Web + VST

## Success Criteria

**Phase 1**: Core infrastructure compiles, passes rt_check, no audio dropouts
**Phase 2**: CLI can control all 40+ parameters, mode switching works, no crashes
**Phase 3**: Web UI controls work, visualizations update, no lag, cross-browser compatible
**Overall**: Communication is reliable, mode switching doesn't break state, controls always work
