# Technical Architecture & Threading

Harmonium uses a high-performance, real-time safe architecture designed to run at low latencies while providing a responsive UI.

## Threading Model

The system is split into two primary domains:

### 1. The Audio Thread (High Priority)
- **Responsibility**: Buffer processing, synthesis, and real-time sequencing.
- **Constraints**: 
    - **No Allocations**: No `Vec::new()`, `String`, or `Arc` clones.
    - **No Locking**: No `std::sync::Mutex` (which can block).
    - **No System Calls**: No I/O or sleep operations.
- **Safety**: Verified in debug builds by `RTCheckAllocator`, which panics on any allocation/deallocation in the audio thread.

### 2. The UI/Control Thread (Low Priority)
- **Responsibility**: State management, parameter updates, visualization, and ML model execution.
- **Communication**: Uses lock-free primitives to send and receive data.

## Communication Primitives

### Triple Buffering (`triple_buffer` crate)
Used for **UI -> Audio** parameter updates.
- Allows the UI to write new `EngineParams` at any time without blocking the audio thread.
- The audio thread always reads the "latest" stable version in constant time.

### SPSC Queues (`rtrb` crate)
Used for **Audio -> UI** events.
- **Harmony State**: Pushes the current chord and measure every few ticks for the UI display.
- **Visualization Events**: Pushes `NoteOn`/`NoteOff` events for the piano roll/oscilloscope.

## Audio Rendering Pipeline

1.  **Engine**: Receives `EngineParams`, runs the `HarmonicDriver` and `Sequencer`.
2.  **VoiceManager**: Receives `AudioEvent` triggers.
3.  **Backends**:
    *   `SynthBackend`: Lightweight FundSP + SoundFont (Oxisynth) rendering.
    *   `Odin2Backend`: Heavyweight, high-quality synthesis with emotional morphing.
4.  **Recorder**: (Optional) Wraps the backend to capture output to WAV/MIDI/MusicXML.

## Memory Management
- **Pre-allocation**: The engine pre-allocates buffers (`events_buffer`, `active_lead_notes`) during initialization.
- **Fixed-size Strings**: Uses `ArrayString` for chord names to avoid heap allocations during chord changes.
- **Public Type Aliases**: Complex types like `FontQueue` and `FinishedRecordings` are aliased for readability.
