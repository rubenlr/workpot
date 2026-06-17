<script module lang="ts">
  import { defineMeta } from "@storybook/addon-svelte-csf";
  import BranchBadge from "./BranchBadge.svelte";
  import type { BranchListItemDto, BranchTracking } from "$lib/types";

  function branch(
    checkedOut: boolean,
    tracking: BranchTracking,
    sync: { ahead: number | null; behind: number | null },
    name = "main",
  ): BranchListItemDto {
    return { name, checked_out: checkedOut, tracking, ...sync };
  }

  const none = { ahead: null, behind: null };
  const ahead = { ahead: 2, behind: 0 };
  const behind = { ahead: 0, behind: 3 };
  const both = { ahead: 2, behind: 1 };

  const { Story } = defineMeta({
    title: "Primitives/BranchBadge",
    component: BranchBadge,
    tags: ["autodocs"],
  });
</script>

<Story
  name="CheckedOutNone"
  args={{ branch: branch(true, "local_remote", none) }}
/>
<Story
  name="CheckedOutAhead"
  args={{ branch: branch(true, "local_remote", ahead) }}
/>
<Story
  name="CheckedOutBehind"
  args={{ branch: branch(true, "local_remote", behind) }}
/>
<Story
  name="CheckedOutBoth"
  args={{ branch: branch(true, "local_remote", both) }}
/>
<Story
  name="LocalOnlyNone"
  args={{ branch: branch(false, "local_only", none) }}
/>
<Story
  name="LocalOnlyAhead"
  args={{ branch: branch(false, "local_only", ahead) }}
/>
<Story
  name="LocalOnlyBehind"
  args={{ branch: branch(false, "local_only", behind) }}
/>
<Story
  name="LocalOnlyBoth"
  args={{ branch: branch(false, "local_only", both) }}
/>
<Story
  name="RemoteOnlyNone"
  args={{ branch: branch(false, "remote_only", none) }}
/>
<Story
  name="RemoteOnlyAhead"
  args={{ branch: branch(false, "remote_only", ahead) }}
/>
<Story
  name="RemoteOnlyBehind"
  args={{ branch: branch(false, "remote_only", behind) }}
/>
<Story
  name="RemoteOnlyBoth"
  args={{ branch: branch(false, "remote_only", both) }}
/>
<Story
  name="LocalRemoteNone"
  args={{ branch: branch(false, "local_remote", none) }}
/>
<Story
  name="LocalRemoteAhead"
  args={{ branch: branch(false, "local_remote", ahead) }}
/>
<Story
  name="LocalRemoteBehind"
  args={{ branch: branch(false, "local_remote", behind) }}
/>
<Story
  name="LocalRemoteBoth"
  args={{ branch: branch(false, "local_remote", both) }}
/>
<Story
  name="LongNameTruncation"
  args={{
    branch: branch(
      false,
      "local_remote",
      both,
      "feature/very-long-branch-name-that-should-truncate",
    ),
  }}
/>
