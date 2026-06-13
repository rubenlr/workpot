<script lang="ts">
  const colorTokens = [
    { name: "primary", class: "bg-primary text-primary-foreground" },
    {
      name: "inverse-surface",
      class: "bg-inverse-surface text-inverse-on-surface",
    },
    {
      name: "inverse-on-surface-variant",
      class: "bg-inverse-surface text-inverse-on-surface-variant",
    },
    {
      name: "dirty-amber",
      class: "bg-dirty-amber text-dirty-amber-foreground",
    },
    {
      name: "clean-emerald",
      class: "bg-clean-emerald text-clean-emerald-foreground",
    },
    {
      name: "git-error-neutral",
      class: "bg-git-error-neutral text-git-error-neutral-foreground",
    },
    {
      name: "tag-blue",
      class:
        "bg-tag-blue-bg/15 text-tag-blue-text ring-1 ring-tag-blue-text/30",
    },
    { name: "error", class: "bg-error text-error-foreground" },
    { name: "card-surface", class: "bg-card-surface text-inverse-on-surface" },
    {
      name: "hover-overlay",
      class: "bg-hover-overlay text-inverse-on-surface",
    },
    {
      name: "input-surface",
      class: "bg-input-surface text-inverse-on-surface",
    },
    {
      name: "section-header",
      class: "bg-section-header text-inverse-on-surface",
    },
  ] as const;

  const shadowSamples = [
    { name: "shadow-dot-dirty", class: "shadow-dot-dirty" },
    { name: "shadow-dot-clean", class: "shadow-dot-clean" },
    { name: "shadow-dot-error", class: "shadow-dot-error" },
    { name: "shadow-row-selected", class: "shadow-row-selected" },
  ] as const;
</script>

<div class="space-y-8 bg-inverse-surface p-6 text-inverse-on-surface">
  <section>
    <h2 class="mb-3 text-lg font-semibold">Semantic colors</h2>
    <div class="grid grid-cols-2 gap-3 sm:grid-cols-3">
      {#each colorTokens as token (token.name)}
        <div class="overflow-hidden rounded-lg border border-card-border">
          <div class="px-3 py-4 text-sm font-medium {token.class}">
            {token.name}
          </div>
        </div>
      {/each}
    </div>
  </section>

  <section>
    <h2 class="mb-3 text-lg font-semibold">Status dot shadows</h2>
    <div class="flex flex-wrap gap-6">
      {#each shadowSamples as sample (sample.name)}
        <div class="flex flex-col items-center gap-2">
          <div
            class="size-4 rounded-full bg-inverse-on-surface-variant {sample.class}"
          ></div>
          <span class="text-xs text-inverse-on-surface-variant"
            >{sample.name}</span
          >
        </div>
      {/each}
    </div>
  </section>

  <section>
    <h2 class="mb-3 text-lg font-semibold">Typography</h2>
    <p class="font-sans text-base">font-sans — system UI stack</p>
    <p class="font-sans text-sm text-inverse-on-surface-variant">
      inverse-on-surface-variant body text
    </p>
  </section>

  <section>
    <h2 class="mb-3 text-lg font-semibold">Theming</h2>
    <p class="text-sm text-inverse-on-surface-variant">
      Use the Storybook background toolbar (dark / light) to preview
      <code class="text-inverse-on-surface">data-theme</code> overrides from stitch-tokens.css.
    </p>
  </section>
</div>
