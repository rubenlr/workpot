<script module lang="ts">
  import { defineMeta } from "@storybook/addon-svelte-csf";
  import DetailSectionHeading from "./DetailSectionHeading.svelte";

  const { Story } = defineMeta({
    title: "Primitives/DetailSectionHeading",
    component: DetailSectionHeading,
    tags: ["autodocs"],
    parameters: { layout: "centered" },
  });
</script>

<Story name="Default">
  <div class="bg-inverse-surface p-4">
    <DetailSectionHeading>Branches</DetailSectionHeading>
  </div>
</Story>
