<script module lang="ts">
  import { defineMeta } from "@storybook/addon-svelte-csf";
  import SyncBadge from "./SyncBadge.svelte";

  const { Story } = defineMeta({
    title: "Components/SyncBadge",
    component: SyncBadge,
  });

  const noop = () => {};
</script>

<Story name="AheadOnly" args={{ ahead: 2, behind: 0 }} />
<Story name="BehindOnly" args={{ ahead: 0, behind: 3 }} />
<Story name="Both" args={{ ahead: 2, behind: 1 }} />
<Story name="BothDisplayOnly" args={{ ahead: 2, behind: 1, branch: "main" }} />
<Story name="None" args={{ ahead: null, behind: null }} />
<Story
  name="PushAvailable"
  args={{ ahead: 2, behind: 0, branch: "main", onPush: noop }}
/>
<Story
  name="PullAvailable"
  args={{ ahead: 0, behind: 3, branch: "main", onPull: noop }}
/>
<Story
  name="BothInteractive"
  args={{ ahead: 2, behind: 1, branch: "main", onPush: noop, onPull: noop }}
/>
<Story
  name="Pushing"
  args={{
    ahead: 2,
    behind: 0,
    branch: "main",
    syncingDirection: "push",
    onPush: noop,
  }}
/>
<Story
  name="Pulling"
  args={{
    ahead: 0,
    behind: 3,
    branch: "main",
    syncingDirection: "pull",
    onPull: noop,
  }}
/>
