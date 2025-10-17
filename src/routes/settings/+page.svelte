<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { getCurrentWindow, LogicalSize } from '@tauri-apps/api/window';
  import { error } from '@tauri-apps/plugin-log';
  import { open } from '@tauri-apps/plugin-dialog';

  let audioQuality = $state<'low' | 'medium' | 'high'>('high');
  let modelName = $state('base.en');
  let saveDir = $state('');
  let weightsPath = $state('');

  onMount(async () => {
    const win = getCurrentWindow();
    try {
      await win.setMaxSize(null);
      await win.setMinSize(new LogicalSize(360, 320));
      await win.setSize(new LogicalSize(520, 400));
      await win.center();
    } catch (e) {
      try { error(`Failed to resize window: ${e}`); } catch {}
    }
  });

  function goBack() {
    goto('/');
  }

  async function pickSaveDir() {
    try {
      const selected = await open({ directory: true, multiple: false, title: 'Choose recordings folder' });
      if (typeof selected === 'string') {
        saveDir = selected;
      }
    } catch (e) {
      try { error(`Dialog error: ${e}`); } catch {}
    }
  }

  async function pickWeightsPath() {
    try {
      const selected = await open({ directory: false, multiple: false, title: 'Select model weights file' });
      if (typeof selected === 'string') {
        weightsPath = selected;
      }
    } catch (e) {
      try { error(`Dialog error: ${e}`); } catch {}
    }
  }
</script>

<main class="wrap">
  <header class="header">
    <button class="back" aria-label="Back" onclick={goBack}>⟵</button>
    <h1>Settings</h1>
  </header>

  <section class="content">
    <div class="group">
      <label class="label" for="audioQuality">Audio quality</label>
      <select id="audioQuality" bind:value={audioQuality} class="select">
        <option value="low">Low</option>
        <option value="medium">Medium</option>
        <option value="high">High</option>
      </select>
    </div>

    <div class="group">
      <label class="label" for="model">Transcription model</label>
      <select id="model" bind:value={modelName} class="select">
        <option value="tiny">tiny</option>
        <option value="tiny.en">tiny.en</option>
        <option value="base">base</option>
        <option value="base.en">base.en</option>
        <option value="small">small</option>
        <option value="small.en">small.en</option>
        <option value="medium">medium</option>
        <option value="medium.en">medium.en</option>
        <option value="large-v2">large-v2</option>
      </select>
    </div>

    <div class="group">
      <label class="label" for="saveDir">Save file location</label>
      <div class="path-row">
        <input id="saveDir" class="input" type="text" spellcheck={false} placeholder="/path/to/recordings" bind:value={saveDir} />
        <button class="browse" type="button" onclick={pickSaveDir}>Browse…</button>
      </div>
    </div>

    <div class="group">
      <label class="label" for="weightsPath">Model weights location</label>
      <div class="path-row">
        <input id="weightsPath" class="input" type="text" spellcheck={false} placeholder="/path/to/ggml-model.bin" bind:value={weightsPath} />
        <button class="browse" type="button" onclick={pickWeightsPath}>Browse…</button>
      </div>
    </div>

    <div class="note">These settings are not yet persisted.</div>
  </section>
</main>

<style>
:global(html), :global(body) { margin: 0; padding: 0; background: #000; color: #fff; }
.wrap { width: 100vw; height: 100vh; display: flex; flex-direction: column; }
.header { display: flex; align-items: center; gap: 8px; padding: 8px; }
.header h1 { font-size: 14px; font-weight: 600; margin: 0; }
.back { border: none; background: transparent; color: #fff; cursor: pointer; font-size: 14px; }
.content { padding: 12px; display: flex; flex-direction: column; gap: 12px; }
.group { display: flex; flex-direction: column; gap: 6px; }
.label { font-size: 12px; opacity: 0.9; }
.select, .input { background: #111; color: #fff; border: 1px solid #333; border-radius: 4px; padding: 6px 8px; font-size: 12px; }
.path-row { display: flex; gap: 8px; }
.browse { background: #222; color: #fff; border: 1px solid #333; border-radius: 4px; padding: 6px 8px; font-size: 12px; cursor: pointer; }
.browse:hover { background: #2a2a2a; }
.note { opacity: 0.7; font-style: italic; font-size: 12px; }
</style>
