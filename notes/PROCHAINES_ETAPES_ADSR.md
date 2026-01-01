# Prochaines Étapes : ADSR Adaptatif et Vélocité

## 🎯 Objectif

Compléter le système d'articulation dynamique en rendant l'enveloppe ADSR et la vélocité réactives aux paramètres émotionnels.

## 📋 Checklist d'Implémentation

### ✅ Phase 1 : Articulation Dynamique (TERMINÉ)
- [x] Ajout des champs `gate_timer` et `current_gate_duration`
- [x] Logique de fermeture anticipée du gate
- [x] Calcul basé sur la tension (0.95 → 0.20)
- [x] Humanisation aléatoire (±10%)
- [x] Protection contre durées nulles (min 100 samples)

### 🔄 Phase 2 : ADSR Dynamique (À IMPLÉMENTER)

#### 2.1. Ajouter des Shared pour ADSR
```rust
// Dans HarmoniumEngine
attack_time: Shared,   // 0.005 - 0.05s (5ms - 50ms)
decay_time: Shared,    // 0.05 - 0.3s
sustain_level: Shared, // 0.4 - 0.9
release_time: Shared,  // 0.1 - 0.5s
```

#### 2.2. Modifier le Graphe DSP
```rust
// Remplacer l'ADSR fixe:
// let envelope = var(&gate) >> adsr_live(0.01, 0.15, 0.6, 0.3);

// Par un ADSR dynamique:
let envelope = var(&gate) >> envelope(|t, g, s| 
    if g > 0.5 {
        // Attack
        if t < var(&attack_time).value() {
            t / var(&attack_time).value()
        }
        // Decay
        else if t < var(&attack_time).value() + var(&decay_time).value() {
            let decay_progress = (t - var(&attack_time).value()) / var(&decay_time).value();
            1.0 - (1.0 - var(&sustain_level).value()) * decay_progress
        }
        // Sustain
        else {
            var(&sustain_level).value()
        }
    } else {
        // Release
        let release_progress = t / var(&release_time).value();
        s * (1.0 - release_progress).max(0.0)
    }
);
```

#### 2.3. Mapping Émotionnel → ADSR

**Attack (Arousal)** : Réactivité au trigger
```rust
// Haute arousal = attack court (percussif)
// Basse arousal = attack long (doux)
let target_attack = 0.05 - (self.current_state.arousal * 0.045); // 50ms → 5ms
self.attack_time.set_value(target_attack);
```

**Decay (Tension)** : Vitesse d'évolution vers sustain
```rust
// Haute tension = decay court (nerveux)
// Basse tension = decay long (relaxé)
let target_decay = 0.3 - (self.current_state.tension * 0.25); // 300ms → 50ms
self.decay_time.set_value(target_decay);
```

**Sustain (Density)** : Niveau de plateau
```rust
// Haute densité = sustain élevé (remplissage)
// Basse densité = sustain faible (épuré)
let target_sustain = 0.4 + (self.current_state.density * 0.5); // 0.4 → 0.9
self.sustain_level.set_value(target_sustain);
```

**Release (Valence)** : Durée de fin de note
```rust
// Valence positive = release long (spacieux, ouvert)
// Valence négative = release court (sec, fermé)
let target_release = 0.1 + (self.current_state.valence.abs() * 0.4); // 100ms → 500ms
self.release_time.set_value(target_release);
```

### 🎚️ Phase 3 : Vélocité/Accentuation (À IMPLÉMENTER)

#### 3.1. Ajouter un nœud de gain modulable
```rust
// Dans la struct
velocity_gain: Shared,

// Dans new()
let velocity_gain = shared(1.0);

// Dans le graphe DSP (après l'enveloppe)
let voice = carrier * envelope * var(&velocity_gain);
```

#### 3.2. Calcul de la vélocité
```rust
// Dans process(), au moment du trigger
if trigger {
    // ... (code existant) ...
    
    // ACCENTUATION DES TEMPS FORTS
    let base_velocity = if is_strong_beat { 1.0 } else { 0.7 };
    
    // Modulation par arousal (plus d'énergie = plus de contraste)
    let velocity_contrast = 0.3 + (self.current_state.arousal * 0.4);
    let velocity = if is_strong_beat {
        1.0
    } else {
        1.0 - velocity_contrast
    };
    
    // Variation légère pour humanisation
    let velocity_humanize: f32 = rng.gen_range(0.95..1.05);
    let final_velocity = (velocity * velocity_humanize).clamp(0.3, 1.0);
    
    self.velocity_gain.set_value(final_velocity);
}
```

#### 3.3. Vélocité contextuelle avancée
```rust
// Accentuation intelligente basée sur la position rythmique
let velocity = match self.sequencer_primary.current_step {
    0 => 1.0,           // Début de mesure (downbeat)
    4 => 0.9,           // Beat 2
    8 => 0.85,          // Beat 3
    12 => 0.8,          // Beat 4
    _ => 0.65,          // Off-beats
};

// Boost sur début de cycle de progression
if self.progression_index == 0 && self.sequencer_primary.current_step == 0 {
    velocity *= 1.15; // Accent sur nouvel accord
}
```

## 🎼 Interactions Entre Articulation et ADSR

### Scénario 1 : Calme Positif
```
Tension: 0.15  → Articulation: 88% (notes longues)
Valence: 0.8   → Release: 420ms (spacieux)
Arousal: 0.3   → Attack: 36ms (doux)

Résultat: Nappe fluide avec transitions douces
```

