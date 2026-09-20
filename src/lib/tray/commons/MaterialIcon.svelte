<script lang="ts">
  import { materialIconFallback } from "./materialIconFallbacks";
  import {
    materialSymbolsFontAvailable,
    probeMaterialSymbolsFont,
  } from "./materialSymbolsFont.svelte";

  let {
    name,
    size = 20,
    filled = false,
    class: className = "",
  }: {
    name: string;
    size?: number;
    filled?: boolean;
    class?: string;
  } = $props();

  probeMaterialSymbolsFont();

  const fontReady = $derived(materialSymbolsFontAvailable());
  const glyph = $derived(fontReady ? name : materialIconFallback(name, filled));
</script>

<span
  class="inline-flex shrink-0 select-none leading-none {fontReady
    ? 'material-symbols-outlined'
    : 'material-icon-fallback'} {className}"
  style:font-size="{size}px"
  style:font-variation-settings={fontReady
    ? `'FILL' ${filled ? 1 : 0}, 'wght' 400, 'GRAD' 0, 'opsz' 24`
    : undefined}
  aria-hidden="true">{glyph}</span
>

<style>
  .material-symbols-outlined {
    font-family: "Material Symbols Outlined";
    font-weight: normal;
    font-style: normal;
    letter-spacing: normal;
    text-transform: none;
    white-space: nowrap;
    overflow-wrap: normal;
    direction: ltr;
    -webkit-font-smoothing: antialiased;
  }

  .material-icon-fallback {
    font-family:
      system-ui,
      -apple-system,
      "Segoe UI",
      sans-serif;
    font-weight: 500;
    font-style: normal;
    line-height: 1;
  }
</style>
