# Harmonium: a library for reactive procedural music generation based on emotions

Harmonium is a rust library to create procedural music with the help of math based
on emotions.

Design to be used in applications, websites and games.

## Method used

Harmonium is based on multiple layers.

1. The brain        Driver of the emotion (Russel)
2. The skeleton     Geometric rhythms (Euclidean)
3. The body         Adaptive chord progression (Steedman, Neo-Riemannienne)
4. The voice        Organic Melody (Fractal Noise)
5. The lung         DSP

### Brain

Everything starts with the state of the system (the emotion of the audience)

1. Arousal          Affect the speed and distortion
2. Valence          Affect the chords and the spacing
3. Tension          Affect the dissonance, the filtering and the geometry of rhythms
4. Density          Affect the number of notes

### Skeleton

* Bjorklund Algorithm: Distributes notes ("pulses") as evenly as possible within the measure ("steps"). This creates natural "groovy" rhythms (e.g., 3 hits on 8 = Tresillo).

* Polyrhythm (Steve Reich): Two sequencers run in parallel. The first takes 16 steps, the second 12. This creates a phase shift that evolves over time.

* Rotation (Necklace vs Bracelet): Tension changes the starting point of the rhythmic circle. The same distribution of notes sounds very different if shifted (rotation).

### The body

* **Generative Grammar (Steedman)** to create logical and resolving phrases (Jazz/Pop).

* **Neo-Riemannian Theory (PLR)** for geometric and dramatic transformations.

* **Unified Coloration**: All notes pass through the **Lydian Chromatic Concept (George Russell)** to ensure harmonic coherence ("Tonal Gravity") regardless of complexity.

* **Emotional Palettes**: The system selects a chord progression (I-IV-V, i-vii°, etc.) based on the emotional quadrant (e.g., "Sad & Tense" vs "Happy & Calm").

* **Inertia (Hysteresis)**: To prevent the music from changing "style" chaotically, the system waits for a significant emotional change before switching progressions.

### The voice

* Hybrid Generation (Biased Random Walk): Combines the long-term structure of Fractal Pink Noise (1/f) with the local harmonic rules of Markov Chains. The fractal noise acts as a "GPS" guiding the general direction, while Markov chains ensure each step makes musical sense.

* Smoothness Control: The Hurst exponent allows adjusting the melody from erratic (low smoothness) to lyrical and conjunct (high smoothness).

### The lung

Sound is sculpted in real-time via fundsp:

* FM Synthesis: Uses a modulator and a carrier. As Tension rises, the FM ratio increases, creating inharmonic sounds (bell/metallic type).

* Articulation (Anti-Legato): Note duration changes dynamically.
  * Low tension = Long and connected notes (Legato).
  * High tension = Short and percussive notes (Staccato).

* Possibility to manually configure SoundFont sample on each sound channel (Will be reworked with something more procedural in the choice of sample based on the emotion)

### Melodic Driver

It prevents monotony by alternating between **Stability** (functional rules) and **Instability** (mathematical transformations) according to the desired tension curve.

### Lydian Chromatic Concept (The Filter)

Based on the work of George Russell, this module does not view music as Major/Minor, but as a gradient of **Tonal Gravity** (Ingoing vs Outgoing).

* **Low Tension**: Forces notes towards the fundamental Lydian scale (Consonance).
* **High Tension**: Allows notes from Augmented/Diminished Lydian scales (Rich Dissonance).

### Steedman & PLR (The Generators)

* **Steedman:** Uses syntax trees to ensure the music "tells a story" (beginning, middle, end).
* **PLR (Neo-Riemannian):** Uses topology (the *Tonnetz*) to connect chords that are tonally unrelated but geometrically close.

## ML integration

To control the emotions dynamically to help to music evolve the library include ML integration
to run a tensorflow model to transform words into parameters to control the value of our emotions.


## Diagram

