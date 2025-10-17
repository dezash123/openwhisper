<script lang="ts">
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { getCurrentWindow, PhysicalPosition, LogicalSize } from '@tauri-apps/api/window';
  import { info } from '@tauri-apps/plugin-log';
  function openRecorder() {
    goto('/record');
  }

  onMount(async () => {
    const win = getCurrentWindow();
    await win.setMaxSize(new LogicalSize(60, 30));
    await win.setMinSize(new LogicalSize(60, 30));
    await win.setSize(new LogicalSize(60, 30));
    // await win.center();
    await win.setPosition(new PhysicalPosition(0, 0));
  });

  function openSettings() {
    goto('/settings');
  }
</script>

<main class="container">
  <div class="button-row">
    <button class="icon-button" aria-label="Record" onclick={openRecorder}>
      <img src="/favicon.svg" alt="Microphone" />
    </button>
    <button class="icon-button gear" aria-label="Settings" onclick={openSettings}>
      <span class="gear-symbol" aria-hidden="true">⚙</span>
    </button>
  </div>
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

.button-row {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
}

.icon-button {
  width: 24px;
  height: 24px;
  border: none;
  background: transparent;
  cursor: pointer;
  padding: 0;
  margin: 0;
  transition: opacity 0.2s ease, transform 0.05s ease;
  display: flex;
  justify-content: center;
  align-items: center;
}

.icon-button:hover { opacity: 0.8; }
.icon-button:active { transform: scale(0.95); }

.icon-button img {
  width: 24px;
  height: 24px;
  pointer-events: none;
  transition: filter 0.3s ease;
}

.gear .gear-symbol {
  color: #fff;
  font-size: 18px;
  line-height: 1;
}

</style>
