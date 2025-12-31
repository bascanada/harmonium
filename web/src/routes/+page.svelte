<script lang="ts">
    import init, { start } from 'harmonium';

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

    // BPM calculé (lecture seule)
    $: bpm = 70 + (arousal * 110);

    // État harmonique (progression)
    let currentChord = "I";
    let currentMeasure = 1;
    let currentCycle = 1;
    let currentStep = 0;
    let isMinorChord = false;

    // Polling pour mettre à jour l'état harmonique (30 FPS)
    let harmonyInterval: number | null = null;

    function startHarmonyPolling() {
        if (harmonyInterval) return;
        
        harmonyInterval = window.setInterval(() => {
            if (handle && isPlaying) {
                currentChord = handle.get_current_chord_name();
                currentMeasure = handle.get_current_measure();
                currentCycle = handle.get_current_cycle();
                currentStep = handle.get_current_step();
                isMinorChord = handle.is_current_chord_minor();
            }
        }, 33); // ~30 FPS
    }

    function stopHarmonyPolling() {
        if (harmonyInterval) {
            clearInterval(harmonyInterval);
            harmonyInterval = null;
        }
    }

    // Mise à jour en temps réel lors du drag du slider
    function updateParams() {
        if (handle && isPlaying) {
            handle.set_params(arousal, valence, density, tension);
        }
    }

    async function togglePlay() {
        if (isPlaying) {
            stopHarmonyPolling();
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
            
            const key = handle.get_key();
            const scale = handle.get_scale();
            const pulses = handle.get_pulses();
            const steps = handle.get_steps();
            
            sessionInfo = `${key} ${scale} | Pulses: ${pulses}/${steps}`;
            
            isPlaying = true;
            status = "Playing - Tweak the sliders!";
            error = "";
            
            // Démarrer le polling de l'état harmonique
            startHarmonyPolling();
        } catch (e) {
            console.error(e);
            error = String(e);
            status = "Error occurred";
        }
    }
</script>

