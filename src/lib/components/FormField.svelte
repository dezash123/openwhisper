<script lang="ts">
  
  interface Props {
    label: string;
    id: string;
    type?: 'text' | 'select' | 'file-picker';
    value?: string;
    options?: Array<{value: string, label: string}>;
    placeholder?: string;
    onInput?: (value: string) => void;
    onBrowse?: () => void;
    browseText?: string;
  }
  
  let { 
    label, 
    id, 
    type = 'text', 
    value = $bindable(''), 
    options = [], 
    placeholder, 
    onInput,
    onBrowse,
    browseText = 'Browse…'
  }: Props = $props();
  
  function handleInput(event: Event) {
    const target = event.target as HTMLInputElement | HTMLSelectElement;
    value = target.value;
    onInput?.(target.value);
  }
</script>

<div class="form-group">
  <label class="form-label" for={id}>{label}</label>
  
  {#if type === 'select'}
    <select {id} class="form-select" bind:value oninput={handleInput}>
      {#each options as option}
        <option value={option.value}>{option.label}</option>
      {/each}
    </select>
  {:else if type === 'file-picker'}
    <div class="flex-row">
      <input 
        {id} 
        class="form-input" 
        type="text" 
        spellcheck={false}
        {placeholder}
        bind:value 
        oninput={handleInput}
      />
      <button class="form-button" type="button" onclick={onBrowse}>
        {browseText}
      </button>
    </div>
  {:else}
    <input 
      {id} 
      class="form-input" 
      {type}
      spellcheck={false}
      {placeholder}
      bind:value 
      oninput={handleInput}
    />
  {/if}
</div>