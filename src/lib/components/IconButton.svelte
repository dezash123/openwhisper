<script lang="ts">
  
  interface Props {
    onclick?: () => void;
    ariaLabel: string;
    src?: string;
    alt?: string;
    symbol?: string;
  }
  
  let { onclick, ariaLabel, src, alt, symbol }: Props = $props();
</script>

<button class="btn btn-icon" aria-label={ariaLabel} {onclick}>
  {#if src}
    <img {src} {alt} />
  {:else if symbol}
    <span class="symbol" aria-hidden="true">{symbol}</span>
  {/if}
</button>

<style>
  .symbol {
    font-size: 18px;
    line-height: 1;
  }
</style>