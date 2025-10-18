<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { getCurrentWindow, LogicalSize } from '@tauri-apps/api/window';
  import IconButton from '$lib/components/IconButton.svelte';

  let WIDTH = 75;
  let HEIGHT = 35;

  onMount(async () => {
    const win = getCurrentWindow();
    await win.setMaxSize(new LogicalSize(WIDTH, HEIGHT));
    await win.setMinSize(new LogicalSize(WIDTH, HEIGHT));
  });
</script>

<main class="container-fullscreen container-centered">
  <div class="button-row">
    <IconButton onclick={() => goto('/record')} ariaLabel="Record" src="/favicon.svg" alt="Microphone" />
    <IconButton onclick={() => goto('/settings')} ariaLabel="Settings" symbol="⚙" />
  </div>
</main>

<style>
  .button-row {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
  }
</style>
