<script lang="ts">
    import init, { start } from 'harmonium';

    let handle: any = null;
    let status = "Ready to play";
    let isPlaying = false;
    let error = "";
    let sessionInfo = "";

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
            
            const bpm = handle.get_bpm().toFixed(1);
            const key = handle.get_key();
            const scale = handle.get_scale();
            const pulses = handle.get_pulses();
            const steps = handle.get_steps();
            
            sessionInfo = `${key} ${scale} | BPM: ${bpm} | Pulses: ${pulses}/${steps}`;
            
            isPlaying = true;
            status = "Playing procedural music...";
            error = "";
        } catch (e) {
            console.error(e);
            error = String(e);
            status = "Error occurred";
        }
    }
</script>

<div class="flex flex-col items-center justify-center min-h-screen bg-neutral-900 text-neutral-100 font-sans">
    <h1 class="text-4xl font-bold mb-8">Harmonium</h1>
    
    <button 
        onclick={togglePlay}
        class="px-8 py-4 text-2xl font-semibold rounded-lg transition-colors duration-200 cursor-pointer
               {isPlaying ? 'bg-red-600 hover:bg-red-700' : 'bg-purple-700 hover:bg-purple-800'} 
               disabled:opacity-50 disabled:cursor-not-allowed"
    >
        {isPlaying ? 'Stop Music' : 'Start Music'}
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
</div>
