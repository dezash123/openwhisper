<script lang="ts">
  import { writeText } from '@tauri-apps/plugin-clipboard-manager';
  import { invoke, Channel } from '@tauri-apps/api/core';
  import { emit } from '@tauri-apps/api/event';
  import { warn, debug, info, error } from '@tauri-apps/plugin-log';
  
  let isRecording = $state(false);
  let isProcessing = $state(false);
  let audioLevels = $state([]);
  let recordingPromise: Promise<string> | null = null;

  async function toggleMic() {
    if (isProcessing) return;
    
    try {
      if (!isRecording) {
        isRecording = true;
        info('Starting recording...');
        
        // Create channel for audio levels
        const audioLevelChan = new Channel<{levels: number[]}>();
        audioLevelChan.onmessage = (message) => {
          audioLevels = message.levels;
          debug(`Audio levels: ${message.levels}`);
        };
        
        // Start the recording and transcription process with channel
        recordingPromise = invoke('record_and_transcribe', {
          audioLevelChan
        }) as Promise<string>;
      } else {
        isProcessing = true;
        
        info('Stopping recording...');
        
        await emit('stop-recording');
        
        // Wait for transcription result
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
</script>

<main class="container">
  <button class="mic-button" onclick={toggleMic} class:recording={isRecording} class:processing={isProcessing}>
    {#if isProcessing}
      <div class="loading-spinner"></div>
    {:else if isRecording}
      <div class="frequency-bars">
        {#each audioLevels as level, i}
          <div class="bar" style={`height: ${Math.min(level * 50, 25)}px`}></div>
        {/each}
      </div>
    {:else}
      <img src="/favicon.svg" alt="Microphone" />
    {/if}
  </button>
</main>

<style>
:global(html) {
  margin: 0;
  padding: 0;
  overflow: hidden;
  background: #000;
}

:global(body) {
  margin: 0;
  padding: 0;
  background: #000;
  overflow: hidden;
}

* {
  box-sizing: border-box;
  margin: 0;
  padding: 0;
}

.container {
  width: 100vw;
  height: 100vh;
  margin: 0;
  padding: 0;
  display: flex;
  justify-content: center;
  align-items: center;
  background: #000;
  position: fixed;
  top: 0;
  left: 0;
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

.mic-button:hover {
  opacity: 0.8;
}

.mic-button:active {
  transform: scale(0.95);
}

.mic-button.recording {
  opacity: 1;
}

.mic-button.processing {
  cursor: not-allowed;
}

.loading-spinner {
  width: 20px;
  height: 20px;
  border: 4px solid #333;
  border-top: 4px solid #fff;
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
  background: #fff;
  min-height: 2px;
  border-radius: 1px;
  transition: height 0.1s ease;
}

.mic-button img {
  width: 30px;
  height: 30px;
  pointer-events: none;
  transition: filter 0.3s ease;
}

</style>
