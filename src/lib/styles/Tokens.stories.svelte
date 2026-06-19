<script module lang="ts">
  import { defineMeta } from "@storybook/addon-svelte-csf";
  import TokensSwatches from "./TokensSwatches.svelte";

  const { Story } = defineMeta({
    title: "Foundations/Tokens",
    component: TokensSwatches,
    tags: ["autodocs"],
    parameters: {
      layout: "fullscreen",
      docs: {
        description: {
          component:
            "Semantic design tokens from tokens.css — colors, status shadows, typography, and theme switching via data-theme.",
        },
      },
    },
  });
</script>

<Story name="Dark" parameters={{ backgrounds: { default: "dark" } }} />
<Story name="Light" parameters={{ backgrounds: { default: "light" } }} />
