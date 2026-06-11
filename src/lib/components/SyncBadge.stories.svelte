<script module lang="ts">
  import { defineMeta } from "@storybook/addon-svelte-csf";
  import SyncBadge from "./SyncBadge.svelte";

  const { Story } = defineMeta({
    title: "Components/SyncBadge",
    component: SyncBadge,
  });
</script>

<Story name="AheadOnly" args={{ ahead: 2, behind: 0 }} />
<Story name="BehindOnly" args={{ ahead: 0, behind: 3 }} />
<Story name="Both" args={{ ahead: 2, behind: 1 }} />
<Story name="None" args={{ ahead: null, behind: null }} />
