<script module lang="ts">
  import { defineMeta } from "@storybook/addon-svelte-csf";
  import BranchBadge from "./BranchBadge.svelte";
  import type { BranchListItemDto, BranchPresence } from "$lib/types";

  function branch(
    presence: BranchPresence,
    sync: { ahead: number | null; behind: number | null },
    name = "main",
  ): BranchListItemDto {
    return { name, presence, ...sync };
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

<Story name="CheckoutNone" args={{ branch: branch("checkout", none) }} />
<Story name="CheckoutAhead" args={{ branch: branch("checkout", ahead) }} />
<Story name="CheckoutBehind" args={{ branch: branch("checkout", behind) }} />
<Story name="CheckoutBoth" args={{ branch: branch("checkout", both) }} />
<Story name="LocalOnlyNone" args={{ branch: branch("local_only", none) }} />
<Story name="LocalOnlyAhead" args={{ branch: branch("local_only", ahead) }} />
<Story name="LocalOnlyBehind" args={{ branch: branch("local_only", behind) }} />
<Story name="LocalOnlyBoth" args={{ branch: branch("local_only", both) }} />
<Story name="RemoteOnlyNone" args={{ branch: branch("remote_only", none) }} />
<Story name="RemoteOnlyAhead" args={{ branch: branch("remote_only", ahead) }} />
<Story
  name="RemoteOnlyBehind"
  args={{ branch: branch("remote_only", behind) }}
/>
<Story name="RemoteOnlyBoth" args={{ branch: branch("remote_only", both) }} />
<Story name="LocalRemoteNone" args={{ branch: branch("local_remote", none) }} />
<Story
  name="LocalRemoteAhead"
  args={{ branch: branch("local_remote", ahead) }}
/>
<Story
  name="LocalRemoteBehind"
  args={{ branch: branch("local_remote", behind) }}
/>
<Story name="LocalRemoteBoth" args={{ branch: branch("local_remote", both) }} />
<Story
  name="LongNameTruncation"
  args={{
    branch: branch(
      "local_remote",
      both,
      "feature/very-long-branch-name-that-should-truncate",
    ),
  }}
/>
