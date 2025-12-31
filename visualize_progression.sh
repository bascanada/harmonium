#!/bin/bash
# Visualisation en temps réel de la progression harmonique

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🎼 VISUALISATION: Progression Harmonique I-vi-IV-V"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "📊 Cycle de 8 mesures:"
echo ""
echo "   ┌───────────────────────────────────────────────┐"
echo "   │  I   →  vi   →  IV   →   V   →  (retour I)  │"
echo "   │ Repos  Couleur  Prep   Tension   Résolution  │"
echo "   └───────────────────────────────────────────────┘"
echo ""
echo "🎛️  Légende des logs:"
echo "   🎵 = Changement d'accord"
echo "   🎭 = Changement émotionnel (Arousal/Valence)"
echo "   🔄 = Morphing rythmique (Density)"
echo "   🔀 = Rotation géométrique (Tension)"
echo ""
echo "⏱️  Lancement pour 45 secondes..."
echo "   (Attendez ~2-3 cycles complets)"
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Capture avec timestamps et couleurs
timeout 45 cargo run --release 2>&1 | \
  grep --line-buffered -E "(Session|🎵|🎭|🔄|🔀)" | \
  while IFS= read -r line; do
    echo "[$(date +%H:%M:%S)] $line"
  done

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ Visualisation terminée!"
echo ""
echo "📈 Analyse attendue:"
echo "   ✓ 4 changements d'accord par cycle (I→vi→IV→V)"
echo "   ✓ Retour cyclique à I (Tonique = résolution)"
echo "   ✓ Tempo réactif à Arousal (70-180 BPM)"
echo "   ✓ Vitesse de changement réactive à Valence"
echo ""
echo "🎯 Si vous voyez ces patterns:"
echo "   → La progression harmonique fonctionne! ✨"
echo ""
echo "📝 Prochaine étape suggérée:"
echo "   • Écouter pendant 2-3 minutes"
echo "   • Identifier les cycles (sensation de 'retour')"
echo "   • Comparer avec l'ancienne version (texture monotone)"
echo ""
