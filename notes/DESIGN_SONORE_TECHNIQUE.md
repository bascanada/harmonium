# 🎵 Design Sonore Enrichi - Résumé Technique

## Transformation: Chiptune → Synthèse Expressive

### Architecture DSP Avant/Après

#### ❌ AVANT (Chiptune basique)
```
Input (frequency, gate)
    ↓
saw_wave
    ↓
lowpass_filter(2000 Hz, Q=1.0)  [STATIQUE]
    ↓
adsr_envelope
    ↓
Output (mono → stereo split)
```

**Problèmes**:
- Timbre unidimensionnel (sawtooth seule)
- Filtre statique (pas de mouvement)
- Aucun effet spatial (son "sec")
- Enveloppe simple (pas d'articulation)

---

#### ✅ APRÈS (Synthèse expressive)
```
Input (frequency, gate, tension, valence, arousal)
    ↓
┌─────────────────────────────────────┐
│  FM SYNTHESIS MODULE                │
│                                     │
│  Modulator: sine(freq × fm_ratio)  │  ← TENSION contrôle ratio (1.0→5.0)
│             ↓                       │
│  Carrier: saw(freq + mod × amount) │  ← Enrichissement spectral
└─────────────────────────────────────┘
    ↓
ADSR Envelope (percussif)
 • Attack:  10ms  (articulation)
 • Decay:   150ms (transition)
 • Sustain: 60%   (énergie)
 • Release: 300ms (extinction naturelle)
    ↓
Lowpass Filter (2000 Hz) [À améliorer: contrôle dynamique]
    ↓
┌─────────────────────────────────────┐
│  SPATIAL FX (Parallel Architecture) │
│                                     │
│  ┌───────┐    ┌──────────────┐    │
│  │ Dry   │    │ Delay(300ms) │    │  ← Profondeur spatiale
│  │ 70%   │ +  │ × 30%        │    │
│  └───────┘    └──────────────┘    │
└─────────────────────────────────────┘
    ↓
Stereo Split
    ↓
Output (L/R)
```

---

## Mapping Émotionnel → Paramètres DSP

| Émotion | Paramètre DSP | Valeur Min | Valeur Max | Effet Sonore |
|---------|--------------|-----------|-----------|--------------|
| **TENSION** | `fm_ratio` | 1.0 | 5.0 | Harmonique ↔ Inharmonique |
| **TENSION** | `fm_amount` | 0.0 | 0.8 | Simple ↔ Complexe spectral |
| **VALENCE** | `reverb_mix` | 10% | 50% | Intime ↔ Spacieux |
| **AROUSAL** | `bpm` | 70 | 180 | Calme ↔ Énergique |
| **AROUSAL** | `distortion` | 0.0 | 0.8 | Clean ↔ Saturé |
| **TENSION** | `filter_cutoff` | 500Hz | 4kHz | Sombre ↔ Brillant |
| **TENSION** | `filter_resonance` | 1.0 | 5.0 | Doux ↔ Résonant |

---

## Exemples de Presets Émotionnels

### 🌙 Calme Mélancolique
```yaml
arousal:  0.2   # BPM: 92
valence:  -0.6  # Son fermé, intime
density:  0.3   # Rythme clairsemé
tension:  0.15  # Doux, harmonique

→ fm_ratio: 1.15 (presque harmonique)
→ fm_amount: 0.12 (subtil)
→ reverb: 15% (intime)
→ Son: Pad synthétique chaud, légèrement nostalgique
```

### ⚡ Tension Anxieuse
```yaml
arousal:  0.65  # BPM: 141
valence:  -0.4  # Négatif, oppressant
density:  0.7   # Rythme dense
tension:  0.85  # Dissonant, inharmonique

→ fm_ratio: 4.4 (inharmonique)
→ fm_amount: 0.68 (modulation intense)
→ reverb: 40% (espace oppressant)
→ Son: Cloches métalliques dissonantes, textures industrielles
```

### 🎉 Joie Exubérante
```yaml
arousal:  0.9   # BPM: 169
valence:  0.8   # Positif, ouvert
density:  0.8   # Rythme très actif
tension:  0.3   # Relativement consonant

→ fm_ratio: 2.2 (octave + quinte)
→ fm_amount: 0.24 (modéré)
→ reverb: 50% (spacieux, aéré)
→ Son: Carillon brillant, textures cristallines
```

---

## Analyse Spectrale (Prédictions)

### Tension LOW (fm_ratio = 1.0)
```
Amplitude
    │ ██
    │ ██
    │ ██  ▓▓
    │ ██  ▓▓  ░░
    │ ██  ▓▓  ░░  ░
    └─────────────────► Frequency
     f0  2f0 3f0 4f0 5f0

Spectre harmonique simple (proche du naturel)
```

### Tension HIGH (fm_ratio = 5.0)
```
Amplitude
    │ ██
    │ ██ ▓▓     ░░
    │ ██ ▓▓ ░░  ░░  ▓▓
    │ ██ ▓▓ ░░  ░░  ▓▓  ░░
    │ ██ ▓▓ ░░  ░░  ▓▓  ░░  ▓
    └──────────────────────────► Frequency
     f0       5f0        10f0

Spectre inharmonique (bell-like, métallique)
Sidebands à f0±5f0, f0±10f0, etc.
```

---

## Performance Impact

### CPU Usage Estimation
```
Ancien patch (simple):
- 1× Sawtooth oscillator
- 1× Static lowpass filter
- 1× ADSR envelope
≈ 5-10% CPU (1 voice @ 44.1kHz)

Nouveau patch (riche):
- 2× Oscillators (sine + saw)
- 1× FM modulation (multiply + add)
- 1× Static lowpass filter
- 1× ADSR envelope
- 1× Delay line (300ms = 13230 samples buffer)
- 1× Parallel mixer
≈ 15-25% CPU (1 voice @ 44.1kHz)

Ratio: ~2.5× plus coûteux, mais BEAUCOUP plus expressif
```

### Memory Footprint
```
Delay buffer: 300ms × 44100 Hz × 4 bytes = 52.9 KB
Total additional memory: ~60 KB per voice
```

---

## Améliorations Futures Identifiées

### 🔴 Priorité HAUTE
1. **Filtre dynamique** (cutoff/resonance contrôlables en temps réel)
   - Solution: `moog(var(&cutoff), var(&resonance))`
   - Impact: Timbre VRAIMENT réactif aux émotions

2. **Reverb algorithmique** (remplacer delay simple)
   - Schroeder reverb (allpass filters + comb filters)
   - Impact: Espace sonore naturel vs artificiel

### 🟡 Priorité MOYENNE
3. **Delay tempo-synced** (calculé depuis BPM)
   - `delay_time = 60.0 / bpm` (delay = 1 beat)
   - Impact: Cohérence rythmique

4. **Multi-voice synthesis** (polyphonie)
   - 4-6 voices simultanées
   - Impact: Accords, textures plus riches

### 🟢 Priorité BASSE
5. **Synthèse additive** (stack d'harmoniques)
   - Contrôle fin des partiels
   - Impact: Timbres organiques (cordes, voix)

6. **Effets spectraux** (FFT-based)
   - Freeze, spectral delay, morphing
   - Impact: Textures expérimentales

---

## Références Techniques

### FM Synthesis
- **Chowning (1973)**: "The Synthesis of Complex Audio Spectra by Means of Frequency Modulation"
- **Yamaha DX7** (1983): Premier synthé FM grand public
- **Native Instruments FM8**: Référence logicielle moderne

### Spatial Audio
- **Dodge & Jerse**: "Computer Music" (Chapitre 7: Reverberation)
- **Schroeder Reverb** (1962): Algorithme fondamental
- **Dattorro Reverb** (1997): Plate reverb algorithmique de référence

### FundSP
- Documentation: https://github.com/SamiPerttu/fundsp
- Audio Graph DSL: Paradigme fonctionnel pour DSP
- Limitations: Contrôle dynamique limité sur certains nodes

---

## Test Audio Comparatif

```bash
# Écouter l'ancien design (git checkout)
git stash
git checkout HEAD~1
cargo run --release  # ← Son "chiptune"

# Revenir au nouveau design
git stash pop
cargo run --release  # ← Son "expressif"
```

**Critères d'évaluation**:
- ✅ Richesse spectrale (harmoniques vs inharmoniques)
- ✅ Profondeur spatiale (sec vs spatial)
- ✅ Articulation (notes percées vs drones)
- ✅ Réactivité émotionnelle (statique vs dynamique)

---

*Document technique généré le 30 décembre 2025*  
*Harmonium v0.1.0 - BAS Canada*
