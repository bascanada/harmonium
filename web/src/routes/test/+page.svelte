<script lang="ts">
    import { onMount } from 'svelte';
    import init, { start } from 'harmonium';
    import EuclideanCircle from '$lib/components/EuclideanCircle.svelte';
    import LiveScore from '$lib/components/LiveScore.svelte';
    import { ai, aiStatus, aiError } from '$lib/ai';

    let handle: any = null;
    let status = "Ready to play";
    let isPlaying = false;
    let error = "";
    let sessionInfo = "";

    // Paramètres émotionnels (modèle dimensionnel)
    let arousal = 0.5;  // Activation/Énergie → BPM
    let valence = 0.3;  // Positif/Négatif → Harmonie
    let density = 0.5;  // Complexité rythmique
    let tension = 0.3;  // Dissonance harmonique

    // Algorithme rythmique (0 = Euclidean 16 steps, 1 = PerfectBalance 48 steps)
    let algorithm = 0;

    // BPM calculé (lecture seule)
    $: bpm = 70 + (arousal * 110);

    // État harmonique (progression)
    let currentChord = "I";
    let currentMeasure = 1;
    let currentCycle = 1;
    let currentStep = 0;
    // Continuous step counter for polyrhythmic visualization
    let totalSteps = 0;
    let lastEngineStep = -1;

    let isMinorChord = false;
    let progressionName = "Folk Peaceful (I-IV-I-V)";
    let progressionLength = 4;
    let progressionChords: string[] = []; // Dynamiquement construit

    // État Visualisation Rythmique
    let primarySteps = 16;
    let primaryPulses = 4;
    let primaryRotation = 0;
    let primaryPattern: boolean[] = [];

    let secondarySteps = 12;
    let secondaryPulses = 3;
    let secondaryRotation = 0;
    let secondaryPattern: boolean[] = [];
    
    // État Partition Live
    let notesData: { key: string, duration: string, type: 'bass' | 'lead', measure: number }[] = [];
    let lastMeasure = 1;

    // AI Input
    let aiInputText = "";

    // Helper MIDI -> Note Name
    function midiToNoteName(midi: number): string {
        const notes = ["c", "c#", "d", "d#", "e", "f", "f#", "g", "g#", "a", "a#", "b"];
        const octave = Math.floor(midi / 12) - 1;
        const note = notes[midi % 12];
        return `${note}/${octave}`;
    }

    // Animation Loop (remplace setInterval)
    function animate() {
        if (!handle || !isPlaying) return;

        // 1. Poll Harmony & Rhythm State
        const rawStep = handle.get_current_step();
        
        // Update continuous step counter
        if (rawStep !== lastEngineStep) {
            let delta = rawStep - lastEngineStep;
            // Handle wrap-around (use actual steps from engine)
            if (delta < 0) {
                delta += primarySteps;
            }
            // First tick initialization
            if (lastEngineStep === -1) {
                totalSteps = rawStep;
            } else {
                totalSteps += delta;
            }
            lastEngineStep = rawStep;
        }
        currentStep = rawStep;

        primarySteps = handle.get_primary_steps();
        primaryPulses = handle.get_primary_pulses();
        primaryRotation = handle.get_primary_rotation();
        // Convertir Uint8Array en boolean[]
        const rawPrimaryPattern = handle.get_primary_pattern();
        primaryPattern = Array.from(rawPrimaryPattern).map(v => v === 1);

        secondarySteps = handle.get_secondary_steps();
        secondaryPulses = handle.get_secondary_pulses();
        secondaryRotation = handle.get_secondary_rotation();
        const rawSecondaryPattern = handle.get_secondary_pattern();
        secondaryPattern = Array.from(rawSecondaryPattern).map(v => v === 1);
        
        currentChord = handle.get_current_chord_name();
        currentMeasure = handle.get_current_measure();
        currentCycle = handle.get_current_cycle();
        isMinorChord = handle.is_current_chord_minor();
        progressionName = handle.get_progression_name();
        
        const newLength = handle.get_progression_length();
        if (newLength !== progressionLength) {
            progressionLength = newLength;
            progressionChords = Array(progressionLength).fill("?");
        }
        
        const chordIndex = handle.get_current_chord_index();
        if (chordIndex < progressionChords.length) {
            progressionChords[chordIndex] = currentChord;
            progressionChords = [...progressionChords];
        }

        // 2. Poll Events (Notes)
        const events = handle.get_events(); // Uint32Array [note, instr, step, dur, ...]
        if (events.length > 0) {
            for (let i = 0; i < events.length; i += 4) {
                const midi = events[i];
                const instr = events[i+1]; // 0=Bass, 1=Lead
                // const step = events[i+2];
                // const dur = events[i+3];
                
                const key = midiToNoteName(midi);
                const type = instr === 0 ? 'bass' : 'lead';
                
                // Ajouter à la partition avec le numéro de mesure
                // notesData = [...notesData, { key, duration: "16", type, measure: currentMeasure }];
            }
        }
        
        // Scroll automatique vers la droite si nécessaire (géré par le composant ou CSS)
        // On ne vide plus notesData pour garder l'historique
        
        requestAnimationFrame(animate);
    }

    // Mise à jour en temps réel lors du drag du slider
    function updateParams() {
        if (handle && isPlaying) {
            handle.set_params(arousal, valence, density, tension);
        }
    }


    async function togglePlay() {
        if (isPlaying) {
            if (handle) {
                handle.free();
                handle = null;
            }
            isPlaying = false;
            status = "Stopped";
            sessionInfo = "";
            return;
        }

        try {
            const AudioContext = window.AudioContext || (window as any).webkitAudioContext;
            if (!AudioContext) {
                throw new Error("Web Audio API is not supported in this browser");
            }
            
            await init();
            handle = start();
            
            // Initialiser les paramètres émotionnels
            handle.set_params(arousal, valence, density, tension);
            handle.set_algorithm(algorithm);

            const key = handle.get_key();
            const scale = handle.get_scale();
            
            sessionInfo = `${key} ${scale}`;
            
            // Reset counters
            totalSteps = 0;
            lastEngineStep = -1;

            isPlaying = true;
            status = "Playing - Tweak the sliders!";
            error = "";
            
            // Démarrer la boucle d'animation
            requestAnimationFrame(animate);
        } catch (e) {
            console.error(e);
            error = String(e);
            status = "Error occurred";
        }
    }

    function reloadPage() {
        window.location.reload();
    }

    // AI Input Handling
    let debounceTimer: number;
    async function analyzeText() {
        if (!aiInputText) return;
        
        clearTimeout(debounceTimer);
        debounceTimer = setTimeout(async () => {
            if ($aiStatus === 'idle' || $aiStatus === 'error') {
                await ai.init();
            }
            
            if ($aiStatus === 'ready') {
                const params = await ai.predictParameters(aiInputText);
                if (params) {
                    console.log("Applying AI Params:", params);
                    arousal = params.arousal;
                    valence = params.valence;
                    tension = params.tension;
                    density = params.density;
                    updateParams();
                } else {
                    console.warn("AI could not determine parameters for this input.");
                }
            }
        }, 600);
    }
