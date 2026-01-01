<script lang="ts">
  import { onMount, afterUpdate } from 'svelte';
  import { Renderer, Stave, StaveNote, Voice, Formatter, Accidental, StaveConnector } from 'vexflow';

  export let notesData: { key: string, duration: string, type: 'bass' | 'lead', measure: number }[] = [];

  let container: HTMLDivElement;
  let renderer, context;
  let scrollContainer: HTMLDivElement;

  function render() {
    if (!container) return;
    container.innerHTML = ''; // Clear précédent

    // 1. Grouper les notes par mesure
    // On suppose que les mesures sont contiguës et croissantes
    const measures = new Map<number, typeof notesData>();
    if (notesData.length > 0) {
        const firstMeasure = notesData[0].measure;
        const lastMeasure = notesData[notesData.length - 1].measure;
        
        // Initialiser les mesures vides entre le début et la fin pour éviter les trous
        for (let m = firstMeasure; m <= lastMeasure; m++) {
            measures.set(m, []);
        }
        
        notesData.forEach(n => {
            if (measures.has(n.measure)) {
                measures.get(n.measure)?.push(n);
            }
        });
    }

    const measureWidth = 300;
    const totalWidth = Math.max(500, measures.size * measureWidth + 50);
    const height = 300; // Plus grand pour le Grand Staff

    // Setup Renderer
    renderer = new Renderer(container, Renderer.Backends.SVG);
    renderer.resize(totalWidth, height);
    context = renderer.getContext();

    // Dessiner chaque mesure
    let x = 10;
    let isFirst = true;

    // Trier les clés de mesures pour l'ordre d'affichage
    const sortedMeasureKeys = Array.from(measures.keys()).sort((a, b) => a - b);

    sortedMeasureKeys.forEach((measureNum) => {
        const measureNotes = measures.get(measureNum) || [];
        
        // --- TREBLE STAVE (Lead) ---
        const topStave = new Stave(x, 40, measureWidth);
        if (isFirst) {
            topStave.addClef("treble").addTimeSignature("4/4");
            topStave.setText("Lead", 3); // Label
        }
        topStave.setContext(context).draw();

        // --- BASS STAVE (Bass) ---
        const bottomStave = new Stave(x, 160, measureWidth);
        if (isFirst) {
            bottomStave.addClef("bass").addTimeSignature("4/4");
            bottomStave.setText("Bass", 3); // Label
        }
        bottomStave.setContext(context).draw();

        // --- CONNECTORS ---
        const brace = new StaveConnector(topStave, bottomStave).setType(isFirst ? 'brace' : 'singleLeft');
        brace.setContext(context).draw();
        
        const lineRight = new StaveConnector(topStave, bottomStave).setType('singleRight');
        lineRight.setContext(context).draw();

        // --- NOTES ---
        const leadNotes = measureNotes.filter(n => n.type === 'lead').map(n => {
             const note = new StaveNote({ 
                keys: [n.key], 
                duration: n.duration,
                clef: "treble" 
            });
            note.setStyle({fillStyle: "#4ade80", strokeStyle: "#4ade80"});
            if (n.key.includes("#")) note.addModifier(new Accidental("#"));
            return note;
        });

        const bassNotes = measureNotes.filter(n => n.type === 'bass').map(n => {
             // Convertir pour la clé de Fa si nécessaire (VexFlow gère bien les notes hautes en clé de Fa)
             // Mais on s'assure que la clé est bien passée
             const note = new StaveNote({ 
                keys: [n.key], 
                duration: n.duration,
                clef: "bass" 
            });
            note.setStyle({fillStyle: "#ff3e00", strokeStyle: "#ff3e00"});
            if (n.key.includes("#")) note.addModifier(new Accidental("#"));
            return note;
        });

        // --- VOICES & FORMATTING ---
        const voices = [];

        if (leadNotes.length > 0) {
            const voice = new Voice({ num_beats: 4, beat_value: 4 });
            voice.setStrict(false); // Permet les mesures incomplètes
            voice.addTickables(leadNotes);
            voices.push(voice);
        }

        if (bassNotes.length > 0) {
            const voice = new Voice({ num_beats: 4, beat_value: 4 });
            voice.setStrict(false);
            voice.addTickables(bassNotes);
            voices.push(voice);
        }

        if (voices.length > 0) {
            new Formatter().joinVoices(voices).format(voices, measureWidth - 50);
            voices.forEach(v => {
                // Dessiner sur la bonne portée
                // Note: VexFlow Voice ne sait pas sur quelle portée il est, on dessine les notes manuellement ou via voice.draw(ctx, stave)
                // Mais ici on a deux portées.
                // Astuce: On a séparé les notes par type avant.
                // On doit dessiner la voix Lead sur topStave et Bass sur bottomStave
                
                // On ne peut pas utiliser voice.draw(context, stave) facilement si on a mixé les voix dans le formatter ?
                // Si, le formatter aligne les ticks. Ensuite on dessine chaque voix sur sa portée.
            });
            
            // Dessin séparé après formatage commun (pour alignement vertical)
            if (voices.length === 2) {
                voices[0].draw(context, topStave);
                voices[1].draw(context, bottomStave);
            } else if (voices.length === 1) {
                // Si une seule voix, vérifier laquelle c'est
                // C'est un peu hacky, on regarde les notes
                const isLead = leadNotes.length > 0;
                voices[0].draw(context, isLead ? topStave : bottomStave);
            }
        }

        x += measureWidth;
        isFirst = false;
    });
  }

  // Réagir aux changements de notes
  $: if (notesData) render();
  
  // Auto-scroll
  afterUpdate(() => {
      if (scrollContainer) {
          scrollContainer.scrollLeft = scrollContainer.scrollWidth;
      }
  });
</script>

<div bind:this={scrollContainer} class="score-scroll-container">
    <div bind:this={container} class="score-content"></div>
</div>

<style>
  .score-scroll-container {
    width: 100%;
    overflow-x: auto;
    background: white;
    border-radius: 8px;
    padding: 10px;
  }
  
  .score-content {
      /* La largeur est définie par le contenu SVG */
      min-width: 500px; 
  }
</style>
