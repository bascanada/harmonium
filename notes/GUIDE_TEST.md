# Guide de Test - Articulation Dynamique

## 🚀 Démarrage Rapide

### Compilation et Exécution
```bash
# Compilation optimisée
make release

# Ou directement
cargo build --release

# Lancement
cargo run --release
```

### Observation des Changements
Le moteur affiche maintenant dans les logs :
- 🔄 Changements de pulses rythmiques
- 🔀 Rotations des patterns (tension)
- 🎵 Changements d'accords
- 🎼 Nouvelles progressions harmoniques
- 🎭 États émotionnels cibles

## 🎧 Ce Qu'il Faut Écouter

### Test 1 : Évolution de la Tension
**Attendu** : Le moteur change automatiquement d'état toutes les 5 secondes

Écoutez l'évolution :
1. **Démarrage** (Tension ~0.2) : Notes relativement longues
2. **Après 5-10s** : Si tension augmente → Notes plus courtes
3. **Après 15-20s** : Variations continues selon les états aléatoires

**Indicateurs auditifs** :
- Tension basse : Son continu, nappe
- Tension moyenne : Groove distinct avec respiration
- Tension haute : Notes piquées, beaucoup d'espace

### Test 2 : Espace pour les Effets
**Delay visible** : Avec articulation courte, vous devriez entendre :
- Les échos du delay (300ms) clairement séparés
- Le reverb tail entre les notes
- Les notes ne se "collent" plus ensemble

**Comparaison mentale** :
- AVANT : Brouillard sonore continu
- APRÈS : Notes distinctes avec profondeur spatiale

### Test 3 : Humanisation
**Écoutez attentivement** : Les notes ne sont jamais exactement de la même longueur
- Variation subtile (±10%)
- Évite la régularité mécanique
- Groove organique même avec pattern euclidien régulier

## 📊 Commandes de Diagnostic

### Vérifier la Compilation
```bash
cargo check
# Attendu: "Finished `dev` profile"
```

### Afficher les Logs Pertinents
```bash
cargo run --release 2>&1 | grep -E "(Tension|Rotation|Morphing)"
```

### Tester Pendant 30 Secondes
```bash
timeout 30 cargo run --release
```

### Capturer les Statistiques
```bash
cargo run --release 2>&1 | tee session_$(date +%Y%m%d_%H%M%S).log
```

## 🎮 Simulations Manuelles

### Modifier les Paramètres Initiaux
Éditez `src/engine.rs`, fonction `new()` :

```rust
// Ligne ~156 - Modifier pour tester différents états
let initial_params = EngineParams {
    arousal: 0.3,   // 0.0 = calme, 1.0 = énergique
    valence: 0.8,   // -1.0 = négatif, 1.0 = positif
    density: 0.4,   // 0.0 = épuré, 1.0 = dense
    tension: 0.2,   // 0.0 = legato, 1.0 = staccato
};
```

**Scénarios suggérés** :

#### Folk Paisible
```rust
arousal: 0.25, valence: 0.75, density: 0.3, tension: 0.15
// → BPM lent, notes longues, harmonie majeure
```

#### Pop Énergique
```rust
arousal: 0.7, valence: 0.6, density: 0.6, tension: 0.5
// → BPM rapide, groove équilibré, progression I-V-vi-IV
```

#### Anxieux/Tendu
```rust
arousal: 0.8, valence: -0.4, density: 0.5, tension: 0.9
// → BPM très rapide, notes ultra-courtes, mineur
```

#### Ambient Drone
```rust
arousal: 0.2, valence: 0.0, density: 0.2, tension: 0.1
// → BPM très lent, notes très longues, minimaliste
```

### Désactiver le Simulateur d'IA
Si vous voulez un état constant (pas de morphing) :

Éditez `src/main.rs`, commentez le thread du simulateur :
```rust
// thread::spawn(move || {
//     simulate_ai_changes(state_clone);
// });
```

## 🔍 Analyse Détaillée

### Voir l'État du Système
Le moteur affiche régulièrement :
```
🎭 EMOTION CHANGE: Arousal 0.56 → 132 BPM | Valence 0.56 | Density 0.32 | Tension 0.90
```

**Calculs mentaux** :
- BPM = 70 + (Arousal × 110)
- Articulation = 95% - (Tension × 75%)
  - Tension 0.90 → Articulation 27.5% (notes très courtes!)

### Observer les Changements de Progression
```
🎼 New Harmonic Context: Pop Energetic (I-V-vi-IV) | Valence: 0.56, Tension: 0.90
```

**Interprétation** :
- Valence positive (0.56) → Progression majeure
- Tension haute (0.90) → Mais notes courtes malgré harmonie joyeuse
- **Résultat** : Pop percussif, énergique mais avec espacement

### Suivre les Mesures et Cycles
```
🎵 Chord: vim | Measure: 25 | Progression: 3/4
```

**Signification** :
- Measure 25 = 6e cycle complet (4 mesures × 6)
- Position 3/4 dans la progression
- Accord vi mineur = pentatonique relative mineure

## 🎨 Modifications Créatives

### Changer la Formule d'Articulation
Éditez `src/engine.rs`, ligne ~480 :

