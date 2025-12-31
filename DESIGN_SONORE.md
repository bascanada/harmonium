# Design Sonore: De Chiptune à Synthèse Expressive

## 🎯 Problème Initial

Le patch DSP original était très basique:
```rust
saw() >> lowpass_hz(2000.0, 1.0) >> adsr_live(...)
```

**Résultat**: Son "Atari 2600" - sec, plat, unidimensionnel.

---

## 🔊 Solution Implémentée

### 1. **FM Synthesis (Modulation de Fréquence)**

Architecture inspirée de Yamaha DX7 / Native Instruments FM8:

```rust
// Modulateur: oscillateur sine à fréquence variable (ratio × carrier)
let modulator_freq = carrier_freq * fm_ratio;  // 1.0→5.0
let modulator = modulator_freq >> sine();

// Carrier modulé (enrichissement spectral)
let carrier_freq = base_freq + (modulator * fm_amount * base_freq);
let carrier = carrier_freq >> saw();
```

**Contrôle émotionnel**:
- `TENSION → fm_ratio` (1.0→5.0)
  - Faible tension: ratio ~1.0 (son doux, harmonique)
  - Haute tension: ratio ~5.0 (son métallique, bell-like, inharmonique)
- `TENSION → fm_amount` (0.0→0.8)
  - Profondeur de modulation = complexité spectrale

**Résultat sonore**:
- Tension basse: Son organique, chaud
- Tension haute: Cloches, carillons, timbres métalliques (Gamelan)

---

### 2. **Spatial Effects (Delay)**

Architecture parallèle pour préserver la clarté:

```rust
// Dry/Wet mix: signal direct + écho retardé
filtered >> (pass() & delay(0.3) * 0.3)
```

**Paramètres**:
- Delay time: 300ms (tempo-synced pour future version)
- Wet level: 30% (équilibre clarté/profondeur)

**Effet**:
- Sortir du son "chiptune" sec
- Créer de l'espace et de la profondeur
- Écho musical plutôt que technique

---

### 3. **Enveloppe ADSR Percussive**

```rust
adsr_live(0.01, 0.15, 0.6, 0.3)
```

- **Attack**: 10ms (percussif, articulé)
- **Decay**: 150ms (transition vers sustain)
- **Sustain**: 60% (maintien d'énergie)
- **Release**: 300ms (extinction naturelle)

**Effet**: Notes articulées plutôt que "drones" continus.

---

## 📊 Mapping Émotionnel → Timbre

| Paramètre Émotionnel | Contrôle DSP | Plage | Effet Sonore |
|---------------------|--------------|-------|--------------|
| **TENSION** | FM Ratio | 1.0 → 5.0 | Doux → Métallique |
| **TENSION** | FM Amount | 0.0 → 0.8 | Simple → Complexe |
| **VALENCE** | Reverb Mix | 10% → 50% | Intime → Spacieux |
| **AROUSAL** | Distortion | 0.0 → 0.8 | Clean → Saturé |
| **TENSION** | Cutoff | 500Hz → 4kHz | Sombre → Brillant |
| **TENSION** | Resonance | 1.0 → 5.0 | Doux → Résonant |

---

## 🎼 Exemples Sonores Attendus

### Calme Contemplatif (Low Arousal, Low Tension)
- BPM: ~80
- FM Ratio: ~1.2 (presque harmonique)
- Reverb: 15% (intime)
- **Son**: Pad synthétique doux, organique

### Tension Anxieuse (Medium Arousal, High Tension)
- BPM: ~140
- FM Ratio: ~4.5 (inharmonique)
- Reverb: 40% (espace oppressant)
- **Son**: Cloches dissonantes, métallique

### Joie Exubérante (High Arousal, Low Tension)
- BPM: ~170
- FM Ratio: ~1.5 (harmonique riche)
- Reverb: 50% (ouvert, aéré)
- **Son**: Carillon joyeux, brillant

---

## 🔮 Améliorations Futures

### A. Reverb Algorithmique
Remplacer le delay simple par:
- Multi-tap delay (early reflections)
- Allpass filters (diffusion)
- Feedback matrix (Schroeder reverb)

### B. Contrôle Dynamique du Filtre
FundSP limitation actuelle: `lowpass_hz()` n'accepte pas `var()`.

Solutions:
```rust
// Option 1: Moog filter (accepte contrôle dynamique)
voice >> moog(var(&cutoff), var(&resonance))

// Option 2: Butterworth paramétrable
voice >> butterpass_hz(var(&cutoff))
```

### C. Synthèse Additive
Pour textures évolutives:
```rust
// Stack d'harmoniques pondérées
let fundamental = var(&freq) >> sine();
let harmonic2 = (var(&freq) * 2.0) >> sine() * 0.5;
let harmonic3 = (var(&freq) * 3.0) >> sine() * 0.3;
let harmonic5 = (var(&freq) * 5.0) >> sine() * 0.2;

fundamental + harmonic2 + harmonic3 + harmonic5
```

### D. Modulation Tempo-Synced
Calculer delay time depuis BPM:
```rust
let delay_time = 60.0 / bpm;  // 1 beat
let spatial = filtered >> (pass() & delay(delay_time) * 0.3);
```

---

## 📚 Références

- **FM Synthesis**: Chowning, John. "The Synthesis of Complex Audio Spectra by Means of Frequency Modulation" (1973)
- **Spatial Audio**: Dodge & Jerse, "Computer Music: Synthesis, Composition, and Performance" (1997)
- **Schroeder Reverb**: Schroeder, M.R. "Natural Sounding Artificial Reverberation" (1962)
- **FundSP Documentation**: https://github.com/SamiPerttu/fundsp

---

## 🎵 Avant/Après

### Avant (Chiptune)
```
saw() >> lowpass_hz(2000.0, 1.0)
```
- Son plat, unidimensionnel
- Aucune texture
- Espace sonore inexistant
- Timbre statique

### Après (Synthèse Expressive)
```
FM(carrier, modulator) >> filter >> (dry & delay)
```
- Spectre riche (FM)
- Texture évolutive (modulation)
- Profondeur spatiale (delay)
- Timbre réactif aux émotions
