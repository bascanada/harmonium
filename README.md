# Harmonium: a library for procedural music generation

Harmonium is a rust library to create procedural music with the help of math based
on emotions.

Design to be used in applications, websites and games.

## Library used

* FunDSP
* rust-music-theory
* cpal

## Method used

Harmonium is based on multiple layers.

1. The brain        Driver of the emotion (Russel)
2. The squelton     Geometrics rythms (Eucledien)
3. The body         Adaptive chords progression (Jean Guy)
4. The voice        Probabilistic Melody (Markov)
5. The lung         DSP

### Brain

Every thing start with the state of the system (the emotion of the audience)

1. Arousal          Affect the speed and distrosion
2. Valence          Affect the chords and the spacing
3. Tension          Affect the dissonance, the filtering and the geometry of rythms
4. Density          Affect the number of notes

### Squelton

    Algorithme de Bjorklund : Répartit les notes ("pulses") le plus équitablement possible dans la mesure ("steps"). C'est ce qui crée des rythmes "groovy" naturels (ex: 3 coups sur 8 = Tresillo).

    Polyrythmie (Steve Reich) : Deux séquenceurs tournent en parallèle. Le premier fait 16 pas, le second 12. Cela crée un déphasage qui évolue dans le temps.

    Rotation (Necklace vs Bracelet) : La Tension change le point de départ du cercle rythmique. Une même distribution de notes sonne très différemment si on la décale (rotation).

### The body

    Palettes Émotionnelles : Le système sélectionne une suite d'accords (I-IV-V, i-vii°, etc.) selon le quadrant émotionnel (ex: "Triste & Tendu" vs "Heureux & Calme").

    Inertie (Hystérésis) : Pour éviter que la musique ne change de "style" chaotiquement, le système attend un changement émotionnel significatif avant de changer de progression.

    Contexte Local : À chaque instant, le système sait quel est l'accord courant (ex: Do Majeur) et quelles sont ses notes constitutives (Do, Mi, Sol).

### The voice

    Poids Décisionnels : Pour choisir la note suivante, le système regarde où il est (Temps fort ? Note tonique ?) et tire au sort parmi des mouvements probables.

        Exemple : Sur un temps fort, il favorise les notes de l'accord (stabilité). Sur un temps faible, il autorise les notes de passage.

    Gap Fill (Temperley) : Si la mélodie fait un grand saut vers le haut, le système force statistiquement la prochaine note à redescendre pour équilibrer la ligne mélodique.

### The lung

Le son est sculpté en temps réel via fundsp :

    Synthèse FM : Utilise un modulateur et une porteuse. Plus la Tension monte, plus le ratio FM augmente, créant des sons inharmoniques (type cloche/métallique).

    Articulation (Anti-Legato) : La durée des notes change dynamiquement.

        Basse tension = Notes longues et liées (Legato).

        Haute tension = Notes courtes et percussives (Staccato).

## Usefull sources
