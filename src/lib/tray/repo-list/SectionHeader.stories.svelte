<script module lang="ts">
  import { defineMeta } from "@storybook/addon-svelte-csf";
  import SectionHeader from "./SectionHeader.svelte";

  const { Story } = defineMeta({
    title: "Components/SectionHeader",
    component: SectionHeader,
  });
</script>

<Story name="Pinned" args={{ label: "Pinned" }} />
<Story name="Dirty" args={{ label: "Dirty" }} />
<Story name="Recent" args={{ label: "Recent" }} />
<Story name="Rest" args={{ label: "Rest" }} />
