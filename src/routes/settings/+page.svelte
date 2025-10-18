<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import { invoke } from '@tauri-apps/api/core';
  import { goto } from '$app/navigation';
  import { getCurrentWindow, LogicalSize } from '@tauri-apps/api/window';
  import IconButton from '$lib/components/IconButton.svelte';
  import FormField from '$lib/components/FormField.svelte';

  let audioQuality = $state<'low'|'medium'|'high'>('high');
  let modelName = $state('base.en');
  let saveDir = $state('');
  let weightsPath = $state('');

  function save() { 
    invoke('set_config', { 
      config: { 
        recording_dir: saveDir, 
        model_name: modelName, 
        audio_quality: audioQuality, 
        model_weights_path: weightsPath 
      } 
    }); 
  }
  
  async function pickSaveDir() { 
    const s = await open({ directory:true, multiple:false, title:'Choose recordings folder' }); 
    if (typeof s==='string') { 
      saveDir = s; 
      save(); 
    } 
  }
  
  async function pickWeightsPath() { 
    const s = await open({ directory:false, multiple:false, title:'Select model weights file' }); 
    if (typeof s==='string') { 
      weightsPath = s; 
      save(); 
    } 
  }

  const win = getCurrentWindow();
  win.setMaxSize(null);
  win.setMinSize(new LogicalSize(560, 420));
  win.setSize(new LogicalSize(760, 520));
  win.center();
  
  invoke<any>('get_config').then(c => { 
    modelName = c.model_name; 
    saveDir = c.recording_dir; 
    audioQuality = c.audio_quality; 
    weightsPath = c.model_weights_path; 
  });
  
  const audioQualityOptions = [
    { value: 'low', label: 'Low' },
    { value: 'medium', label: 'Medium' },
    { value: 'high', label: 'High' }
  ];
  
  const modelOptions = [
    { value: 'tiny', label: 'tiny' },
    { value: 'tiny.en', label: 'tiny.en' },
    { value: 'base', label: 'base' },
    { value: 'base.en', label: 'base.en' },
    { value: 'small', label: 'small' },
    { value: 'small.en', label: 'small.en' },
    { value: 'medium', label: 'medium' },
    { value: 'medium.en', label: 'medium.en' },
    { value: 'large-v2', label: 'large-v2' }
  ];
</script>

<main class="wrap">
  <header class="header">
    <IconButton onclick={() => goto('/')} ariaLabel="Back" symbol="⟵" />
    <h1>Settings</h1>
  </header>

  <section class="content">
    <FormField 
      label="Audio quality" 
      id="audioQuality" 
      type="select" 
      options={audioQualityOptions}
      bind:value={audioQuality} 
      onInput={save} 
    />
    
    <FormField 
      label="Transcription model" 
      id="model" 
      type="select" 
      options={modelOptions}
      bind:value={modelName} 
      onInput={save} 
    />
    
    <FormField 
      label="Save file location" 
      id="saveDir" 
      type="file-picker" 
      placeholder="/path/to/recordings"
      bind:value={saveDir} 
      onInput={save}
      onBrowse={pickSaveDir} 
    />
    
    <FormField 
      label="Model weights location" 
      id="weightsPath" 
      type="file-picker" 
      placeholder="/path/to/ggml-model.bin"
      bind:value={weightsPath} 
      onInput={save}
      onBrowse={pickWeightsPath} 
    />
  </section>
</main>

<style>
  .wrap {
    width: 100vw;
    height: 100vh;
    display: flex;
    flex-direction: column;
  }

  .header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px;
  }

  .header h1 {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
  }

  .content {
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
</style>

