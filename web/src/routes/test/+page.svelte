<script lang="ts">
    import { onMount } from 'svelte';
    import init, { start } from 'harmonium';
    import EuclideanCircle from '$lib/components/EuclideanCircle.svelte';
    import { ai, aiStatus, aiError } from '$lib/ai';
    import abcjs from 'abcjs';

    let handle: any = null;
    let status = "Ready to play";
    let isPlaying = false;
    let error = "";
    let sessionInfo = "";

    // Paramètres émotionnels (modèle dimensionnel)
    let arousal = 0.5;  // Activation/Énergie → BPM
    let valence = 0.3;  // Positif/Négatif → Harmonie
    let density = 0.5;  // Complexité rythmique (< 0.3 = Carré, > 0.3 = Hexagone)
    let tension = 0.3;  // Dissonance harmonique (> 0.3 active le Triangle → polyrythme 4:3)

    // Algorithme rythmique (0 = Euclidean 16 steps, 1 = PerfectBalance 48 steps)
    let algorithm = 0;

    // Poly steps for PerfectBalance mode (48, 96, 192...)
    let polySteps = 48;

    // Mode d'harmonie (0 = Basic, 1 = Driver)
    let harmonyMode = 1; // Default to Driver

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
    
    // AI Input
    let aiInputText = "";

    // SoundFont & Engine
    let loadedFonts: { id: number, name: string, data: Uint8Array }[] = [];
    let nextBankId = 0;
    
    // Channels: 0=Bass, 1=Lead, 2=Snare, 3=Hat
    // -1 = FM, >=0 = Bank ID
    let channelRouting = [-1, -1, -1, -1];
    let mutedChannels = [false, false, false, false];
    let channelGains = [0.6, 1.0, 0.5, 0.4]; // Bass, Lead, Snare, Hat (default gains)
    const channelNames = ["Bass", "Lead", "Snare", "Hat"];

    // Recording State
    let isRecordingWav = false;
    let isRecordingMidi = false;
    let isRecordingAbc = false;
    let abcString = "";

    function toggleWavRecording() {
        if (!handle) return;
        if (isRecordingWav) {
            handle.stop_recording_wav();
            isRecordingWav = false;
        } else {
            handle.start_recording_wav();
            isRecordingWav = true;
        }
    }

    function toggleMidiRecording() {
        if (!handle) return;
        if (isRecordingMidi) {
            handle.stop_recording_midi();
            isRecordingMidi = false;
        } else {
            handle.start_recording_midi();
            isRecordingMidi = true;
        }
    }

    function toggleAbcRecording() {
        if (!handle) return;
        if (isRecordingAbc) {
            handle.stop_recording_abc();
            isRecordingAbc = false;
        } else {
            handle.start_recording_abc();
            isRecordingAbc = true;
            abcString = ""; // Clear previous score
        }
    }

    function checkRecordings() {
        if (!handle) return;
        
        // Loop to get all finished recordings
        while (true) {
            const recording = handle.pop_finished_recording();
            if (!recording) break;
            
            const fmt = recording.format;
            const data = recording.data;
            
            if (fmt === 'abc') {
                const textDecoder = new TextDecoder();
                abcString = textDecoder.decode(data);
                // Render ABC
                setTimeout(() => {
                    abcjs.renderAbc("paper", abcString, { responsive: "resize" });
                }, 0);
            }

            const mimeType = fmt === 'wav' ? 'audio/wav' : (fmt === 'midi' ? 'audio/midi' : 'text/plain');
            const ext = fmt === 'wav' ? 'wav' : (fmt === 'midi' ? 'mid' : 'abc');

            const blob = new Blob([data], { type: mimeType });
            const url = URL.createObjectURL(blob);
            const a = document.createElement('a');
            a.href = url;
            a.download = `recording_${new Date().toISOString()}.${ext}`;
            document.body.appendChild(a);
            a.click();
            document.body.removeChild(a);
            URL.revokeObjectURL(url);
        }
    }

    onMount(() => {
        const interval = setInterval(checkRecordings, 1000);
        return () => clearInterval(interval);
    });

    async function loadSoundFont(event: Event) {
        const input = event.target as HTMLInputElement;
        if (input.files && input.files[0]) {
            const file = input.files[0];
            const buffer = await file.arrayBuffer();
            const bytes = new Uint8Array(buffer);
            
            const bankId = nextBankId++;
            loadedFonts = [...loadedFonts, { id: bankId, name: file.name, data: bytes }];
            
            if (handle) {
                // Engine running: add dynamically
                handle.add_soundfont(bankId, bytes);
                console.log(`Added SoundFont ${file.name} to Bank ${bankId}`);
            }
            
            // Auto-route to this new font for all channels that are currently FM?
            // Or just let the user choose. Let's auto-route for convenience if it's the first one.
            if (loadedFonts.length === 1) {
                channelRouting = [bankId, bankId, bankId, bankId];
                if (handle) {
                    channelRouting.forEach((mode, ch) => handle.set_channel_routing(ch, mode));
                }
            }
        }
    }

    function cycleChannelEngine(channel: number) {
        // Options: -1 (FM), then all loaded bank IDs
        const options = [-1, ...loadedFonts.map(f => f.id)];
        const currentIndex = options.indexOf(channelRouting[channel]);
        const nextIndex = (currentIndex + 1) % options.length;
        const nextValue = options[nextIndex];
        
        channelRouting[channel] = nextValue;
        if (handle) {
            handle.set_channel_routing(channel, nextValue);
        }
    }

    function toggleMute(channel: number) {
        mutedChannels[channel] = !mutedChannels[channel];
        if (handle) {
            handle.set_channel_muted(channel, mutedChannels[channel]);
        }
    }

    function updateGain(channel: number, value: number) {
        channelGains[channel] = value;
        if (handle) {
            // 0=Bass, 1=Lead, 2=Snare, 3=Hat
            if (channel === 0) handle.set_gain_bass(value);
            else if (channel === 1) handle.set_gain_lead(value);
            else if (channel === 2) handle.set_gain_snare(value);
            else if (channel === 3) handle.set_gain_hat(value);
        }
    }

    function getEngineName(routingValue: number): string {
        if (routingValue === -1) return "FM";
        const font = loadedFonts.find(f => f.id === routingValue);
        return font ? font.name : "Unknown";
    }

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

        // Sync harmony mode from backend
        harmonyMode = handle.get_harmony_mode();
        
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
            }
        }
        
        requestAnimationFrame(animate);
    }

    // Mise à jour en temps réel lors du drag du slider
    function updateParams() {
        if (handle && isPlaying) {
            handle.set_arousal(arousal);
            handle.set_valence(valence);
            handle.set_density(density);
            handle.set_tension(tension);
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
            handle = start(undefined);
            
            // Load all fonts
            for (const font of loadedFonts) {
                handle.add_soundfont(font.id, font.data);
            }
            
            // Apply initial routing
            channelRouting.forEach((mode, ch) => {
                handle.set_channel_routing(ch, mode);
            });
            
            // Initialiser les paramètres émotionnels
            handle.set_arousal(arousal);
            handle.set_valence(valence);
            handle.set_density(density);
            handle.set_tension(tension);
            handle.set_algorithm(algorithm);
            handle.set_harmony_mode(harmonyMode);
            handle.set_poly_steps(polySteps);

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

    // AI Input Handling
    let debounceTimer: any;
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
                        <p class="text-xs text-neutral-500">Geometric polygons - Perfect polyrhythms</p>
                    </div>
                </label>

                {#if algorithm === 1}
                    <div class="mt-3 p-3 bg-purple-900/20 rounded-lg border border-purple-500/30">
                        <label class="block text-xs text-purple-300 mb-2">Resolution (steps per measure)</label>
                        <div class="flex gap-2">
                            {#each [48, 96, 192] as steps}
                                <button
                                    class="flex-1 py-2 px-3 rounded font-mono text-sm transition-colors
                                        {polySteps === steps
                                            ? 'bg-purple-600 text-white'
                                            : 'bg-neutral-800 text-neutral-400 hover:bg-neutral-700'}"
                                    onclick={() => polySteps = steps}
                                >
                                    {steps}
                                </button>
                            {/each}
                        </div>
                        <p class="text-xs text-neutral-500 mt-2">
                            {#if polySteps === 48}
                                Standard - Good for most polyrhythms
                            {:else if polySteps === 96}
                                High - Finer subdivisions (32nd notes)
                            {:else}
                                Ultra - Maximum precision (64th notes)
                            {/if}
                        </p>
                    </div>
                {/if}
            </div>
        </div>

        <!-- Harmony Mode Selection (only when stopped) -->
        <div class="mb-6 p-4 bg-neutral-800 rounded-xl border border-neutral-700 w-80">
            <h3 class="text-sm font-semibold text-neutral-400 mb-3 text-center">Harmony Engine</h3>
            <div class="flex flex-col gap-2">
                <label class="flex items-center gap-3 p-3 rounded-lg cursor-pointer transition-colors
                    {harmonyMode === 0 ? 'bg-green-500/20 border border-green-500' : 'bg-neutral-700/50 border border-transparent hover:bg-neutral-700'}">
                    <input
                        type="radio"
                        name="harmonyMode"
                        value={0}
                        bind:group={harmonyMode}
                        class="w-4 h-4 accent-green-500"
                    />
                    <div>
                        <span class="font-semibold {harmonyMode === 0 ? 'text-green-400' : 'text-neutral-300'}">Basic</span>
                        <p class="text-xs text-neutral-500">Russell Circumplex (I-IV-vi-V)</p>
                    </div>
                </label>

                <label class="flex items-center gap-3 p-3 rounded-lg cursor-pointer transition-colors
                    {harmonyMode === 1 ? 'bg-cyan-500/20 border border-cyan-500' : 'bg-neutral-700/50 border border-transparent hover:bg-neutral-700'}">
                    <input
                        type="radio"
                        name="harmonyMode"
                        value={1}
                        bind:group={harmonyMode}
                        class="w-4 h-4 accent-cyan-500"
                    />
                    <div>
                        <span class="font-semibold {harmonyMode === 1 ? 'text-cyan-400' : 'text-neutral-300'}">Driver</span>
                        <p class="text-xs text-neutral-500">Steedman + Neo-Riemannian + LCC</p>
                    </div>
                </label>
            </div>
        </div>
    {/if}

    <div class="flex flex-col items-center gap-4">
        <div class="flex gap-4">
            <button
                onclick={togglePlay}
                class="px-8 py-4 text-2xl font-semibold rounded-lg transition-colors duration-200 cursor-pointer
                    {isPlaying ? 'bg-red-600 hover:bg-red-700' : 'bg-purple-700 hover:bg-purple-800'}
                    disabled:opacity-50 disabled:cursor-not-allowed"
            >
                {isPlaying ? 'Stop Music' : 'Start Music'}
            </button>
        </div>

        {#if isPlaying}
            <div class="flex gap-4">
                <button
                    onclick={toggleWavRecording}
                    class="px-4 py-2 font-semibold rounded-lg transition-colors duration-200 cursor-pointer flex items-center gap-2
                        {isRecordingWav ? 'bg-red-500 hover:bg-red-600 text-white animate-pulse' : 'bg-neutral-700 hover:bg-neutral-600 text-neutral-300'}"
                >
                    <div class="w-3 h-3 rounded-full {isRecordingWav ? 'bg-white' : 'bg-red-500'}"></div>
                    {isRecordingWav ? 'Stop WAV' : 'Record WAV'}
                </button>

                <button
                    onclick={toggleMidiRecording}
                    class="px-4 py-2 font-semibold rounded-lg transition-colors duration-200 cursor-pointer flex items-center gap-2
                        {isRecordingMidi ? 'bg-red-500 hover:bg-red-600 text-white animate-pulse' : 'bg-neutral-700 hover:bg-neutral-600 text-neutral-300'}"
                >
                    <div class="w-3 h-3 rounded-full {isRecordingMidi ? 'bg-white' : 'bg-red-500'}"></div>
                    {isRecordingMidi ? 'Stop MIDI' : 'Record MIDI'}
                </button>

                <button
                    onclick={toggleAbcRecording}
                    class="px-4 py-2 font-semibold rounded-lg transition-colors duration-200 cursor-pointer flex items-center gap-2
                        {isRecordingAbc ? 'bg-red-500 hover:bg-red-600 text-white animate-pulse' : 'bg-neutral-700 hover:bg-neutral-600 text-neutral-300'}"
                >
                    <div class="w-3 h-3 rounded-full {isRecordingAbc ? 'bg-white' : 'bg-red-500'}"></div>
                    {isRecordingAbc ? 'Stop ABC' : 'Record ABC'}
                </button>
            </div>
        {/if}
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
                            {algorithm === 0 ? 'Euclidean Circles' : `Polygon Circles (${polySteps} steps)`}
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
                                {polySteps}-step circle enables perfect polyrhythms with finer subdivisions.
                            {/if}
                        </p>
                    </div>

                    <!-- 2. PROGRESSION HARMONIQUE -->
                    <div class="bg-neutral-800 rounded-xl p-6 shadow-xl border border-neutral-700">
                        <h2 class="text-xl font-bold mb-2 text-center text-purple-300">Harmonic Context</h2>
                        <div class="flex justify-center gap-2 mb-2">
                            <span class="text-xs px-2 py-1 rounded {harmonyMode === 0 ? 'bg-green-500/30 text-green-300' : 'bg-cyan-500/30 text-cyan-300'}">
                                {harmonyMode === 0 ? 'Basic' : 'Driver'}
                            </span>
                        </div>
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

                    <!-- Sound Engine Control -->
                    <div class="mb-6 p-4 bg-neutral-800 rounded-lg border border-neutral-700">
                        <h3 class="text-lg font-semibold mb-2">Sound Engine</h3>
                        
                        <!-- SoundFont Loader -->
                        <div class="mb-4">
                            <label class="block text-xs text-neutral-400 mb-1">SoundFonts (.sf2)</label>
                            <div class="flex flex-col gap-2">
                                <label class="cursor-pointer bg-neutral-900 border border-neutral-600 rounded px-3 py-2 text-sm text-neutral-300 hover:bg-neutral-800 transition-colors flex justify-center items-center">
                                    <span>+ Add SoundFont</span>
                                    <input type="file" accept=".sf2" onchange={loadSoundFont} class="hidden" />
                                </label>
                                {#if loadedFonts.length > 0}
                                    <div class="flex flex-col gap-1 mt-2">
                                        {#each loadedFonts as font}
                                            <div class="text-xs text-neutral-400 bg-neutral-900/50 px-2 py-1 rounded flex justify-between">
                                                <span class="truncate">{font.name}</span>
                                                <span class="text-neutral-600">Bank {font.id}</span>
                                            </div>
                                        {/each}
                                    </div>
                                {/if}
                            </div>
                        </div>

                        <!-- Channel Mixer -->
                        <div class="grid grid-cols-2 gap-3">
                            {#each channelNames as name, i}
                                <div class="p-3 bg-neutral-900 rounded-lg border border-neutral-700">
                                    <!-- Header: Name + Engine + Mute -->
                                    <div class="flex items-center gap-2 mb-2">
                                        <button
                                            class="flex-1 px-2 py-1 rounded text-sm font-medium transition-colors {channelRouting[i] !== -1 ? 'bg-blue-900/50 text-blue-200' : 'bg-neutral-800 text-neutral-400'}"
                                            onclick={() => cycleChannelEngine(i)}
                                            title="Cycle Engine"
                                        >
                                            <div class="flex justify-between items-center">
                                                <span>{name}</span>
                                                <span class="text-xs opacity-75">{getEngineName(channelRouting[i])}</span>
                                            </div>
                                        </button>
                                        <button
                                            class="w-8 h-8 rounded flex items-center justify-center transition-colors {mutedChannels[i] ? 'bg-red-500/20 text-red-400' : 'bg-neutral-800 text-neutral-500 hover:text-neutral-300'}"
                                            onclick={() => toggleMute(i)}
                                            title={mutedChannels[i] ? "Unmute" : "Mute"}
                                        >
                                            {#if mutedChannels[i]}
                                                <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M11 5L6 9H2v6h4l5 4V5z"/><line x1="23" y1="9" x2="17" y2="15"/><line x1="17" y1="9" x2="23" y2="15"/></svg>
                                            {:else}
                                                <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5"/></svg>
                                            {/if}
                                        </button>
                                    </div>
                                    <!-- Volume Slider -->
                                    <input
                                        type="range"
                                        min="0"
                                        max="1"
                                        step="0.01"
                                        value={channelGains[i]}
                                        oninput={(e) => updateGain(i, parseFloat(e.currentTarget.value))}
                                        class="w-full h-2 bg-neutral-700 rounded-lg appearance-none cursor-pointer accent-emerald-500"
                                        disabled={mutedChannels[i]}
                                    />
                                </div>
                            {/each}
                        </div>
                    </div>

                    <!-- Algorithm: Current mode indicator -->
                    <div class="mb-6 p-4 bg-neutral-900 rounded-lg border-l-4 {algorithm === 0 ? 'border-orange-500' : 'border-purple-500'}">
                        <div class="flex justify-between items-center">
                            <span class="text-lg font-semibold">Rhythm</span>
                            <span class="text-xl font-mono {algorithm === 0 ? 'text-orange-400' : 'text-purple-400'}">
                                {algorithm === 0 ? 'Euclidean' : 'PerfectBalance'}
                            </span>
                        </div>
                        <p class="text-xs text-neutral-500 mt-1">
                            {algorithm === 0 ? '16 steps - Classic Bjorklund' : `${polySteps} steps - Geometric polygons`}
                        </p>
                    </div>

                    <!-- Harmony Mode: Current mode indicator -->
                    <div class="mb-6 p-4 bg-neutral-900 rounded-lg border-l-4 {harmonyMode === 0 ? 'border-green-500' : 'border-cyan-500'}">
                        <div class="flex justify-between items-center">
                            <span class="text-lg font-semibold">Harmony</span>
                            <span class="text-xl font-mono {harmonyMode === 0 ? 'text-green-400' : 'text-cyan-400'}">
                                {harmonyMode === 0 ? 'Basic' : 'Driver'}
                            </span>
                        </div>
                        <p class="text-xs text-neutral-500 mt-1">
                            {harmonyMode === 0 ? 'Russell Circumplex quadrants' : 'Steedman + Neo-Riemannian + LCC'}
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

            {#if abcString}
                <div class="w-full bg-white rounded-xl p-6 shadow-xl border border-neutral-700 overflow-hidden mt-8">
                    <h2 class="text-xl font-bold mb-4 text-center text-black">Captured Score (ABC)</h2>
                    <div id="paper" class="w-full overflow-x-auto"></div>
                </div>
            {/if}
        </div>
    {/if}
</div>