```mermaid
  graph TD
    %% --- STYLING ---
    classDef control fill:#f9f,stroke:#333,stroke-width:4px,color:black;
    classDef brain fill:#e1f5fe,stroke:#0277bd,stroke-width:2px,color:black;
    classDef theory fill:#fff9c4,stroke:#fbc02d,stroke-width:2px,color:black;
    classDef perform fill:#e0f2f1,stroke:#00695c,stroke-width:2px,color:black;
    classDef output fill:#212121,stroke:#000,stroke-width:2px,color:white;

    %% --- 1. CONTROL LAYER ---
    subgraph CONTROL ["1. CONTROL (Inputs)"]
        User[("User / AI<br/>(Web Interface)")]:::control
        Param_E[("EmotionState<br/>(Arousal, Valence, Tension)")]:::control
        Param_D[("Density & Rhythm")]:::control
        
        User --> Param_E
        User --> Param_D
    end

    %% --- 2. HARMONIC BRAIN ---
    subgraph DRIVER ["2. HARMONIC DRIVER (The Choice)"]
        Decision{"Tension > 0.6 ?"}:::brain
        
        Param_E --> Decision
        
        %% Strategy A: Low Tension
        Narrative["Narrative Strategy<br/>(Steedman Grammar)<br/>Rules: V -> I"]:::theory
        
        %% Strategy B: High Tension
        Morphing["Morphing Strategy<br/>(Neo-Riemannian PLR)<br/>Geometry: P, L, R"]:::theory
        
        Decision -- No (Stable) --> Narrative
        Decision -- Yes (Unstable) --> Morphing
    end

    %% --- 3. COLORATION ---
    subgraph COLOR ["3. COLORATION (Lydian Chromatic Concept)"]
        LCC_Engine["LCC Context Manager<br/>(Tonal Gravity Calculation)"]:::brain
        LCC_Scale["Lydian Parent Scale<br/>(ex: Lydian Augmented)"]:::theory
        
        Param_E -- "Valence (Mood)" --> LCC_Engine
        Narrative --> LCC_Engine
        Morphing --> LCC_Engine
        
        LCC_Engine -->|"Forces notes<br/>into scale"| LCC_Scale
    end

    %% --- 4. PERFORMANCE & TEXTURE ---
    subgraph VOICER ["4. TEXTURE & RHYTHM (The 'Play')"]
        Sequencer["Sequencer (Clock)"]:::perform
        Euclid["Euclidean Generator<br/>(Rhythmic Mask)"]:::perform
        
        VoiceEngine{"Voicing Engine<br/>(Allocator)"}:::perform
        
        Param_D -- "Density" --> Euclid
        Sequencer --> Euclid
        Euclid -- "Trigger (1/0)" --> VoiceEngine
        LCC_Scale -- "Note Reservoir" --> VoiceEngine
        
        %% Playing Styles
        Style_Block["Style: Block Chords<br/>(Melody Harmonization)"]:::theory
        Style_Shell["Style: Shell Voicing<br/>(Left Hand: 3rd/7th)"]:::theory
        
        VoiceEngine --> Style_Block
        VoiceEngine --> Style_Shell
    end

    %% --- 5. AUDIO OUTPUT ---
    subgraph AUDIO ["5. AUDIO RENDERING"]
        Synth["FM / Wavetable Synthesizer"]:::output
        Midi["MIDI / ABC Output"]:::output
        
        Style_Block --> Synth
        Style_Shell --> Synth
        Style_Block --> Midi
    end
```

## Usefull sources

Here are the books that made this project possible

### Fondation and Geometric Music

* Loy, Gareth (2006). *Musimathics: The Mathematical Foundations of Music*.
* Toussaint, Godfried (2013). *The Geometry of Musical Rhythm*.
* Van Heerden, Derrick Scott (2018). *Music, Geometry and Mathematics*.
* Russell, G. (2001). *The Lydian Chromatic Concept of Tonal Organization*.
* Steedman, M. J. (1984). *A Generative Grammar for Jazz Chord Sequences*.
* Cohn, R. (1998) & Lewin, D. (1987). *Generalized Musical Intervals and Transformations*.

### Algorithmes Rythmiques

* Toussaint, Godfried (2005). The Euclidean Algorithm Generates Traditional Musical Rhythms
* Milne, A. J., Bulger, D., & Herff, S. A. (2017). Exploring the space of perfectly balanced rhythms and scales
* Carey, Norman & Clampitt, David (1989). Aspects of well-formed scales.

### Procedural

* Hiller, Lejaren & Isaacson, Leonard (1959). Experimental Music

## Library used

Here are the library that make this project possible

Music generation:

* rust-music-theory

Sound generation:

* FunDSP
* oxysynth

Sound output:

* cpal (device)
* hound (wav)
* midly (midi)

ML:

* candle
* tokenizers