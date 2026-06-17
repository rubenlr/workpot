<script module lang="ts">
  import { defineMeta } from "@storybook/addon-svelte-csf";
  import MaterialIcon from "./MaterialIcon.svelte";

  const { Story } = defineMeta({
    title: "Primitives/MaterialIcon",
    component: MaterialIcon,
    tags: ["autodocs"],
    args: {
      name: "folder",
    },
  });
</script>

<Story name="Outlined" args={{ name: "folder", filled: false }} />
<Story name="Filled" args={{ name: "folder", filled: true }} />
<Story name="Size16" args={{ name: "search", size: 16 }} />
<Story name="Size20" args={{ name: "search", size: 20 }} />
<Story name="Size24" args={{ name: "search", size: 24 }} />
<Story
  name="CustomClass"
  args={{ name: "star", class: "text-primary-accent" }}
/>
