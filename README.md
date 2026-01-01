# Harmonium: a library for procedural music generation

Harmonium is a rust library to create procedural music with the help of math based
on emotions.

Design to be used in applications, websites and games.

## Library used

* FunDSP
* rust-music-theory
* cpal

## Method used

Harmonium is based on multiple layers.

1. The brain        Driver of the emotion (Russel)
2. The skeleton     Geometric rhythms (Euclidean)
3. The body         Adaptive chord progression (Jean Guy)
4. The voice        Organic Melody (Fractal Noise)
5. The lung         DSP

### Brain

Everything starts with the state of the system (the emotion of the audience)

1. Arousal          Affect the speed and distortion
2. Valence          Affect the chords and the spacing
3. Tension          Affect the dissonance, the filtering and the geometry of rhythms
4. Density          Affect the number of notes

### Skeleton

    Bjorklund Algorithm: Distributes notes ("pulses") as evenly as possible within the measure ("steps"). This creates natural "groovy" rhythms (e.g., 3 hits on 8 = Tresillo).

    Polyrhythm (Steve Reich): Two sequencers run in parallel. The first takes 16 steps, the second 12. This creates a phase shift that evolves over time.

    Rotation (Necklace vs Bracelet): Tension changes the starting point of the rhythmic circle. The same distribution of notes sounds very different if shifted (rotation).

### The body

    Emotional Palettes: The system selects a chord progression (I-IV-V, i-vii°, etc.) based on the emotional quadrant (e.g., "Sad & Tense" vs "Happy & Calm").

    Inertia (Hysteresis): To prevent the music from changing "style" chaotically, the system waits for a significant emotional change before switching progressions.

    Local Context: At any moment, the system knows the current chord (e.g., C Major) and its constituent notes (C, E, G).

### The voice

    Hybrid Generation (Biased Random Walk): Combines the long-term structure of Fractal Pink Noise (1/f) with the local harmonic rules of Markov Chains. The fractal noise acts as a "GPS" guiding the general direction, while Markov chains ensure each step makes musical sense.

    Smoothness Control: The Hurst exponent allows adjusting the melody from erratic (low smoothness) to lyrical and conjunct (high smoothness).

### The lung

Sound is sculpted in real-time via fundsp:

    FM Synthesis: Uses a modulator and a carrier. As Tension rises, the FM ratio increases, creating inharmonic sounds (bell/metallic type).

    Articulation (Anti-Legato): Note duration changes dynamically.

        Low tension = Long and connected notes (Legato).

        High tension = Short and percussive notes (Staccato).

## Usefull sources
