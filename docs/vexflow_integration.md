# VexFlow Integration Guide

This guide explains how `harmonium_core` integrates with VexFlow for real-time score visualization.

## Architecture

```text
  +------------------+          +-------------------+          +-------------------+
  |                  |  Audio   |                   |  Events  |                   |
  |  MusicKernel     |  Events  |  ScoreBuffer      |  (JSON)  |  Frontend (Vex)   |
  |  (Rust)          +--------->+  (Rust)           +--------->+  (TypeScript)     |
  |                  |          |                   |          |                   |
  +--------+---------+          +---------+---------+          +---------+---------+
           |                             |                             |
           | Shared Note IDs             | Shared Note IDs             | Shared Note IDs
           v                             v                             v
  +--------+---------+          +---------+---------+          +---------+---------+
  |                  |          |                   |          |                   |
  |  Audio Engine    |          |  HarmoniumScore   |          |  VexFlow Renderer |
  |  (C++)           |          |  (Rust)           |          |  (JS/Canvas)      |
  |                  |          |                   |          |                   |
  +------------------+          +-------------------+          +-------------------+
```

## Data Format

The `HarmoniumScore` format is the bridge between Rust and the TypeScript frontend. It contains all the information needed for notation:

-   **Tempo & Time Signature**: Global metadata.
-   **Parts**: Each part corresponds to a VexFlow Stave (or multiple staves for piano).
-   **Measures**: Each measure contains notes and chord symbols.
-   **ScoreNoteEvents**: Individual musical events (notes, rests, chords, drums).

### Format Specification

-   **Beat Positions**: 1-indexed. Beat `1.0` is the start of the measure.
-   **Pitch Strings**: Pitches use a VexFlow-compatible format: `[note][alter]/[octave]` (e.g., `c#/4`, `bb/3`, `f/5`).
-   **Duration Strings**: Durations follow VexFlow's naming convention:
    -   `w`: Whole
    -   `h`: Half
    -   `q`: Quarter
    -   `8`: Eighth
    -   `16`: Sixteenth
    -   `32`: Thirty-second
    -   Append `d` for dotted notes (e.g., `qd`, `hd`).

## Synchronization & Highlighting

Playback highlighting is achieved via shared note IDs:

1.  The `MusicKernel` generates a unique `u64` ID for each `NoteOn` event using `next_note_id()`.
2.  This ID is added to both the `AudioEvent` (for playback) and the `ScoreNoteEvent` (for visualization).
3.  The frontend receives the IDs in the `HarmoniumScore` JSON.
4.  As the `AudioEngine` plays notes, it sends back the current active note IDs.
5.  The frontend uses these IDs to identify and highlight the corresponding VexFlow notes.

## Example Conversion (TypeScript)

To convert a `Pitch` object to a VexFlow key:

```typescript
function pitchToVexFlow(pitch: any): string {
    const step = pitch.step.toLowerCase();
    const alter = pitch.alter === 1 ? '#' : pitch.alter === -1 ? 'b' : '';
    return `${step}${alter}/${pitch.octave}`;
}
```

## Troubleshooting

-   **Notes not appearing**: Verify the `Part` ID matches the frontend configuration.
-   **Invalid positions**: Check `beat` positions; they must be within the measure's time signature.
-   **Highlighting mismatch**: Ensure `next_note_id()` is called only once per actual note event and shared correctly between audio and score.
