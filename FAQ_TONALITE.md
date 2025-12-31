# FAQ: Tonalité vs Progression Harmonique

## ❓ Question Fréquente

> "Pourquoi `F PentatonicMinor | Pulses: 4/16` ne change jamais en haut?"

---

## 📚 Explication Rapide

### 2 Niveaux d'Harmonie

Harmonium utilise une architecture à **2 niveaux**:

```
┌─────────────────────────────────────────┐
│  NIVEAU 1: Tonalité Globale (Global)   │
│  F PentatonicMinor (ne change jamais)  │  ← Ce que vous voyez en haut
│                                         │
│  C'est la "MAISON"                      │
└─────────────────────────────────────────┘
                  ↓
┌─────────────────────────────────────────┐
│  NIVEAU 2: Progression Locale (Local)  │
│  I → vi → IV → V (change)               │  ← Ce que vous voyez dans le panneau
│                                         │
│  Ce sont les "PIÈCES" de la maison      │
└─────────────────────────────────────────┘
```

---

## 🎼 Analogie Musicale

### C'est comme une chanson pop:

**Exemple**: "Someone Like You" d'Adele
- **Tonalité globale**: La Majeur (ne change jamais pendant la chanson)
- **Progression locale**: I → V → vi → IV (répète en boucle)

**Dans Harmonium**:
- **Tonalité globale**: `F PentatonicMinor` (la fondation)
- **Progression locale**: `I → vi → IV → V` (les accords qui bougent)

---

## 🔍 Détail Technique

### Tonalité Globale (Global Key)

```rust
// Défini au démarrage de la session
let random_key = PitchSymbol::F;
let random_scale = ScaleType::PentatonicMinor;

// NE CHANGE PAS pendant toute la session
```

**Rôle**: Définit la "palette de notes" disponibles
- F PentatonicMinor = F, Ab, Bb, C, Eb (5 notes)

### Progression Locale (Local Harmony)

```rust
// Change toutes les 2-4 mesures
const CHORD_PROGRESSION: [(i32, bool); 4] = [
    (0, false),  // I   (F Maj)
    (9, true),   // vi  (D Min)
    (5, false),  // IV  (Bb Maj)
    (7, false),  // V   (C Maj)
];
```

**Rôle**: Définit quel accord est "actif" à chaque moment
- Change automatiquement selon Valence

---

## 🎯 Pourquoi 2 Niveaux?

### Avantages Musicaux

1. **Cohérence**: Tout reste dans la même tonalité = son unifié
2. **Variété**: Les accords changent = pas monotone
3. **Structure**: Progression I-vi-IV-V = sensation de "chanson" vs "drone"

### Exemple Visuel

```
Tonalité: F PentatonicMinor (constant)
    │
    ├─ Mesure 1-2:  Accord I   (F Maj)   ← Notes: F, A, C
    ├─ Mesure 3-4:  Accord vi  (D Min)   ← Notes: D, F, A
    ├─ Mesure 5-6:  Accord IV  (Bb Maj)  ← Notes: Bb, D, F
    └─ Mesure 7-8:  Accord V   (C Maj)   ← Notes: C, E, G
                        ↓
              Retour à I (cycle)
```

**Toutes ces notes** proviennent de F PentatonicMinor!

---

## 🔄 Modulation (Futur)

### Ce qui pourrait changer à l'avenir:

**Modulation** = Changer de tonalité globale pendant la session

Exemple:
```
00:00 - 01:00  F PentatonicMinor  (Progression: I-vi-IV-V)
01:00 - 02:00  G PentatonicMinor  (Progression: I-vi-IV-V)
02:00 - 03:00  A PentatonicMinor  (Progression: I-vi-IV-V)
```

**Contrôle potentiel**: Haute Arousal + Haute Tension = déclenche modulation

### Code pour implémenter (futur):

```rust
// Dans engine.rs
if self.cycle_counter % 4 == 0 && self.current_state.tension > 0.8 {
    // Moduler +2 demi-tons (ex: F → G)
    let new_root = (self.global_key_root + 2) % 12;
    self.harmony.modulate(new_root);
    
    log::info("🎵 MODULATION: F → G");
}
```

---

## 📊 Affichage UI Actuel

### Ce que vous voyez maintenant:

```
┌─────────────────────────────────────┐
│  🎹 Global Key:                     │
│  F PentatonicMinor | Pulses: 4/16  │  ← CONSTANT (normal!)
│  (The "home" tonality)              │
└─────────────────────────────────────┘

┌─────────────────────────────────────┐
│  🎼 Harmonic Progression            │
│  Local chord changes                │
│                                     │
│  Current Chord: vi (Minor)          │  ← CHANGE (toutes les 2-4 mesures)
│  I → vi → IV → V                    │
└─────────────────────────────────────┘
```

---

## ✅ Résumé

### Question: Pourquoi ça ne change pas?
**Réponse**: C'est **normal** et **voulu**!

- **Tonalité globale** (en haut) = CONSTANT
- **Progression locale** (panneau) = VARIABLE

### Analogie Finale

```
Tonalité globale  =  Langue parlée (Français)
Progression locale =  Phrases (sujet-verbe-complément)

Vous ne changez pas de langue pendant une conversation,
mais vous changez de phrases!
```

---

## 🎓 Pour Aller Plus Loin

### Concepts Musicaux

1. **Tonalité** (Key): La "famille de notes"
2. **Mode** (Scale): Comment organiser ces notes (Pentatonique, Diatonique, etc.)
3. **Progression** (Chord Changes): Quels accords jouer dans quel ordre
4. **Modulation** (Key Change): Changer de tonalité (rare, dramatique)

### Références

- **I-vi-IV-V**: "4 Chords Song" (Axis of Awesome)
- **Pentatonique**: Gamme de 5 notes (blues, rock, pop)
- **Mineur**: Mode avec tierce mineure (mélancolique)

---

*FAQ - Harmonium v0.2.0*  
*30 décembre 2025*