```rust
// Actuel:
let articulation_ratio = 0.95 - (self.current_state.tension * 0.75);

// Plus extrême (5% → 95%):
let articulation_ratio = 0.95 - (self.current_state.tension * 0.90);

// Plus subtil (50% → 95%):
let articulation_ratio = 0.95 - (self.current_state.tension * 0.45);
```

### Modifier l'Humanisation
```rust
// Actuel: ±10%
let humanize: f32 = rng.gen_range(0.9..1.1);

// Plus naturel: ±20%
let humanize: f32 = rng.gen_range(0.8..1.2);

// Plus robotique: ±2%
let humanize: f32 = rng.gen_range(0.98..1.02);
```

### Ajouter un Seuil Minimum Plus Élevé
```rust
// Actuel: minimum 100 samples (2.3ms)
if self.current_gate_duration < 100 { 
    self.current_gate_duration = 100; 
}

// Plus long: minimum 500 samples (11ms)
if self.current_gate_duration < 500 { 
    self.current_gate_duration = 500; 
}
```

## 📈 Benchmarking

### Mesurer l'Utilisation CPU
```bash
# macOS
top -pid $(pgrep harmonium) -stats cpu,mem

# Linux
htop -p $(pgrep harmonium)
```

### Profiling Détaillé
```bash
# Avec cargo flamegraph (nécessite installation)
cargo install flamegraph
cargo flamegraph

# Ouvrir flamegraph.svg pour voir les hotspots
```

## 🐛 Troubleshooting

### Pas de Son
- Vérifiez la sortie audio : `Output device: ...`
- Vérifiez le volume système
- Essayez de redémarrer avec `cargo clean && cargo run --release`

### Son Haché
- Possible buffer underrun
- Réduire `density` (moins de notes)
- Réduire `arousal` (BPM plus lent)

### Toutes les Notes Pareilles
- L'humanisation fonctionne-t-elle ? (±10% devrait être perceptible)
- La tension change-t-elle ? (voir logs `🎭 EMOTION CHANGE`)
- Vérifier que `gate_timer` est bien utilisé

### Compilation Échoue
```bash
# Nettoyer et recompiler
cargo clean
cargo build --release

# Vérifier la version de Rust
rustc --version
# Attendu: 1.70+ ou plus récent
```

## 📚 Documentation Associée

- `ARTICULATION_DYNAMIQUE.md` - Explication technique complète
- `VISUALISATION_ARTICULATION.md` - Graphiques et exemples visuels
- `PROCHAINES_ETAPES_ADSR.md` - Roadmap des améliorations futures
- `VISUAL_SUMMARY.md` - Résumé visuel en ASCII art
- `SESSION_ARTICULATION_30DEC2024.md` - Notes de développement

## 🎓 Concepts Clés à Comprendre

### 1. Articulation vs Legato
- **Legato** : Notes liées sans interruption (ancien comportement)
- **Articulation** : Durée contrôlée avec silence (nouveau comportement)

### 2. Gate vs ADSR
- **Gate** : Signal on/off contrôlant quand la note sonne
- **ADSR** : Enveloppe définissant comment la note évolue pendant qu'elle sonne

### 3. Tension vs Valence
- **Tension** : Dissonance/Stress → Contrôle articulation
- **Valence** : Positif/Négatif → Contrôle harmonie

### 4. Arousal vs Density
- **Arousal** : Énergie → Contrôle BPM
- **Density** : Complexité → Contrôle nombre de notes (pulses)

## ✅ Checklist d'Écoute Critique

Après avoir lancé le moteur pendant 2-3 minutes :

- [ ] J'entends clairement l'espace entre les notes (quand tension > 0.5)
- [ ] Le delay crée un effet spatial audible
- [ ] Les notes ne sont pas toutes de la même longueur
- [ ] Le BPM change progressivement (morphing visible dans logs)
- [ ] Les progressions harmoniques changent environ toutes les 8 mesures
- [ ] Le son n'est PAS un mur continu et monotone
- [ ] Je peux identifier des "grooves" rythmiques distincts

Si tous les points sont cochés : ✅ L'articulation dynamique fonctionne!

## 🎉 Commandes Bonus

### Créer une Boucle de Test
```bash
# Enregistrer 10 sessions de 30s chacune
for i in {1..10}; do
    echo "=== Session $i ==="
    timeout 30 cargo run --release 2>&1 | tee "test_session_$i.log"
    sleep 2
done
```

### Extraire les Statistiques
```bash
# Compter les changements de tension
grep "EMOTION CHANGE" session.log | wc -l

# Voir la distribution des tensions
grep "Tension:" session.log | awk '{print $NF}' | sort -n
```

### Comparaison A/B
```bash
# Sauvegarder la version actuelle
git stash

# Revenir à l'ancienne version (avant articulation)
git checkout HEAD~1

# Tester l'ancien comportement
cargo run --release &
sleep 30
killall harmonium

# Revenir à la nouvelle version
git checkout -
git stash pop

# Tester le nouveau comportement
cargo run --release
```

---

**Bon test!** 🎵✨

Pour toute question, consultez la documentation dans les fichiers `.md` du projet.
