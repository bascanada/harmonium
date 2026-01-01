## 🎼 Architecture Harmonique: 2 Niveaux Expliqués

```
┌────────────────────────────────────────────────────────────────┐
│                    SESSION HARMONIUM                           │
│                                                                │
│  🏠 TONALITÉ GLOBALE (La Maison)                              │
│     F PentatonicMinor                                         │
│     ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━  │
│     Notes disponibles: F, Ab, Bb, C, Eb                       │
│     ⚠️  NE CHANGE JAMAIS pendant la session                   │
│                                                                │
│     Affiché en haut: "F PentatonicMinor | Pulses: 4/16"       │
│                                                                │
└────────────────────────────────────────────────────────────────┘
                           │
                           │ Utilise ces notes pour construire
                           │ les accords de la progression
                           ↓
┌────────────────────────────────────────────────────────────────┐
│  🎵 PROGRESSION LOCALE (Les Pièces de la Maison)              │
│                                                                │
│  Mesure 1-2:  🟡 I   (F Majeur)    ← Notes: F, A, C          │
│               Fonction: Tonique (Repos, Stabilité)            │
│                                                                │
│  Mesure 3-4:  🔵 vi  (D Mineur)    ← Notes: D, F, A          │
│               Fonction: Relative (Mélancolie, Couleur)        │
│                                                                │
│  Mesure 5-6:  🟡 IV  (Bb Majeur)   ← Notes: Bb, D, F         │
│               Fonction: Sous-dominante (Préparation)          │
│                                                                │
│  Mesure 7-8:  🟡 V   (C Majeur)    ← Notes: C, E, G          │
│               Fonction: Dominante (Tension → Résolution)      │
│                                                                │
│  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━  │
│  ✅ CHANGE automatiquement toutes les 2-4 mesures            │
│     (Contrôlé par Valence: >0.5 = rapide, <0.5 = lent)       │
│                                                                │
│  Affiché dans le panneau: "Current Chord: vi (Minor)"         │
│                                                                │
└────────────────────────────────────────────────────────────────┘
```

---

## 🎯 Ce qui se passe en temps réel:

```
Timeline (BPM = 120):

Seconde 0    ┌──────────────────┐
             │  Tonalité: F Pm  │  ← Reste constant
             │  Accord:   I     │  ← Change ici
             └──────────────────┘

Seconde 8    ┌──────────────────┐
             │  Tonalité: F Pm  │  ← Toujours constant
             │  Accord:   vi    │  ← Change ici (nouveau!)
             └──────────────────┘

Seconde 16   ┌──────────────────┐
             │  Tonalité: F Pm  │  ← Toujours constant
             │  Accord:   IV    │  ← Change encore
             └──────────────────┘

Seconde 24   ┌──────────────────┐
             │  Tonalité: F Pm  │  ← Toujours constant
             │  Accord:   V     │  ← Change encore
             └──────────────────┘

Seconde 32   ┌──────────────────┐
             │  Tonalité: F Pm  │  ← Toujours constant
             │  Accord:   I     │  ← RETOUR (cycle complet!)
             └──────────────────┘
```

---

## 🎨 Visualisation du Son

### Tonalité Globale = La Palette de Couleurs

```
F PentatonicMinor = 🎨 { 🔴 Rouge, 🟠 Orange, 🟡 Jaune, 🟢 Vert, 🔵 Bleu }
                          F      Ab      Bb      C      Eb
```

**Toute la session** utilise seulement ces 5 couleurs!

### Progression Locale = Le Mélange sur la Toile

```
Accord I:   🟡🟠🔴  (Jaune + Orange + Rouge)
Accord vi:  🔵🟡🟠  (Bleu + Jaune + Orange)
Accord IV:  🟢🔵🟡  (Vert + Bleu + Jaune)
Accord V:   🔴🟢🟢  (Rouge + Vert + Vert)
```

Les **couleurs changent**, mais toutes viennent de la **même palette**!

---

## 📊 Comparaison Analogies

| Concept | Tonalité Globale | Progression Locale |
|---------|------------------|-------------------|
| **Architecture** | La Maison | Les Pièces |
| **Langue** | Le Français | Les Phrases |
| **Cuisine** | Les Ingrédients | Les Plats |
| **Peinture** | La Palette | Les Mélanges |
| **Film** | Le Lieu | Les Scènes |

**Conclusion**: Vous ne changez pas de maison pendant un film, mais les personnages changent de pièce!

---

## 🔮 Futur: Modulation

### Si on implémente le changement de tonalité:

```
00:00  ┌──────────────────────────────────┐
       │  Tonalité: F Pm                  │
       │  Accords:  I → vi → IV → V (×4)  │
       └──────────────────────────────────┘

02:00  ┌──────────────────────────────────┐
       │  Tonalité: G Pm  ← MODULATION!   │
       │  Accords:  I → vi → IV → V (×4)  │
       └──────────────────────────────────┘

04:00  ┌──────────────────────────────────┐
       │  Tonalité: A Pm  ← MODULATION!   │
       │  Accords:  I → vi → IV → V (×4)  │
       └──────────────────────────────────┘
```

**Effet musical**: Comme changer de pièce dans un autre bâtiment!

---

*Diagramme Explicatif - Harmonium v0.2.0*
