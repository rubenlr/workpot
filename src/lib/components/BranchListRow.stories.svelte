<script module lang="ts">
  import { defineMeta } from "@storybook/addon-svelte-csf";
  import BranchListRow from "./BranchListRow.svelte";
  import { storyBranches } from "./repoStoryFixtures";

  const branches = storyBranches();

  const { Story } = defineMeta({
    title: "Components/BranchListRow",
    component: BranchListRow,
  });
</script>

<Story name="Current" args={{ branch: branches[0] }} />
<Story name="Remote" args={{ branch: branches[1] }} />
<Story name="FeatureWithSync" args={{ branch: branches[2] }} />