<div class="flex flex-col items-center justify-center min-h-screen bg-neutral-900 text-neutral-100 font-sans p-8">
    <h1 class="text-4xl font-bold mb-2">🎵 Harmonium</h1>
    <p class="text-neutral-400 mb-8">Morphing Music Engine</p>
    
    <button 
        onclick={togglePlay}
        class="px-8 py-4 text-2xl font-semibold rounded-lg transition-colors duration-200 cursor-pointer
               {isPlaying ? 'bg-red-600 hover:bg-red-700' : 'bg-purple-700 hover:bg-purple-800'} 
               disabled:opacity-50 disabled:cursor-not-allowed"
    >
        {isPlaying ? '⏹ Stop Music' : '▶ Start Music'}
    </button>

    <div class="mt-6 text-neutral-400 text-lg">
        {status}
    </div>
    
    {#if sessionInfo}
        <div class="mt-2 flex flex-col items-center gap-2">
            <div class="text-purple-300 text-xl font-mono">
                🎹 Global Key: {sessionInfo}
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
        <!-- Affichage de la progression harmonique -->
        <div class="mt-8 w-full max-w-2xl bg-gradient-to-br from-purple-900 to-indigo-900 rounded-xl p-6 shadow-2xl border-2 border-purple-500">
            <h2 class="text-2xl font-bold mb-2 text-center">🎼 Harmonic Progression</h2>
            <p class="text-xs text-neutral-400 text-center mb-4">
                Local chord changes within the global key
            </p>
            
            <div class="grid grid-cols-2 gap-4 mb-6">
                <!-- Accord courant -->
                <div class="bg-neutral-900 rounded-lg p-4 text-center">
                    <div class="text-sm text-neutral-400 mb-1">Current Chord</div>
                    <div class="text-5xl font-bold {isMinorChord ? 'text-blue-400' : 'text-yellow-400'}">
                        {currentChord}
                    </div>
                    <div class="text-xs text-neutral-500 mt-2">
                        {isMinorChord ? 'Minor' : 'Major'}
                    </div>
                </div>

                <!-- Mesure et Cycle -->
                <div class="bg-neutral-900 rounded-lg p-4">
                    <div class="flex justify-between items-center mb-2">
                        <span class="text-sm text-neutral-400">Measure</span>
                        <span class="text-2xl font-mono text-green-400">{currentMeasure}</span>
                    </div>
                    <div class="flex justify-between items-center">
                        <span class="text-sm text-neutral-400">Cycle</span>
                        <span class="text-2xl font-mono text-purple-400">{currentCycle}</span>
                    </div>
                </div>
            </div>

            <!-- Progression visuelle I-vi-IV-V -->
            <div class="bg-neutral-900 rounded-lg p-4">
                <div class="text-sm text-neutral-400 mb-3 text-center">
                    Progression: I → vi → IV → V
                    <span class="text-xs block mt-1 text-neutral-600">
                        (Roman numerals = scale degrees within global key)
                    </span>
                </div>
                <div class="flex justify-between items-center">
                    {#each ['I', 'vi', 'IV', 'V'] as chord, index}
                        <div class="flex flex-col items-center">
                            <div class="w-16 h-16 rounded-full flex items-center justify-center text-xl font-bold transition-all duration-300
                                {currentChord === chord 
                                    ? 'bg-purple-600 text-white scale-110 shadow-lg shadow-purple-500/50' 
                                    : 'bg-neutral-700 text-neutral-400'}"
                            >
                                {chord}
                            </div>
                            <div class="text-xs text-neutral-500 mt-2">
                                {index === 0 ? 'Tonic' : index === 1 ? 'Relative' : index === 2 ? 'Subdominant' : 'Dominant'}
                            </div>
                        </div>
                        {#if index < 3}
                            <div class="text-neutral-600 text-2xl">→</div>
                        {/if}
                    {/each}
                </div>
            </div>

            <!-- Barre de progression de la mesure -->
            <div class="mt-4 bg-neutral-900 rounded-lg p-4">
                <div class="text-sm text-neutral-400 mb-2">Step: {currentStep}/16</div>
                <div class="w-full bg-neutral-700 rounded-full h-2 overflow-hidden">
                    <div 
                        class="bg-gradient-to-r from-purple-600 to-pink-600 h-full transition-all duration-100"
                        style="width: {(currentStep / 16) * 100}%"
                    ></div>
                </div>
            </div>
        </div>

        <!-- Panneau de contrôle en temps réel -->
        <div class="mt-12 w-full max-w-2xl bg-neutral-800 rounded-xl p-8 shadow-2xl">
            <h2 class="text-2xl font-bold mb-2 text-center">Emotional Controls</h2>
            <p class="text-sm text-neutral-400 text-center mb-6">Based on Russell's Circumplex Model</p>
            
            <!-- BPM Display (Read-only, computed from arousal) -->
            <div class="mb-6 p-4 bg-neutral-900 rounded-lg border-2 border-purple-600">
                <div class="flex justify-between items-center">
                    <span class="text-lg font-semibold">🎯 BPM (Tempo)</span>
                    <span class="text-3xl font-mono text-purple-400">{bpm.toFixed(0)}</span>
                </div>
                <p class="text-xs text-neutral-500 mt-2">
                    ⚡ Automatically computed from Arousal
                </p>
            </div>

            <!-- Arousal -->
            <div class="mb-6">
                <div class="flex justify-between mb-2">
                    <label for="arousal" class="text-lg font-semibold">🔥 Arousal (Activation)</label>
                    <span class="text-purple-400 font-mono text-lg">{arousal.toFixed(2)}</span>
                </div>
                <input 
                    id="arousal"
                    type="range" 
                    min="0" 
                    max="1" 
                    step="0.01" 
                    bind:value={arousal}
                    oninput={updateParams}
                    class="w-full h-3 bg-neutral-700 rounded-lg appearance-none cursor-pointer accent-red-600"
                />
                <div class="flex justify-between text-xs text-neutral-500 mt-1">
                    <span>Low Energy (70 BPM)</span>
                    <span>High Energy (180 BPM)</span>
                </div>
            </div>

            <!-- Valence -->
            <div class="mb-6">
                <div class="flex justify-between mb-2">
                    <label for="valence" class="text-lg font-semibold">😊 Valence (Emotion)</label>
                    <span class="text-purple-400 font-mono text-lg">{valence.toFixed(2)}</span>
                </div>
                <input 
                    id="valence"
                    type="range" 
                    min="-1" 
                    max="1" 
                    step="0.01" 
                    bind:value={valence}
                    oninput={updateParams}
                    class="w-full h-3 bg-neutral-700 rounded-lg appearance-none cursor-pointer accent-green-600"
                />
                <div class="flex justify-between text-xs text-neutral-500 mt-1">
                    <span>Negative (-1.0)</span>
                    <span>Neutral (0.0)</span>
                    <span>Positive (1.0)</span>
                </div>
            </div>

            <!-- Density -->
            <div class="mb-6">
                <div class="flex justify-between mb-2">
                    <label for="density" class="text-lg font-semibold">🥁 Density (Rhythm)</label>
                    <span class="text-purple-400 font-mono text-lg">{density.toFixed(2)}</span>
                </div>
                <input 
                    id="density"
                    type="range" 
                    min="0" 
                    max="1" 
                    step="0.01" 
                    bind:value={density}
                    oninput={updateParams}
                    class="w-full h-3 bg-neutral-700 rounded-lg appearance-none cursor-pointer accent-blue-600"
                />
                <div class="flex justify-between text-xs text-neutral-500 mt-1">
                    <span>Sparse (0.0)</span>
                    <span>Dense (1.0)</span>
                </div>
            </div>

            <!-- Tension -->
            <div class="mb-6">
                <div class="flex justify-between mb-2">
                    <label for="tension" class="text-lg font-semibold">⚡ Tension (Harmony)</label>
                    <span class="text-purple-400 font-mono text-lg">{tension.toFixed(2)}</span>
                </div>
                <input 
                    id="tension"
                    type="range" 
                    min="0" 
                    max="1" 
                    step="0.01" 
                    bind:value={tension}
                    oninput={updateParams}
                    class="w-full h-3 bg-neutral-700 rounded-lg appearance-none cursor-pointer accent-yellow-600"
                />
                <div class="flex justify-between text-xs text-neutral-500 mt-1">
                    <span>Consonant (0.0)</span>
                    <span>Dissonant (1.0)</span>
                </div>
            </div>

            <div class="mt-8 p-4 bg-neutral-900 rounded-lg">
                <p class="text-sm text-neutral-400 text-center">
                    🔄 The engine smoothly morphs between emotional states
                </p>
            </div>
        </div>
    {/if}
</div>
