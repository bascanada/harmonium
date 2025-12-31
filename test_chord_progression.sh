#!/bin/bash
# Test de la progression harmonique I-vi-IV-V

echo "🎼 TEST PROGRESSION HARMONIQUE: De texture à chanson"
echo "====================================================="
echo ""

echo "🎵 Progression implémentée: I - vi - IV - V (\"4 Chords Song\")"
echo "   Utilisée dans des milliers de chansons pop"
echo "   Références: Journey, U2, Lady Gaga, etc."
echo ""

echo "📊 Structure:"
echo "   • Mesure 1-2:  I   (Do Maj)  - Tonique (maison)"
echo "   • Mesure 3-4:  vi  (La Min)  - Relative mineure (mélancolique)"
echo "   • Mesure 5-6:  IV  (Fa Maj)  - Sous-dominante (préparation)"
echo "   • Mesure 7-8:  V   (Sol Maj) - Dominante (tension → retour I)"
echo ""

echo "🎛️  Contrôle émotionnel:"
echo "   VALENCE > 0.5 → Changements rapides (2 mesures/accord)"
echo "   VALENCE < 0.5 → Changements lents (4 mesures/accord)"
echo ""

echo "⏱️  Lancement du moteur pour 30 secondes..."
echo ""

# Lancer et capturer les changements d'accords
timeout 30 cargo run --release 2>&1 | grep -E "(Session|🎵 Chord|EMOTION)" | head -n 40 || true

echo ""
echo "✅ Test terminé!"
echo ""
echo "🎯 Vérifications attendues:"
echo "   ✓ Les accords changent cycliquement (I → vi → IV → V → I...)"
echo "   ✓ La mélodie suit l'accord courant (notes d'accord privilégiées)"
echo "   ✓ Le rythme de changement réagit à la valence"
echo "   ✓ Sensation de PROGRESSION plutôt que boucle statique"
echo ""
echo "🎼 Différence avec l'ancien système:"
echo "   AVANT: Gamme fixe (C pentatonique) → texture monotone"
echo "   APRÈS: Progression harmonique → phrases musicales cohérentes"
