# Rhythm & Sequencer System

The sequencer engine handles the temporal aspect of Harmonium, generating rhythmic patterns across multiple channels (Bass, Lead, Snare, Hat).

## Rhythm Modes

### 1. Euclidean
- Based on the Bjorklund algorithm.
- Distributes a fixed number of pulses as evenly as possible across a set of steps.
- Ideal for traditional folk and world music rhythms.

### 2. Perfect Balance (XronoMorph Style)
- Uses the "Perfect Balance" theorem where regular polygons are inscribed in a circle representing the time cycle.
- **Density** controls the number of vertices (complexity).
- **Tension** controls the rotation/phase shift (syncopation).
- Guarantees a mathematical "balance" in the rhythmic energy.

### 3. Classic Groove
- Uses heuristic-based patterns derived from real drum kit performances.
- Includes logic for **Ghost Notes**, **Tom Fills**, and **Cymbal Variations**.
- Responds dynamically to **Arousal**:
    - Low Arousal: Uses Rimshots and Pedal Hats.
    - High Arousal: Triggers Crash Cymbals and intense Snare rolls.

## Multi-Sequencer Architecture

Harmonium often runs multiple sequencers in parallel to create polyrhythms:
- **Primary Sequencer**: Typically 16 or 48 steps. Controls the main "feel".
- **Secondary Sequencer**: Can have a different step count (e.g., 12 steps) to create "phasing" effects similar to the works of Steve Reich.

## Step Trigger
Each step in the sequencer returns a `StepTrigger` struct:
- `kick`: Foundation.
- `snare`: Backbeat/Tension.
- `hat`: Fill/High frequency.
- `bass`: Harmonic foundation pulse.
- `lead`: Melodic pulse.
- `velocity`: Global dynamic level (0.0 to 1.0).

## Humanization
The sequencer doesn't just play binary "on/off" triggers. It humanizes the output by:
- **Velocity Scaling**: Adjusting hits based on emotional energy.
- **Fill Zones**: Detecting the end of a measure to trigger more complex patterns.
- **Masking**: Intelligently dropping hi-hat hits when a strong snare or kick occurs to avoid "clutter".
