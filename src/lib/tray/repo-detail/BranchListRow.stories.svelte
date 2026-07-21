<script module lang="ts">
  import { defineMeta } from "@storybook/addon-svelte-csf";
  import BranchListRow from "./BranchListRow.svelte";
  import {
    storyBranches,
    storyCheckoutLocalOnlyBranch,
    storyRepoBase,
  } from "$lib/tray/repo-list/repoStoryFixtures";

  const noop = () => {};
  const branches = storyBranches();

  const { Story } = defineMeta({
    title: "Components/BranchListRow",
    component: BranchListRow,
    tags: ["autodocs"],
    args: {
      repoPath: storyRepoBase.path,
      onToggleHidden: noop,
    },
  });
</script>

<Story name="CheckedOut" args={{ branch: branches[0] }} />
<Story
  name="CheckedOutLocalOnly"
  args={{ branch: storyCheckoutLocalOnlyBranch() }}
/>
<Story name="Remote" args={{ branch: branches[2] }} />
<Story
  name="FeatureWithSync"
  args={{ branch: { ...branches[3], hidden: false } }}
/>
<Story name="Hidden" args={{ branch: branches[3] }} />