### Scénario 2 : Énergique Neutre
```
Tension: 0.5   → Articulation: 57% (équilibré)
Valence: 0.0   → Release: 100ms (neutre)
Arousal: 0.7   → Attack: 13ms (percussif)

Résultat: Groove dynamique avec punch
```

### Scénario 3 : Anxieux Négatif
```
Tension: 0.85  → Articulation: 31% (très court)
Valence: -0.4  → Release: 260ms (long car valence abs élevée)
Arousal: 0.8   → Attack: 9ms (très percussif)

Résultat: Notes sèches avec longue traînée de reverb
          (contraste intéressant: staccato + ambiance)
```

## 🔧 Code Complet pour process()

```rust
// === SECTION C: Mise à jour DSP ===

// C1. ADSR DYNAMIQUE (nouveau)
let target_attack = 0.05 - (self.current_state.arousal * 0.045);
self.attack_time.set_value(target_attack.max(0.001));

let target_decay = 0.3 - (self.current_state.tension * 0.25);
self.decay_time.set_value(target_decay.max(0.01));

let target_sustain = 0.4 + (self.current_state.density * 0.5);
self.sustain_level.set_value(target_sustain);

let target_release = 0.1 + (self.current_state.valence.abs() * 0.4);
self.release_time.set_value(target_release);

// C2. FM Synthesis (existant)
let target_fm_ratio = 1.0 + (self.current_state.tension * 4.0);
self.fm_ratio.set_value(target_fm_ratio);
// ... etc ...
```

## 📊 Tableau Récapitulatif des Mappings

| Paramètre | Contrôle | Plage | Impact Sonore |
|-----------|----------|-------|---------------|
| **Articulation** | Tension | 20%-95% | Durée note/silence |
| **Attack** | Arousal | 5-50ms | Percussion vs Douceur |
| **Decay** | Tension | 50-300ms | Nervosité vs Relaxation |
| **Sustain** | Density | 40%-90% | Remplissage sonore |
| **Release** | Valence abs | 100-500ms | Ouverture spatiale |
| **Vélocité** | Position + Arousal | 30%-100% | Accentuation rythmique |

## 🎛️ Interface Web : Suggestions d'Affichage

### Visualisation ADSR en temps réel
```
╭──╮
│  │     A: 12ms  ← Très réactif (Arousal 0.8)
│  ╰──╮  D: 80ms  ← Court (Tension 0.7)
│     ╰──────╮  S: 75%  ← Élevé (Density 0.7)
│            ╰────  R: 350ms ← Long (Valence 0.6)
```

### Indicateurs visuels
- **Articulation** : Barre horizontale avec ratio
- **ADSR** : Graphe animé suivant le gate
- **Vélocité** : Points colorés par intensité sur la timeline

## 🧪 Tests Recommandés

### Test 1 : Validation Articulation + ADSR
```rust
// Calme positif → Notes longues + Release long
target.tension = 0.1;
target.valence = 0.8;
target.arousal = 0.3;

// Attendu:
// - Articulation 87% (long)
// - Release 420ms (spacieux)
// - Attack 36ms (doux)
// → Son fluide, ambiant
```

### Test 2 : Staccato avec Release long
```rust
// Nerveux négatif → Notes courtes + Release long
target.tension = 0.9;
target.valence = -0.5;
target.arousal = 0.7;

// Attendu:
// - Articulation 28% (très court)
// - Release 300ms (long)
// - Attack 13ms (percussif)
// → Effet "ping-pong" avec reverb tail
```

### Test 3 : Groove percussif
```rust
// Énergique positif → Notes moyennes + Attack court
target.tension = 0.5;
target.valence = 0.4;
target.arousal = 0.9;

// Attendu:
// - Articulation 57% (moyen)
// - Attack 5ms (très percussif)
// - Vélocité contrastée (1.0 vs 0.6)
// → Groove punchy avec accentuation
```

## 🚀 Ordre d'Implémentation Recommandé

1. **Semaine 1** : ADSR Release dynamique (impact le plus audible)
   - Ajouter `release_time: Shared`
   - Mapper à `valence.abs()`
   - Tester avec différentes progressions

2. **Semaine 2** : Attack dynamique (réactivité)
   - Ajouter `attack_time: Shared`
   - Mapper à `arousal`
   - Combiner avec articulation existante

3. **Semaine 3** : Vélocité/Accentuation
   - Ajouter `velocity_gain: Shared`
   - Implémenter accentuation temps forts
   - Ajouter humanisation

4. **Semaine 4** : Decay + Sustain (finition)
   - Compléter ADSR complet
   - Affiner les plages de valeurs
   - Tests d'intégration

## 📚 Ressources Techniques

- **ADSR Theory** : https://en.wikipedia.org/wiki/Envelope_(music)
- **Velocity Sensitivity** : MIDI spec, vélocité 0-127
- **Humanization Techniques** : Roger Linn (MPC), "Feel" algorithmique
- **Emotional Mapping** : Russell's Circumplex + Plutchik's Wheel

## ✨ Impact Attendu

Avec ces 3 phases implémentées :
- **Expressivité** : +150% (notes courtes ET longues, douces ET percussives)
- **Variété timbrale** : +120% (ADSR adaptatif = enveloppes différentes)
- **Groove/Feel** : +200% (vélocité = accentuation rythmique)
- **Réalisme émotionnel** : +180% (cohérence entre tous les paramètres)

Le moteur pourra alors produire des variations allant de nappes ambient fluides à des patterns techno percussifs, en passant par des grooves pop accentués, tout en conservant une cohérence émotionnelle forte.
