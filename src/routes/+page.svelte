<script lang="ts">
  import { writeText } from '@tauri-apps/plugin-clipboard-manager';
  import { invoke } from '@tauri-apps/api/core';
  let isRecording = $state(false);

  async function toggleMic() {
    isRecording = !isRecording;
    console.log('Mic toggled:', isRecording);
    if (!isRecording) {
      const text = await invoke('get_text', { text: 'Hello, World!' });
      await writeText(text);
      console.log('Text copied to clipboard:', text);
    }
  }
</script>

<main class="container">
  <button class="mic-button" onclick={toggleMic} class:recording={isRecording}>
    <img src={isRecording ? "/stop.svg" : "/favicon.svg"} alt={isRecording ? "Stop" : "Microphone"} />
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

.mic-button img {
  width: 56px;
  height: 56px;
  pointer-events: none;
  transition: filter 0.3s ease;
}

</style>
