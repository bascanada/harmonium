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
            
            const key = handle.get_key();
            const scale = handle.get_scale();
            const pulses = handle.get_pulses();
            const steps = handle.get_steps();
            
            sessionInfo = `${key} ${scale} | Pulses: ${pulses}/${steps}`;
            
            isPlaying = true;
            status = "Playing - Tweak the sliders!";
            error = "";
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
        <div class="mt-2 text-purple-300 text-xl font-mono">
            {sessionInfo}
        </div>
    {/if}
    
    {#if error}
        <div class="mt-4 text-red-400 max-w-md text-center">
            {error}
        </div>
    {/if}

    {#if isPlaying}
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
