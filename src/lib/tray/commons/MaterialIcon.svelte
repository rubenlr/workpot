<script lang="ts">
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
</script>

<span
  class="material-symbols-outlined inline-flex shrink-0 select-none leading-none {className}"
  style="font-size: {size}px; font-variation-settings: 'FILL' {filled
    ? 1
    : 0}, 'wght' 400, 'GRAD' 0, 'opsz' 24;"
  aria-hidden="true">{name}</span
>

<style>
  .material-symbols-outlined {
    font-family: "Material Symbols Outlined";
    font-weight: normal;
    font-style: normal;
    letter-spacing: normal;
    text-transform: none;
    white-space: nowrap;
    word-wrap: normal;
    direction: ltr;
    -webkit-font-smoothing: antialiased;
  }
</style>
