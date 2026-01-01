# UI Harmonique: Visualisation en Temps Réel

## 🎯 Fonctionnalités Ajoutées

### 1. État Harmonique Exposé via WASM

#### Backend (Rust)
**Nouvelle structure**: `HarmonyState`
```rust
pub struct HarmonyState {
    pub current_chord_index: usize,  // 0-3 (position dans I-vi-IV-V)
    pub chord_root_offset: i32,      // Demi-tons (0=I, 5=IV, 7=V, 9=vi)
    pub chord_is_minor: bool,        // true si mineur
    pub chord_name: String,          // "I", "vi", "IV", "V"
    pub measure_number: usize,       // Numéro de mesure
    pub cycle_number: usize,         // Numéro de cycle complet
    pub current_step: usize,         // Step 0-15
}
```

**Partage via Arc<Mutex>**:
- Mise à jour dans `engine.rs` à chaque tick
- Exposée via `audio.rs` → `lib.rs`
- Accessible depuis WASM (lecture seule)

---

### 2. Bindings WASM Ajoutés

#### Getters Exposés (TypeScript)
```typescript
interface Handle {
    // Contrôles émotionnels (existants)
    set_arousal(value: number): void;
    set_valence(value: number): void;
    set_density(value: number): void;
    set_tension(value: number): void;
    
    // NOUVEAUX: État harmonique
    get_current_chord_name(): string;      // "I", "vi", "IV", "V"
    get_current_chord_index(): number;     // 0-3
    is_current_chord_minor(): boolean;     // true/false
    get_current_measure(): number;         // 1, 2, 3...
    get_current_cycle(): number;           // 1, 2, 3...
    get_current_step(): number;            // 0-15
}
```

---

### 3. Composant UI Svelte

#### Panneau "Harmonic Progression"

**Affichage en temps réel**:
- 🎼 **Accord courant**: Taille XL avec couleur (Majeur=Jaune, Mineur=Bleu)
- 📊 **Mesure/Cycle**: Compteurs temps réel
- 🔄 **Progression visuelle**: 4 cercles (I-vi-IV-V) avec highlight actif
- 📈 **Barre de progression**: Steps 0-15 en temps réel

#### Polling Mechanism
```typescript
// 30 FPS (33ms) pour fluidité
setInterval(() => {
    currentChord = handle.get_current_chord_name();
    currentMeasure = handle.get_current_measure();
    currentCycle = handle.get_current_cycle();
    currentStep = handle.get_current_step();
    isMinorChord = handle.is_current_chord_minor();
}, 33);
```

---

## 🎨 Design Visuel

### Palette de Couleurs

| Élément | Couleur | Signification |
|---------|---------|---------------|
| Accord Majeur | Jaune (`text-yellow-400`) | Lumineux, positif |
| Accord Mineur | Bleu (`text-blue-400`) | Mélancolique |
| Accord Actif | Purple (`bg-purple-600`) | Highlight animation |
| Mesure | Vert (`text-green-400`) | Tempo/Timing |
| Cycle | Purple (`text-purple-400`) | Structure globale |

### Animation
- **Scale transition**: 110% sur accord actif
- **Shadow glow**: Purple avec blur
- **Barre progression**: Gradient purple→pink
- **Transition**: 300ms smooth

---

## 📐 Architecture des Données

### Flow de Données

```
┌─────────────────────────────────────────────────────┐
│  RUST ENGINE (engine.rs)                           │
│                                                     │
│  process() {                                        │
│    - Tick séquenceurs                              │
│    - Détection nouvelle mesure                     │
│    - Changement d'accord si nécessaire             │
│    - Mise à jour harmony_state (Arc<Mutex>)        │
│  }                                                  │
└─────────────────────────────────────────────────────┘
                     ↓
┌─────────────────────────────────────────────────────┐
│  WASM BINDINGS (lib.rs)                            │
│                                                     │
│  Handle {                                           │
│    harmony_state: Arc<Mutex<HarmonyState>>         │
│                                                     │
│    get_current_chord_name() → harmony_state.lock() │
│  }                                                  │
└─────────────────────────────────────────────────────┘
                     ↓
┌─────────────────────────────────────────────────────┐
│  SVELTE UI (+page.svelte)                          │
│                                                     │
│  setInterval(() => {                                │
│    currentChord = handle.get_current_chord_name(); │
│    // Déclenche réactivité Svelte ($:)             │
│  }, 33ms)                                           │
└─────────────────────────────────────────────────────┘
                     ↓
┌─────────────────────────────────────────────────────┐
│  DOM (Browser)                                      │
│                                                     │
│  <div class="text-5xl {isMinor ? 'blue' : 'yellow'}">│
│    {currentChord}                                   │
│  </div>                                             │
└─────────────────────────────────────────────────────┘
```

---

## 🔄 Mises à Jour en Temps Réel

### Fréquence de Polling

| Élément | Fréquence | Raison |
|---------|-----------|--------|
| **Accord** | 30 FPS | Changement tous les 2-4 mesures (lent) |
| **Mesure** | 30 FPS | Changement ~2-4 fois par seconde (moyen) |
| **Step** | 30 FPS | Changement ~10 fois par seconde (rapide) |

