<script lang="ts">
  import { invoke, Channel } from '@tauri-apps/api/core';
  import { emit } from '@tauri-apps/api/event';
  import { error } from '@tauri-apps/plugin-log';
  import { goto } from '$app/navigation';
  
  let isProcessing = $state(false);
  let audioLevels = $state<number[]>([]);
  let recordingPromise: Promise<string> | null = null;

  async function stopRecordingAndProcess() { 
    try { 
      isProcessing = true; 
      await emit('stop-recording'); 
    } catch (err) { 
      error(String(err)); 
    } finally { 
      goto('/'); 
    } 
  }

  const audioLevelChan = new Channel<{ levels: number[] }>();
  audioLevelChan.onmessage = (m) => { audioLevels = m.levels };

  try { 
    recordingPromise = invoke('record_and_transcribe', { audioLevelChan }) as Promise<string>; 
  }
  catch (err) { 
    error(String(err)); 
    audioLevels = []; 
    recordingPromise = null; 
    goto('/'); 
  }
</script>

<main class="container-fullscreen container-centered">
  <div class="center">
    {#if isProcessing}
      <div class="loading-spinner"></div>
    {:else}
      <button class="btn bars-button" aria-label="Stop recording" onclick={stopRecordingAndProcess}>
        <div class="frequency-bars">
          {#each audioLevels as level}
            <div class="bar" style={`height: ${Math.min(level * 50, 25)}px`}></div>
          {/each}
        </div>
      </button>
    {/if}
  </div>
</main>

<style>
  .center {
    display: flex;
    justify-content: center;
    align-items: center;
  }

  .bars-button {
    width: 60px;
    height: 30px;
    display: flex;
    justify-content: center;
    align-items: center;
  }

  .loading-spinner {
    width: 20px;
    height: 20px;
    border: 4px solid var(--color-border);
    border-top: 4px solid var(--color-fg);
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
  }

  .frequency-bars {
    display: flex;
    align-items: flex-end;
    justify-content: center;
    gap: 1px;
    height: 25px;
    width: 55px;
  }

  .bar {
    width: 3px;
    min-height: 2px;
    background: var(--color-fg);
    border-radius: 1px;
    transition: height 0.1s ease;
  }
</style>
