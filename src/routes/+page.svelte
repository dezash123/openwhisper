<script lang="ts">
  import { writeText } from '@tauri-apps/plugin-clipboard-manager';
  import { invoke } from '@tauri-apps/api/core';
  let isRecording = $state(false);
  let isProcessing = $state(false);

  async function toggleMic() {
    if (isProcessing) return;
    
    try {
      if (!isRecording) {
        // Start recording
        await invoke('start_recording');
        isRecording = true;
        console.log('Recording started');
      } else {
        // Stop recording and transcribe
        isProcessing = true;
        console.log('Stopping recording and transcribing...');
        
        const transcription = await invoke('stop_recording_and_transcribe') as string;
        
        if (transcription && transcription.trim()) {
          await writeText(transcription);
          console.log('Transcription copied to clipboard:', transcription);
        } else {
          console.log('No transcription received');
        }
        
        isRecording = false;
        isProcessing = false;
      }
    } catch (error) {
      console.error('Error:', error);
      isRecording = false;
      isProcessing = false;
    }
  }
</script>

<main class="container">
  <button class="mic-button" onclick={toggleMic} class:recording={isRecording} class:processing={isProcessing}>
    {#if isProcessing}
      <div class="loading-spinner"></div>
    {:else}
      <img src={isRecording ? "/stop.svg" : "/favicon.svg"} alt={isRecording ? "Stop" : "Microphone"} />
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
  height: 60px;
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
  width: 40px;
  height: 40px;
  border: 4px solid #333;
  border-top: 4px solid #fff;
  border-radius: 50%;
  animation: spin 1s linear infinite;
}

@keyframes spin {
  0% { transform: rotate(0deg); }
  100% { transform: rotate(360deg); }
}

.mic-button img {
  width: 56px;
  height: 56px;
  pointer-events: none;
  transition: filter 0.3s ease;
}

</style>