**Optimisation**: Un seul intervalle pour tout (évite overhead)

### Performance
- **CPU UI**: < 1% (lecture simple de mutex)
- **CPU Engine**: +0% (mise à jour déjà nécessaire)
- **Latence**: ~33ms (imperceptible à l'oreille)

---

## 📱 Responsive Design

### Breakpoints

```css
/* Mobile: Stack vertical */
@media (max-width: 640px) {
    .grid-cols-2 → .grid-cols-1
}

/* Tablet/Desktop: Grid horizontal */
@media (min-width: 641px) {
    .grid-cols-2 (maintenu)
}
```

### Tailles Adaptatives
- **Accord**: `text-5xl` (mobile) → `text-6xl` (desktop)
- **Cercles**: `w-12 h-12` (mobile) → `w-16 h-16` (desktop)

---

## 🎯 User Experience

### Feedback Visuel

1. **Changement d'accord**: 
   - Animation scale (110%)
   - Glow effect (shadow)
   - Transition smooth (300ms)

2. **Progression dans mesure**:
   - Barre de progression fluide
   - Gradient animé
   - Compteur step/16

3. **Cycle complet**:
   - Compteur incrémental
   - Reset visuel sur cycle

### États Visuels

| État | Visuel |
|------|--------|
| **Actif** | Scale 110%, purple glow, white text |
| **Inactif** | Scale 100%, neutral gray |
| **Majeur** | Yellow text |
| **Mineur** | Blue text |

---

## 🧪 Test Manuel

### Checklist UI

```bash
# 1. Lancer le serveur
./dev_server.sh

# 2. Ouvrir http://localhost:5173

# 3. Cliquer "Start Music"

# 4. Vérifier affichage initial:
   ✓ Accord: "I" (jaune, actif)
   ✓ Mesure: 1
   ✓ Cycle: 1
   ✓ Step: 0-15 (animation)

# 5. Attendre ~10 secondes (changement accord)
   ✓ Accord passe à "vi" (bleu, actif)
   ✓ Animation scale + glow
   ✓ Mesure incrémente

# 6. Observer cycle complet (I→vi→IV→V→I)
   ✓ 4 changements d'accords
   ✓ Retour à "I"
   ✓ Cycle incrémente

# 7. Tester sliders:
   ✓ Valence > 0.5: Changements rapides (2 mesures)
   ✓ Valence < 0.5: Changements lents (4 mesures)
```

---

## 📚 Documentation Technique

### Fichiers Modifiés

| Fichier | Changements |
|---------|-------------|
| `src/engine.rs` | + `HarmonyState` struct, mise à jour dans process() |
| `src/audio.rs` | Retour `Arc<Mutex<HarmonyState>>` |
| `src/lib.rs` | + 6 getters harmony, Handle avec harmony_state |
| `src/main.rs` | Destructure tuple avec harmony_state |
| `web/src/routes/+page.svelte` | + Composant progression, polling |

### Lignes de Code

- **Rust**: ~150 lignes ajoutées
- **Svelte**: ~80 lignes ajoutées
- **Total**: ~230 lignes

---

## 🔮 Extensions Futures

### 1. Visualisation Avancée
```svelte
<!-- Affichage des notes de l'accord -->
<div>Notes: {currentChordNotes.join(', ')}</div>

<!-- Cercle de quintes interactif -->
<svg viewBox="0 0 200 200">
  <circle cx="100" cy="100" r="80" fill="none" stroke="white" />
  <!-- Points pour chaque tonalité -->
</svg>
```

### 2. Historique de Progression
```typescript
let chordHistory: string[] = [];

// Capturer les changements
$: if (currentChord) {
  chordHistory.push(currentChord);
  if (chordHistory.length > 20) chordHistory.shift();
}
```

### 3. Prédiction Visuelle
```svelte
<!-- Afficher le prochain accord -->
<div class="text-neutral-500">
  Next: {nextChord} in {measuresUntilChange} measures
</div>
```

### 4. MIDI Export
```typescript
// Enregistrer la séquence harmonique
function exportToMIDI() {
  const midi = new MIDIFile();
  chordHistory.forEach((chord, i) => {
    midi.addChord(chord, i * beatsPerChord);
  });
  midi.download();
}
```

---

## ✅ Résumé

### Ce qui fonctionne maintenant:
- ✅ État harmonique exposé via WASM
- ✅ Polling 30 FPS pour fluidité
- ✅ Affichage temps réel de l'accord courant
- ✅ Visualisation progression I-vi-IV-V
- ✅ Barre de progression step/mesure
- ✅ Animations smooth et glow effects
- ✅ Couleurs selon type (Majeur/Mineur)
- ✅ Compteurs mesure/cycle

### Bénéfices utilisateur:
- 🎯 **Compréhension**: Voir où on en est dans la progression
- 🎨 **Engagement**: Visualisation rend l'expérience plus immersive
- 🎓 **Éducatif**: Apprendre la structure I-vi-IV-V
- 🎵 **Prédictif**: Anticiper les changements harmoniques

---

*Documentation UI - Harmonium v0.2.0*  
*30 décembre 2025*