</script>

<div class="flex flex-col items-center justify-center min-h-screen bg-neutral-900 text-neutral-100 font-sans p-8">
    <h1 class="text-4xl font-bold mb-2">Harmonium</h1>
    <p class="text-neutral-400 mb-8">Morphing Music Engine</p>
    
    {#if !isPlaying}
        <!-- Algorithm Selection (only when stopped) -->
        <div class="mb-6 p-4 bg-neutral-800 rounded-xl border border-neutral-700 w-80">
            <h3 class="text-sm font-semibold text-neutral-400 mb-3 text-center">Rhythm Algorithm</h3>
            <div class="flex flex-col gap-2">
                <label class="flex items-center gap-3 p-3 rounded-lg cursor-pointer transition-colors
                    {algorithm === 0 ? 'bg-orange-500/20 border border-orange-500' : 'bg-neutral-700/50 border border-transparent hover:bg-neutral-700'}">
                    <input
                        type="radio"
                        name="algorithm"
                        value={0}
                        bind:group={algorithm}
                        class="w-4 h-4 accent-orange-500"
                    />
                    <div>
                        <span class="font-semibold {algorithm === 0 ? 'text-orange-400' : 'text-neutral-300'}">Euclidean</span>
                        <p class="text-xs text-neutral-500">16 steps - Classic Bjorklund grooves</p>
                    </div>
                </label>

                <label class="flex items-center gap-3 p-3 rounded-lg cursor-pointer transition-colors
                    {algorithm === 1 ? 'bg-purple-500/20 border border-purple-500' : 'bg-neutral-700/50 border border-transparent hover:bg-neutral-700'}">
                    <input
                        type="radio"
                        name="algorithm"
                        value={1}
                        bind:group={algorithm}
                        class="w-4 h-4 accent-purple-500"
                    />
                    <div>
                        <span class="font-semibold {algorithm === 1 ? 'text-purple-400' : 'text-neutral-300'}">PerfectBalance</span>
                        <p class="text-xs text-neutral-500">48 steps - Perfect 4:3 polyrhythms</p>
                    </div>
                </label>
            </div>
        </div>
    {/if}

    <div class="flex gap-4">
        <button
            onclick={togglePlay}
            class="px-8 py-4 text-2xl font-semibold rounded-lg transition-colors duration-200 cursor-pointer
                {isPlaying ? 'bg-red-600 hover:bg-red-700' : 'bg-purple-700 hover:bg-purple-800'}
                disabled:opacity-50 disabled:cursor-not-allowed"
        >
            {isPlaying ? 'Stop Music' : 'Start Music'}
        </button>

        <button
            onclick={reloadPage}
            class="px-8 py-4 text-2xl font-semibold rounded-lg transition-colors duration-200 cursor-pointer bg-neutral-700 hover:bg-neutral-600"
        >
            New Seed
        </button>
    </div>

    {#if sessionInfo}
        <div class="mt-2 flex flex-col items-center gap-2">
            <div class="text-purple-300 text-xl font-mono">
                Global Key: {sessionInfo}
            </div>
            <div class="text-xs text-neutral-500">
                (The "home" tonality - stays constant during session)
            </div>
        </div>
    {/if}
    
    {#if error}
        <div class="mt-4 text-red-400 max-w-md text-center">
            {error}
        </div>
    {/if}

    {#if isPlaying}
        <div class="w-full max-w-6xl mt-8 flex flex-col gap-8">
            
            <!-- TOP SECTION: VISUALS & CONTROLS -->
            <div class="grid grid-cols-1 lg:grid-cols-2 gap-8 w-full">
                
                <!-- COLONNE GAUCHE : VISUALISATION (Cercles + Harmonie) -->
                <div class="flex flex-col gap-6">
                    
                    <!-- 1. CERCLES EUCLIDIENS (Polyrythmie) -->
                    <div class="bg-neutral-800 rounded-xl p-6 shadow-xl border border-neutral-700">
                        <h2 class="text-xl font-bold mb-4 text-center text-purple-300">
                            {algorithm === 0 ? 'Euclidean Circles' : 'Polygon Circles (48 steps)'}
                        </h2>
                        <div class="flex flex-wrap justify-center items-center gap-4">
                            <EuclideanCircle
                                steps={primarySteps}
                                pulses={primaryPulses}
                                rotation={primaryRotation}
                                externalPattern={primaryPattern.length > 0 ? primaryPattern : null}
                                color="#ff3e00"
                                label={algorithm === 0 ? "PRIMARY" : "POLYRHYTHM"}
                                currentStep={totalSteps}
                                radius={algorithm === 0 ? 120 : 150}
                            />
                            {#if algorithm === 0}
                                <!-- Euclidean mode: 2 cercles indépendants (polyrythme 16:12) -->
                                <EuclideanCircle
                                    steps={secondarySteps}
                                    pulses={secondaryPulses}
                                    rotation={secondaryRotation}
                                    externalPattern={secondaryPattern.length > 0 ? secondaryPattern : null}
                                    color="#4ade80"
                                    label="SECONDARY"
                                    currentStep={totalSteps}
                                    radius={120}
                                />
                            {/if}
                        </div>
                        <p class="text-xs text-center text-neutral-500 mt-4">
                            {#if algorithm === 0}
                                Observe how the two rings rotate against each other based on Tension.
                            {:else}
                                48-step circle enables perfect 4:3 polyrhythms (Square + Triangle).
                            {/if}
                        </p>
                    </div>

                    <!-- 2. PROGRESSION HARMONIQUE -->
                    <div class="bg-neutral-800 rounded-xl p-6 shadow-xl border border-neutral-700">
                        <h2 class="text-xl font-bold mb-2 text-center text-purple-300">Harmonic Context</h2>
                        <p class="text-xs text-neutral-400 text-center mb-4">{progressionName}</p>
                        
                        <div class="flex justify-center items-center gap-3 flex-wrap mb-4">
                            {#each progressionChords as chord, index}
                                <div class="flex items-center">
                                    <div class="w-12 h-12 rounded-full flex items-center justify-center text-lg font-bold transition-all duration-300
                                        {currentChord === chord 
                                            ? 'bg-purple-600 text-white scale-110 shadow-lg shadow-purple-500/50' 
                                            : 'bg-neutral-700 text-neutral-400'}"
                                    >
                                        {chord || '?'}
                                    </div>
                                    {#if index < progressionChords.length - 1}
                                        <div class="text-neutral-600 text-sm mx-1">→</div>
                                    {/if}
                                </div>
                            {/each}
                        </div>
                        
                        <div class="flex justify-between items-center bg-neutral-900/50 rounded p-2">
                            <span class="text-sm text-neutral-400">Measure {currentMeasure}</span>
                            <span class="text-2xl font-bold {isMinorChord ? 'text-blue-400' : 'text-yellow-400'}">{currentChord}</span>
                        </div>
                    </div>
                </div>

                <!-- COLONNE DROITE : CONTROLES -->
                <div class="bg-neutral-800 rounded-xl p-8 shadow-xl h-fit sticky top-8">
                    <h2 class="text-2xl font-bold mb-2 text-center">Emotional Controls</h2>
                    <p class="text-sm text-neutral-400 text-center mb-6">Russell's Circumplex Model</p>
                    
                    <!-- AI Control -->
                    <div class="mb-6 p-4 bg-neutral-800 rounded-lg border border-neutral-700">
                        <h3 class="text-lg font-semibold mb-2">AI Director</h3>
                        <div class="flex gap-2">
                            <input 
                                type="text" 
                                bind:value={aiInputText} 
                                placeholder="Enter a list of words to describe emotions (e.g. 'battle fire danger')"
                                class="flex-1 bg-neutral-900 border border-neutral-600 rounded px-3 py-2 text-white"
                                onkeydown={(e) => e.key === 'Enter' && analyzeText()}
                            />
                            <button 
                                onclick={analyzeText}
                                disabled={$aiStatus === 'loading'}
                                class="bg-purple-600 hover:bg-purple-700 text-white px-4 py-2 rounded disabled:opacity-50"
                            >
                                {$aiStatus === 'loading' ? '...' : 'Set'}
                            </button>
                        </div>
                        {#if $aiError}
                            <div class="text-red-400 text-xs mt-2">{$aiError}</div>
                        {/if}
                        {#if $aiStatus === 'ready' && !aiInputText}
                            <div class="text-green-400 text-xs mt-2">AI Engine Ready</div>
                        {/if}
                    </div>

                    <!-- Algorithm: Current mode indicator -->
                    <div class="mb-6 p-4 bg-neutral-900 rounded-lg border-l-4 {algorithm === 0 ? 'border-orange-500' : 'border-purple-500'}">
                        <div class="flex justify-between items-center">
                            <span class="text-lg font-semibold">Algorithm</span>
                            <span class="text-xl font-mono {algorithm === 0 ? 'text-orange-400' : 'text-purple-400'}">
                                {algorithm === 0 ? 'Euclidean' : 'PerfectBalance'}
                            </span>
                        </div>
                        <p class="text-xs text-neutral-500 mt-1">
                            {algorithm === 0 ? '16 steps - Classic Bjorklund' : '48 steps - Geometric polygons 4:3'}
                        </p>
                    </div>

                    <!-- BPM Display -->
                    <div class="mb-6 p-4 bg-neutral-900 rounded-lg border-l-4 border-purple-600">
                        <div class="flex justify-between items-center">
                            <span class="text-lg font-semibold">BPM</span>
                            <span class="text-3xl font-mono text-purple-400">{bpm.toFixed(0)}</span>
                        </div>
                    </div>

                    <!-- Arousal -->
                    <div class="mb-6">
                        <div class="flex justify-between mb-2">
                            <label for="arousal" class="text-lg font-semibold">Arousal</label>
                            <span class="text-purple-400 font-mono">{arousal.toFixed(2)}</span>
                        </div>
                        <input 
                            id="arousal" type="range" min="0" max="1" step="0.01" 
                            bind:value={arousal} oninput={updateParams}
                            class="w-full h-3 bg-neutral-700 rounded-lg appearance-none cursor-pointer accent-red-600"
                        />
                        <div class="text-xs text-neutral-500 mt-1 text-right">Energy / Tempo</div>
                    </div>

                    <!-- Valence -->
                    <div class="mb-6">
                        <div class="flex justify-between mb-2">
                            <label for="valence" class="text-lg font-semibold">Valence</label>
                            <span class="text-purple-400 font-mono">{valence.toFixed(2)}</span>
                        </div>
                        <input 
                            id="valence" type="range" min="-1" max="1" step="0.01" 
                            bind:value={valence} oninput={updateParams}
                            class="w-full h-3 bg-neutral-700 rounded-lg appearance-none cursor-pointer accent-green-600"
                        />
                        <div class="text-xs text-neutral-500 mt-1 text-right">Emotion / Harmony</div>
                    </div>

                    <!-- Density -->
                    <div class="mb-6">
                        <div class="flex justify-between mb-2">
                            <label for="density" class="text-lg font-semibold">Density</label>
                            <span class="text-purple-400 font-mono">{density.toFixed(2)}</span>
                        </div>
                        <input 
                            id="density" type="range" min="0" max="1" step="0.01" 
                            bind:value={density} oninput={updateParams}
                            class="w-full h-3 bg-neutral-700 rounded-lg appearance-none cursor-pointer accent-blue-600"
                        />
                        <div class="text-xs text-neutral-500 mt-1 text-right">Rhythm Complexity</div>
                    </div>

                    <!-- Tension -->
                    <div class="mb-6">
                        <div class="flex justify-between mb-2">
                            <label for="tension" class="text-lg font-semibold">Tension</label>
                            <span class="text-purple-400 font-mono">{tension.toFixed(2)}</span>
                        </div>
                        <input 
                            id="tension" type="range" min="0" max="1" step="0.01" 
                            bind:value={tension} oninput={updateParams}
                            class="w-full h-3 bg-neutral-700 rounded-lg appearance-none cursor-pointer accent-yellow-600"
                        />
                        <div class="text-xs text-neutral-500 mt-1 text-right">Dissonance / Rotation</div>
                    </div>

                </div>
            </div>

            <!-- BOTTOM SECTION: LIVE SCORE (Full Width) -->
            <!-- 
            <div class="w-full bg-neutral-800 rounded-xl p-6 shadow-xl border border-neutral-700 overflow-hidden">
                <h2 class="text-xl font-bold mb-4 text-center text-green-300">Live Score</h2>
                <div class="flex justify-center overflow-x-auto">
                    <LiveScore notesData={notesData} />
                </div>
                <p class="text-xs text-center text-neutral-500 mt-2">
                    Real-time generated notes (Bass = Red, Lead = Green)
                </p>
            </div> 
            -->
        </div>
    {/if}
</div>
