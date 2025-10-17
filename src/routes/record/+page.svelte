<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { writeText } from '@tauri-apps/plugin-clipboard-manager';
  import { invoke, Channel } from '@tauri-apps/api/core';
  import { emit } from '@tauri-apps/api/event';
  import { warn, debug, info, error } from '@tauri-apps/plugin-log';
  import { getCurrentWindow, LogicalSize } from '@tauri-apps/api/window';

  let isRecording = $state(false);
  let isProcessing = $state(false);
  let audioLevels = $state<number[]>([]);
  let recordingPromise: Promise<string> | null = null;

  async function toggleMic() {
    if (isProcessing) return;

    try {
      if (!isRecording) {
        isRecording = true;
        info('Starting recording...');

        const audioLevelChan = new Channel<{ levels: number[] }>();
        audioLevelChan.onmessage = (message) => {
          audioLevels = message.levels;
          debug(`Audio levels: ${message.levels}`);
        };

        recordingPromise = invoke('record_and_transcribe', { audioLevelChan }) as Promise<string>;
      } else {
        isProcessing = true;
        info('Stopping recording...');
        await emit('stop-recording');

        if (recordingPromise) {
          try {
            const transcription = await recordingPromise;
            if (transcription && transcription.trim()) {
              await writeText(transcription);
              info(`Transcription copied to clipboard: ${transcription}`);
            } else {
              warn('No transcription received');
            }
          } catch (recordingError) {
            error(`Recording error: ${recordingError}`);
          }
        }

        isRecording = false;
        isProcessing = false;
        audioLevels = [];
        recordingPromise = null;
      }
    } catch (err) {
      error(`Error: ${err}`);
      isRecording = false;
      isProcessing = false;
      audioLevels = [];
      recordingPromise = null;
    }
  }

  function goBack() {
    goto('/');
  }
</script>

<main class="container">
  <div class="top">
    <button class="back" aria-label="Back" onclick={goBack}>⟵</button>
  </div>
  <div class="center">
    <button class="mic-button" onclick={toggleMic} class:recording={isRecording} class:processing={isProcessing}>
      {#if isProcessing}
        <div class="loading-spinner"></div>
      {:else if isRecording}
        <div class="frequency-bars">
          {#each audioLevels as level}
            <div class="bar" style={`height: ${Math.min(level * 50, 25)}px`}></div>
          {/each}
        </div>
      {:else}
        <img src="/favicon.svg" alt="Microphone" />
      {/if}
    </button>
  </div>
</main>

<style>
:global(html), :global(body) {
  margin: 0;
  padding: 0;
  background: #000;
}

.container {
  width: 100vw;
  height: 100vh;
  display: grid;
  grid-template-rows: auto 1fr;
  background: #000;
}

.top {
  display: flex;
  align-items: center;
  padding: 6px;
}

.back {
  border: none;
  color: #fff;
  background: transparent;
  cursor: pointer;
  font-size: 14px;
}

.center {
  display: flex;
  justify-content: center;
  align-items: center;
}

.mic-button {
  width: 60px;
  height: 30px;
  border: none;
  background: transparent;
  cursor: pointer;
  padding: 0;
  margin: 0;
  transition: opacity 0.2s ease;
  display: flex;
  justify-content: center;
  align-items: center;
}

.mic-button:hover { opacity: 0.8; }
.mic-button:active { transform: scale(0.95); }
.mic-button.processing { cursor: not-allowed; }

.loading-spinner {
  width: 20px;
  height: 20px;
  border: 4px solid #333;
  border-top: 4px solid #fff;
  border-radius: 50%;
  animation: spin 1s linear infinite;
}

@keyframes spin { 0% { transform: rotate(0deg);} 100% { transform: rotate(360deg);} }

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
  background: #fff;
  min-height: 2px;
  border-radius: 1px;
  transition: height 0.1s ease;
}

.mic-button img {
  width: 30px;
  height: 30px;
  pointer-events: none;
}
</style>

